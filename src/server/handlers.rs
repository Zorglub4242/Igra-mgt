/// API Request Handlers
/// Reuses core business logic from existing modules
use axum::{
    extract::{Path, Query},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::core::{
    log_parser, storage, updater, wallet::WalletManager, ConfigManager, DockerManager,
};

// ============================================================================
// Response Types
// ============================================================================

#[derive(Serialize)]
pub struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

impl<T> ApiResponse<T> {
    fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }
}

#[derive(Serialize)]
pub struct PortMapping {
    host_port: String,
    container_port: String,
    protocol: String,
}

#[derive(Serialize)]
pub struct ServiceInfo {
    name: String,
    status: String,
    health: Option<String>,
    cpu_percent: f64,
    memory_mb: f64,
    network_rx_mb: f64,
    network_tx_mb: f64,
    uptime: Option<String>,
    // Image and ports
    image: String,
    ports: Vec<PortMapping>,
    // Storage
    container_size_mb: f64,
    volume_size_mb: f64,
    // Parsed metrics from logs
    status_text: Option<String>,
    primary_metric: Option<String>,
    secondary_metric: Option<String>,
    is_healthy_metric: bool,
    // Project identification
    project_name: Option<String>, // Docker Compose project name
    // Metrics availability
    has_metrics: bool, // Whether a plugin is available for this service
}

// WalletInfo is now imported from crate::core::wallet module

#[derive(Deserialize)]
pub struct ServicesQuery {
    #[serde(default)]
    show_all: bool, // Show all containers (not just IGRA)

    #[serde(default)]
    profiles: Option<String>, // Comma-separated list of profiles to filter by

    #[serde(default)]
    statuses: Option<String>, // Comma-separated list of statuses to filter by (healthy, running, stopped, unhealthy)

    #[serde(default)]
    project: Option<String>, // Filter by project name

    #[serde(default)]
    name: Option<String>, // Filter by container name (partial match, case-insensitive)
}

#[derive(Deserialize)]
pub struct LogsQuery {
    #[serde(default = "default_tail")]
    tail: usize,
    #[serde(default)]
    follow: bool,
}

fn default_tail() -> usize {
    100
}

#[derive(Serialize)]
pub struct ParsedLogLine {
    timestamp: String,
    level: String,
    module: String,
    message: String,
}

#[derive(Deserialize)]
pub struct ParsedLogsQuery {
    #[serde(default = "default_tail")]
    tail: usize,
    #[serde(default)]
    level: Option<String>, // Filter: ERROR, WARN, INFO, DEBUG, TRACE
    #[serde(default)]
    module: Option<String>, // Filter by module name
}

// ============================================================================
// Network Topology Types
// ============================================================================

#[derive(Serialize, Clone)]
pub struct NetworkTopology {
    pub nodes: Vec<NetworkNode>,
    pub edges: Vec<NetworkEdge>,
}

#[derive(Serialize, Clone)]
pub struct NetworkNode {
    pub id: String,
    pub label: String,
    pub node_type: String, // "container" | "service" | "network" | "domain" | "gateway" | "firewall_rule"
    pub status: String,    // "running" | "stopped" | "active" | "inactive"
    pub ports: Vec<String>,
    pub ip_address: Option<String>,
    pub layer: String, // "internet" | "firewall" | "gateway" | "docker" | "systemd" | "management"
    pub metadata: HashMap<String, String>,
    pub warnings: Vec<String>, // Security warnings or issues
    pub domains: Vec<String>,  // Domain names (for internet layer)
}

#[derive(Serialize, Clone)]
pub struct NetworkEdge {
    pub source: String,
    pub target: String,
    pub edge_type: String, // "port_mapping" | "network" | "dependency" | "http" | "websocket" | "ipc"
    pub label: Option<String>,
    pub protocol: Option<String>, // "http" | "ws" | "tcp" | "ipc"
    pub metadata: HashMap<String, String>,
}

// Internal types for connection parsing
#[derive(Clone, Debug)]
struct DetectedConnection {
    source: String,
    target: String,
    port: Option<u16>,
    protocol: String,
    connection_type: String,
    label: Option<String>,
}

// ============================================================================
// Service Management Handlers
// ============================================================================

pub async fn get_services(
    Query(params): Query<ServicesQuery>,
) -> Result<Json<ApiResponse<Vec<ServiceInfo>>>, StatusCode> {
    let docker = DockerManager::new()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let containers = docker
        .list_containers_filtered(params.show_all)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Process all containers in parallel for speed
    let tasks: Vec<_> = containers
        .into_iter()
        .map(|c| {
            tokio::spawn(async move {
                let docker = DockerManager::new().await.ok()?;

                // Get stats for resource metrics
                let stats = docker.get_container_stats(&c.name).await.ok().flatten();

                let (
                    cpu_percent,
                    memory_mb,
                    network_rx_mb,
                    network_tx_mb,
                    container_size_mb,
                    volume_size_mb,
                ) = if let Some(s) = stats {
                    (
                        s.cpu_percent,
                        s.memory_usage as f64 / 1024.0 / 1024.0,
                        s.network_rx as f64 / 1024.0 / 1024.0,
                        s.network_tx as f64 / 1024.0 / 1024.0,
                        s.container_size as f64 / 1024.0 / 1024.0,
                        s.volume_size as f64 / 1024.0 / 1024.0,
                    )
                } else {
                    (0.0, 0.0, 0.0, 0.0, 0.0, 0.0)
                };

                // Parse ports into structured format
                let ports: Vec<PortMapping> = c
                    .ports
                    .iter()
                    .filter_map(|p| {
                        if p.contains("->") {
                            if let Some((left, right)) = p.split_once("->") {
                                let host_port =
                                    left.rsplit_once(':').map(|(_, port)| port).unwrap_or("");
                                let container_port =
                                    right.split_once('/').map(|(port, _)| port).unwrap_or(right);

                                if !host_port.is_empty() && !host_port.starts_with(':') {
                                    return Some(PortMapping {
                                        host_port: host_port.to_string(),
                                        container_port: container_port.to_string(),
                                        protocol: "tcp".to_string(),
                                    });
                                }
                            }
                        }
                        None
                    })
                    .collect();

                // Fetch last 30 lines of logs and parse metrics (fast - only for key services)
                let (status_text, primary_metric, secondary_metric, is_healthy_metric) =
                    if c.status.contains("Up") {
                        let logs = docker.get_logs(&c.name, Some(30)).await.unwrap_or_default();
                        let metrics = log_parser::parse_service_logs(&c.name, &logs);
                        (
                            metrics.status_text,
                            metrics.primary_metric,
                            metrics.secondary_metric,
                            metrics.is_healthy,
                        )
                    } else {
                        (None, None, None, true)
                    };

                // Check if a metrics plugin is available for this service
                let has_metrics = docker
                    .metrics_registry
                    .find_plugin(&c.name, &c.image)
                    .is_some();

                Some(ServiceInfo {
                    name: c.name,
                    status: c.status,
                    health: c.health,
                    cpu_percent,
                    memory_mb,
                    network_rx_mb,
                    network_tx_mb,
                    uptime: None,
                    image: c.image,
                    ports,
                    container_size_mb,
                    volume_size_mb,
                    status_text,
                    primary_metric,
                    secondary_metric,
                    is_healthy_metric,
                    project_name: c.project_name,
                    has_metrics,
                })
            })
        })
        .collect();

    // Wait for all parallel tasks
    let mut services = Vec::new();
    for task in tasks {
        if let Ok(Some(service)) = task.await {
            services.push(service);
        }
    }

    // Apply filters
    if params.profiles.is_some()
        || params.statuses.is_some()
        || params.project.is_some()
        || params.name.is_some()
    {
        services = services
            .into_iter()
            .filter(|service| {
                // Filter by profiles (requires checking docker-compose labels)
                if let Some(ref profiles_str) = params.profiles {
                    let requested_profiles: Vec<&str> = profiles_str
                        .split(',')
                        .map(|s| s.trim())
                        .filter(|s| !s.is_empty())
                        .collect();

                    if !requested_profiles.is_empty() {
                        let is_in_requested_profile = requested_profiles.iter().any(|p| match *p {
                            "kaspad" => matches!(service.name.as_str(), "kaspad" | "kaspa-miner"),
                            "backend" => matches!(
                                service.name.as_str(),
                                "execution-layer" | "block-builder" | "viaduct"
                            ),
                            "frontend-w1" => matches!(
                                service.name.as_str(),
                                "traefik" | "rpc-provider-0" | "kaswallet-0"
                            ),
                            "frontend-w2" => matches!(
                                service.name.as_str(),
                                "traefik"
                                    | "rpc-provider-0"
                                    | "rpc-provider-1"
                                    | "kaswallet-0"
                                    | "kaswallet-1"
                            ),
                            "frontend-w3" => matches!(
                                service.name.as_str(),
                                "traefik"
                                    | "rpc-provider-0"
                                    | "rpc-provider-1"
                                    | "rpc-provider-2"
                                    | "kaswallet-0"
                                    | "kaswallet-1"
                                    | "kaswallet-2"
                            ),
                            "frontend-w4" => matches!(
                                service.name.as_str(),
                                "traefik"
                                    | "rpc-provider-0"
                                    | "rpc-provider-1"
                                    | "rpc-provider-2"
                                    | "rpc-provider-3"
                                    | "kaswallet-0"
                                    | "kaswallet-1"
                                    | "kaswallet-2"
                                    | "kaswallet-3"
                            ),
                            "frontend-w5" => matches!(
                                service.name.as_str(),
                                "traefik"
                                    | "rpc-provider-0"
                                    | "rpc-provider-1"
                                    | "rpc-provider-2"
                                    | "rpc-provider-3"
                                    | "rpc-provider-4"
                                    | "kaswallet-0"
                                    | "kaswallet-1"
                                    | "kaswallet-2"
                                    | "kaswallet-3"
                                    | "kaswallet-4"
                            ),
                            "kaswallets" => service.name.starts_with("kaswallet-"),
                            "rpc-providers" => service.name.starts_with("rpc-provider-"),
                            _ => false,
                        });

                        if !is_in_requested_profile {
                            return false;
                        }
                    }
                }

                // Filter by status
                if let Some(ref statuses_str) = params.statuses {
                    let requested_statuses: Vec<&str> = statuses_str.split(',').collect();
                    let mut status_match = false;

                    for status in &requested_statuses {
                        match *status {
                            "healthy" => {
                                if service.status.contains("Up")
                                    && service.status.contains("healthy")
                                {
                                    status_match = true;
                                }
                            }
                            "running" => {
                                if service.status.contains("Up")
                                    && !service.status.contains("healthy")
                                {
                                    status_match = true;
                                }
                            }
                            "stopped" => {
                                if service.status.contains("Exited") {
                                    status_match = true;
                                }
                            }
                            "unhealthy" => {
                                if service.status.contains("Up")
                                    && !service.status.contains("healthy")
                                    && !service.is_healthy_metric
                                {
                                    status_match = true;
                                }
                            }
                            _ => {}
                        }
                    }

                    if !status_match {
                        return false;
                    }
                }

                // Filter by project name
                if let Some(ref project) = params.project {
                    if service.project_name.as_ref() != Some(project) {
                        return false;
                    }
                }

                // Filter by container name (partial match, case-insensitive)
                if let Some(ref name) = params.name {
                    if !service.name.to_lowercase().contains(&name.to_lowercase()) {
                        return false;
                    }
                }

                true
            })
            .collect();
    }

    Ok(Json(ApiResponse::ok(services)))
}

pub async fn start_service(
    Path(name): Path<String>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    let docker = DockerManager::new()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    docker
        .start_service(&name)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ApiResponse::ok(format!("Service {} started", name))))
}

pub async fn stop_service(
    Path(name): Path<String>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    let docker = DockerManager::new()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    docker
        .stop_service(&name)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ApiResponse::ok(format!("Service {} stopped", name))))
}

pub async fn restart_service(
    Path(name): Path<String>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    let docker = DockerManager::new()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    docker
        .restart_service(&name)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ApiResponse::ok(format!("Service {} restarted", name))))
}

pub async fn get_logs(
    Path(name): Path<String>,
    Query(params): Query<LogsQuery>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    if params.follow {
        // Real-time log streaming is provided via WebSocket endpoint.
        return Err(StatusCode::NOT_IMPLEMENTED);
    }

    let docker = DockerManager::new()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let logs = docker
        .get_logs(&name, Some(params.tail))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ApiResponse::ok(logs)))
}

pub async fn get_logs_parsed(
    Path(name): Path<String>,
    Query(params): Query<ParsedLogsQuery>,
) -> Result<Json<ApiResponse<Vec<ParsedLogLine>>>, StatusCode> {
    let docker = DockerManager::new()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let logs = docker
        .get_logs(&name, Some(params.tail))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Parse each log line
    let mut parsed_logs: Vec<ParsedLogLine> = logs
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let parsed = log_parser::parse_docker_log_line(line);
            ParsedLogLine {
                timestamp: parsed.timestamp,
                level: parsed.level.to_string().to_string(),
                module: parsed.module_short,
                message: parsed.message,
            }
        })
        .collect();

    // Apply filters
    if let Some(ref level_filter) = params.level {
        let level_upper = level_filter.to_uppercase();
        parsed_logs.retain(|log| log.level.contains(&level_upper));
    }

    if let Some(ref module_filter) = params.module {
        let module_lower = module_filter.to_lowercase();
        parsed_logs.retain(|log| log.module.to_lowercase().contains(&module_lower));
    }

    Ok(Json(ApiResponse::ok(parsed_logs)))
}

// ============================================================================
// Wallet Handlers
// ============================================================================

pub async fn get_wallets(
) -> Result<Json<ApiResponse<Vec<crate::core::wallet::WalletInfo>>>, StatusCode> {
    let wallet_manager = WalletManager::new().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let wallets = wallet_manager
        .list_wallets()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ApiResponse::ok(wallets)))
}

pub async fn get_wallet_balance(
    Path(id): Path<usize>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    let wallet_manager = WalletManager::new().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let balance = wallet_manager
        .get_balance(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ApiResponse::ok(format!("{:.2} KAS", balance))))
}

pub async fn get_wallet_detail(
    Path(id): Path<usize>,
) -> Result<Json<ApiResponse<Vec<crate::core::wallet::UtxoInfo>>>, StatusCode> {
    let wallet_manager = WalletManager::new().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let utxos = wallet_manager
        .get_utxos(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ApiResponse::ok(utxos)))
}

// ============================================================================
// Storage Handlers
// ============================================================================

pub async fn get_storage() -> Result<Json<ApiResponse<storage::StorageAnalysis>>, StatusCode> {
    let analysis = storage::analyze_storage()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ApiResponse::ok(analysis)))
}

pub async fn get_storage_history(
) -> Result<Json<ApiResponse<Vec<storage::StorageMeasurement>>>, StatusCode> {
    let history = storage::StorageHistory::load().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ApiResponse::ok(history.measurements)))
}

pub async fn prune_storage() -> Result<Json<ApiResponse<String>>, StatusCode> {
    // Run docker system prune to clean up build cache
    let output = tokio::process::Command::new("docker")
        .args(&["system", "prune", "-f", "--volumes"])
        .output()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(Json(ApiResponse::ok(format!(
            "Prune completed: {}",
            stdout
        ))))
    } else {
        Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

pub async fn truncate_container_log(
    Path(container_id): Path<String>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    storage::truncate_container_log(&container_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ApiResponse::ok(format!(
        "Container log truncated successfully: {}",
        container_id
    ))))
}

pub async fn get_log_rotation_config(
) -> Result<Json<ApiResponse<storage::LogRotationConfig>>, StatusCode> {
    let config =
        storage::get_log_rotation_config().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ApiResponse::ok(config)))
}

pub async fn update_global_log_rotation(
    Json(settings): Json<storage::LogRotationSettings>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    storage::update_global_log_rotation(&settings)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ApiResponse::ok(
        "Global log rotation settings updated. Restart containers to apply changes.".to_string(),
    )))
}

pub async fn get_container_log_rotation(
    axum::extract::Path(container_name): axum::extract::Path<String>,
) -> Result<Json<ApiResponse<storage::LogRotationSettings>>, StatusCode> {
    let settings = storage::get_container_log_rotation(&container_name)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ApiResponse::ok(settings)))
}

pub async fn update_container_log_rotation(
    axum::extract::Path(container_name): axum::extract::Path<String>,
    Json(settings): Json<storage::LogRotationSettings>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    storage::update_container_log_rotation(&container_name, Some(&settings))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ApiResponse::ok(format!(
        "Log rotation settings updated for '{}'. Restart container to apply changes.",
        container_name
    ))))
}

pub async fn delete_container_log_rotation(
    axum::extract::Path(container_name): axum::extract::Path<String>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    storage::update_container_log_rotation(&container_name, None)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ApiResponse::ok(format!(
        "'{}' will now use global log rotation settings. Restart container to apply changes.",
        container_name
    ))))
}

// ============================================================================
// Configuration Handlers
// ============================================================================

pub async fn get_config() -> Result<Json<ApiResponse<HashMap<String, String>>>, StatusCode> {
    let config_manager =
        ConfigManager::load_from_project().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let config = config_manager.to_map();

    Ok(Json(ApiResponse::ok(config)))
}

pub async fn get_system_info(
) -> Result<Json<ApiResponse<crate::server::SystemResources>>, StatusCode> {
    let system_resources = crate::server::collect_system_resources();
    Ok(Json(ApiResponse::ok(system_resources)))
}

#[derive(Serialize)]
pub struct RpcToken {
    pub index: usize,
    pub token: Option<String>,
}

pub async fn get_rpc_tokens() -> Result<Json<ApiResponse<Vec<RpcToken>>>, StatusCode> {
    let config =
        ConfigManager::load_from_project().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let tokens: Vec<RpcToken> = config
        .get_rpc_tokens()
        .into_iter()
        .map(|(index, token)| RpcToken { index, token })
        .collect();

    Ok(Json(ApiResponse::ok(tokens)))
}

#[derive(Serialize)]
pub struct SslInfo {
    pub domain: Option<String>,
    pub has_ovh_config: bool,
}

pub async fn get_ssl_info() -> Result<Json<ApiResponse<SslInfo>>, StatusCode> {
    let config =
        ConfigManager::load_from_project().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let domain_config = config.get_domain_config();
    let info = SslInfo {
        domain: domain_config.as_ref().map(|d| d.domain.clone()),
        has_ovh_config: domain_config.map(|d| d.has_ovh_config()).unwrap_or(false),
    };

    Ok(Json(ApiResponse::ok(info)))
}

// ============================================================================
// Monitoring Handlers
// ============================================================================

pub async fn health_check() -> Result<Json<ApiResponse<String>>, StatusCode> {
    Ok(Json(ApiResponse::ok("healthy".to_string())))
}

#[derive(Serialize)]
pub struct MetricsInfo {
    system_cpu: f64,
    system_memory_percent: f64,
    system_disk_percent: f64,
    docker_containers_running: usize,
    docker_images: usize,
}

pub async fn get_metrics() -> Result<Json<ApiResponse<MetricsInfo>>, StatusCode> {
    use sysinfo::{CpuRefreshKind, Disks, MemoryRefreshKind, RefreshKind, System};

    let docker = DockerManager::new()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get Docker container count
    let containers = docker
        .list_containers()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Initialize system info with CPU refresh
    let mut sys = System::new_with_specifics(
        RefreshKind::new()
            .with_cpu(CpuRefreshKind::everything())
            .with_memory(MemoryRefreshKind::everything()),
    );

    // Refresh to get accurate metrics
    sys.refresh_all();

    // Get CPU usage (global)
    let system_cpu = sys.global_cpu_info().cpu_usage() as f64;

    // Get memory usage percentage
    let memory_total = sys.total_memory() as f64;
    let memory_used = sys.used_memory() as f64;
    let system_memory_percent = if memory_total > 0.0 {
        (memory_used / memory_total) * 100.0
    } else {
        0.0
    };

    // Get disk usage for root filesystem
    let disks = Disks::new_with_refreshed_list();
    let system_disk_percent = disks
        .iter()
        .find(|disk| disk.mount_point().to_str() == Some("/"))
        .map(|disk| {
            let total = disk.total_space() as f64;
            let available = disk.available_space() as f64;
            let used = total - available;
            if total > 0.0 {
                (used / total) * 100.0
            } else {
                0.0
            }
        })
        .unwrap_or(0.0);

    let metrics = MetricsInfo {
        system_cpu,
        system_memory_percent,
        system_disk_percent,
        docker_containers_running: containers.len(),
        docker_images: 0, // Not used by frontend, can be implemented if needed
    };

    Ok(Json(ApiResponse::ok(metrics)))
}

// ============================================================================
// Profile Handlers
// ============================================================================

#[derive(Serialize)]
pub struct ProfileInfo {
    name: String,
    is_active: bool,
    services: Vec<String>,
}

pub async fn get_profiles() -> Result<Json<ApiResponse<Vec<ProfileInfo>>>, StatusCode> {
    let docker = DockerManager::new()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let containers = docker
        .list_containers()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let active_profiles = DockerManager::get_active_profiles_from_list(&containers);

    // Define all available profiles and their services
    let all_profiles = vec![
        ("kaspad", vec!["kaspad", "kaspa-miner"]),
        (
            "backend",
            vec!["execution-layer", "block-builder", "viaduct"],
        ),
        (
            "frontend-w1",
            vec!["traefik", "rpc-provider-0", "kaswallet-0"],
        ),
        ("frontend-w2", vec!["rpc-provider-1", "kaswallet-1"]),
        ("frontend-w3", vec!["rpc-provider-2", "kaswallet-2"]),
        ("frontend-w4", vec!["rpc-provider-3", "kaswallet-3"]),
        ("frontend-w5", vec!["rpc-provider-4", "kaswallet-4"]),
        (
            "kaswallets",
            vec![
                "kaswallet-0",
                "kaswallet-1",
                "kaswallet-2",
                "kaswallet-3",
                "kaswallet-4",
            ],
        ),
        (
            "rpc-providers",
            vec![
                "rpc-provider-0",
                "rpc-provider-1",
                "rpc-provider-2",
                "rpc-provider-3",
                "rpc-provider-4",
            ],
        ),
    ];

    let profiles: Vec<ProfileInfo> = all_profiles
        .into_iter()
        .map(|(name, services)| ProfileInfo {
            name: name.to_string(),
            is_active: active_profiles.contains(&name.to_string()),
            services: services.into_iter().map(|s| s.to_string()).collect(),
        })
        .collect();

    Ok(Json(ApiResponse::ok(profiles)))
}

pub async fn start_profile(
    Path(name): Path<String>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    let docker = DockerManager::new()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    docker
        .start_profile(&name)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ApiResponse::ok(format!("Profile {} started", name))))
}

pub async fn stop_profile(
    Path(name): Path<String>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    let docker = DockerManager::new()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    docker
        .stop_profile(&name)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ApiResponse::ok(format!("Profile {} stopped", name))))
}

// ============================================================================
// Transaction Monitoring Handlers
// ============================================================================

use crate::core::l2_monitor::{
    Statistics, TransactionInfo as L2TransactionInfo, TransactionMonitor,
};

#[derive(Serialize)]
pub struct TransactionInfo {
    hash: String,
    from: String,
    to: Option<String>,
    value_ikas: f64,
    gas_fee_ikas: f64,
    block_number: u64,
    timestamp: String,
    status: bool,
    tx_type: String,
    l1_fee_kas: Option<f64>,
}

impl From<L2TransactionInfo> for TransactionInfo {
    fn from(tx: L2TransactionInfo) -> Self {
        let value_ikas = tx.value_ikas();
        let gas_fee_ikas = tx.gas_fee_ikas();

        TransactionInfo {
            hash: tx.hash,
            from: tx.from,
            to: tx.to,
            value_ikas,
            gas_fee_ikas,
            block_number: tx.block_number,
            timestamp: tx.timestamp.to_rfc3339(),
            status: tx.status,
            tx_type: format!("{:?}", tx.tx_type),
            l1_fee_kas: tx.l1_fee,
        }
    }
}

#[derive(Serialize)]
pub struct TransactionStats {
    current_block: u64,
    total_transactions: u64,
    successful_transactions: u64,
    failed_transactions: u64,
    total_gas_fees_ikas: f64,
    total_l1_fees_kas: f64,
    tps: f64,
    uptime: String,
}

impl From<Statistics> for TransactionStats {
    fn from(stats: Statistics) -> Self {
        TransactionStats {
            current_block: stats.current_block,
            total_transactions: stats.total_transactions,
            successful_transactions: stats.successful_transactions,
            failed_transactions: stats.failed_transactions,
            total_gas_fees_ikas: stats.total_gas_fees_ikas,
            total_l1_fees_kas: stats.total_l1_fees_kas,
            tps: stats.tps(),
            uptime: stats.uptime(),
        }
    }
}

#[derive(Deserialize)]
pub struct TransactionsQuery {
    #[serde(default = "default_tx_limit")]
    limit: usize,
    #[serde(default)]
    filter: Option<String>, // all, transfer, contract, entry
}

fn default_tx_limit() -> usize {
    50
}

pub async fn get_transactions(
    Query(params): Query<TransactionsQuery>,
) -> Result<Json<ApiResponse<Vec<TransactionInfo>>>, StatusCode> {
    let monitor = TransactionMonitor::new()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let transactions = monitor
        .poll_new_transactions()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Convert and filter
    let mut converted: Vec<TransactionInfo> =
        transactions.into_iter().map(|tx| tx.into()).collect();

    // Apply filter if specified
    if let Some(filter) = params.filter {
        let filter_lower = filter.to_lowercase();
        converted.retain(|tx| {
            filter_lower == "all" || tx.tx_type.to_lowercase().contains(&filter_lower)
        });
    }

    // Limit results
    converted.truncate(params.limit);

    Ok(Json(ApiResponse::ok(converted)))
}

pub async fn get_transaction_stats() -> Result<Json<ApiResponse<TransactionStats>>, StatusCode> {
    let monitor = TransactionMonitor::new()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let stats = monitor.get_statistics().await;

    Ok(Json(ApiResponse::ok(stats.into())))
}

// ============================================================================
// Version Management Handler
// ============================================================================

/// Check for updates from GitHub releases
/// Uses core::updater module - same business logic as TUI and CLI
pub async fn get_version_info() -> Result<Json<ApiResponse<updater::VersionInfo>>, StatusCode> {
    let version_info = updater::check_for_updates()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ApiResponse::ok(version_info)))
}

#[derive(Serialize)]
pub struct UpdateStatus {
    message: String,
    step: String,
    success: bool,
}

/// Trigger automatic update
/// Downloads latest release, installs it, and restarts the service
pub async fn trigger_update() -> Result<Json<ApiResponse<UpdateStatus>>, StatusCode> {
    use std::fs;
    use std::path::Path;
    use std::process::Command;

    // Download latest release to /tmp
    let download_path = Path::new("/tmp/igra-cli-update");

    match updater::download_latest_release(download_path).await {
        Ok(_) => {
            // Extract the binary from the tarball to a temp location
            let extract_result = Command::new("tar")
                .args(&["-xzf", download_path.to_str().unwrap(), "-C", "/tmp"])
                .output();

            match extract_result {
                Err(e) => {
                    return Ok(Json(ApiResponse::ok(UpdateStatus {
                        message: format!("Failed to run tar command: {}", e),
                        step: "extract_failed".to_string(),
                        success: false,
                    })));
                }
                Ok(output) if !output.status.success() => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Ok(Json(ApiResponse::ok(UpdateStatus {
                        message: format!("Failed to extract tarball: {}", stderr),
                        step: "extract_failed".to_string(),
                        success: false,
                    })));
                }
                _ => {}
            }

            // The extracted binary is now at /tmp/igra-cli
            let new_binary = Path::new("/tmp/igra-cli");

            // Make executable
            let _ = Command::new("chmod")
                .args(&["+x", new_binary.to_str().unwrap()])
                .output();

            // Create an update script that will be executed by the new binary
            let update_script = r#"#!/bin/bash
# Stop the service
systemctl stop igra-web-ui 2>/dev/null || sudo systemctl stop igra-web-ui

# Copy new binary
cp /tmp/igra-cli /usr/local/bin/igra-cli 2>/dev/null || sudo cp /tmp/igra-cli /usr/local/bin/igra-cli

# Clean up
rm -f /tmp/igra-cli /tmp/igra-cli-update /tmp/igra-update.sh

# Start the service
systemctl start igra-web-ui 2>/dev/null || sudo systemctl start igra-web-ui
"#;

            // Write the update script
            let script_path = Path::new("/tmp/igra-update.sh");
            if let Err(e) = fs::write(script_path, update_script) {
                return Ok(Json(ApiResponse::ok(UpdateStatus {
                    message: format!("Failed to create update script: {}", e),
                    step: "script_failed".to_string(),
                    success: false,
                })));
            }

            // Make script executable
            let _ = Command::new("chmod")
                .args(&["+x", script_path.to_str().unwrap()])
                .output();

            // Schedule the update to run in 2 seconds
            // This allows the response to be sent before we kill ourselves
            tokio::spawn(async {
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                // Use systemd-run to execute the script detached from the service
                // This ensures the script continues after the service stops
                let _ = Command::new("systemd-run")
                    .args(&[
                        "--scope",
                        "--unit=igra-cli-update",
                        "bash",
                        "/tmp/igra-update.sh",
                    ])
                    .spawn();
            });

            Ok(Json(ApiResponse::ok(UpdateStatus {
                message: "Update downloaded! Service will restart in 2 seconds...".to_string(),
                step: "completed".to_string(),
                success: true,
            })))
        }
        Err(e) => Ok(Json(ApiResponse::ok(UpdateStatus {
            message: format!("Failed to download update: {}", e),
            step: "download_failed".to_string(),
            success: false,
        }))),
    }
}

/// Restart the igra-web-ui systemd service
pub async fn restart_igra_service() -> Result<Json<ApiResponse<UpdateStatus>>, StatusCode> {
    use std::process::Command;

    // Schedule the restart to run in 2 seconds
    // This allows the response to be sent before we kill ourselves
    tokio::spawn(async {
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        // Try with and without sudo
        let _ = Command::new("systemctl")
            .args(&["restart", "igra-web-ui"])
            .spawn()
            .or_else(|_| {
                Command::new("sudo")
                    .args(&["systemctl", "restart", "igra-web-ui"])
                    .spawn()
            });
    });

    Ok(Json(ApiResponse::ok(UpdateStatus {
        message: "Service will restart in 2 seconds... Please refresh this page in a few seconds."
            .to_string(),
        step: "restarting".to_string(),
        success: true,
    })))
}

// ============================================================================
// Service Details & Notes Endpoints
// ============================================================================

#[derive(Deserialize)]
pub struct UpdateNoteRequest {
    note: String,
}

/// GET /api/services/:name/details - Get comprehensive service details
pub async fn get_service_details(
    Path(service_name): Path<String>,
) -> Result<Json<crate::core::docker::ServiceDetails>, StatusCode> {
    let docker = DockerManager::new()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let details = docker
        .get_service_details(&service_name)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(details))
}

/// GET /api/services/:name/note - Get service note
pub async fn get_service_note(
    Path(service_name): Path<String>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    let docker = DockerManager::new()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get container to determine image
    let containers = docker
        .list_containers()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let container = containers
        .iter()
        .find(|c| c.name == service_name || c.name.trim_start_matches('/') == service_name)
        .ok_or(StatusCode::NOT_FOUND)?;

    let notes = crate::core::service_notes::ServiceNotes::load().unwrap_or_default();

    let note = notes.get_note(&service_name, &container.image);

    Ok(Json(ApiResponse::ok(note)))
}

/// PUT /api/services/:name/note - Update service note
pub async fn update_service_note(
    Path(service_name): Path<String>,
    Json(payload): Json<UpdateNoteRequest>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    let mut notes = crate::core::service_notes::ServiceNotes::load().unwrap_or_default();

    notes.set_note(service_name.clone(), payload.note.clone());

    notes
        .save()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ApiResponse::ok(
        "Note updated successfully".to_string(),
    )))
}

// ============================================================================
// User Management Endpoints (Admin Only)
// ============================================================================

use crate::server::auth_handlers::{require_admin, require_auth};

use crate::server::auth_backend::FileAuthBackend;
#[cfg(feature = "server")]
use axum_login::AuthSession;

#[derive(Serialize)]
pub struct UserInfo {
    pub username: String,
    pub roles: Vec<String>,
    pub enabled: bool,
}

#[derive(Deserialize)]
pub struct AddUserRequest {
    pub username: String,
    pub password: String,
    pub roles: Vec<String>,
}

#[derive(Deserialize)]
pub struct ResetPasswordRequest {
    pub password: String,
}

#[derive(Deserialize)]
pub struct UpdateUserRolesRequest {
    pub roles: Vec<String>,
}

/// GET /api/users - List all users (admin only)
#[cfg(feature = "server")]
pub async fn get_users(
    auth_session: AuthSession<FileAuthBackend>,
) -> Result<Json<ApiResponse<Vec<UserInfo>>>, StatusCode> {
    let user = require_auth(auth_session).await?;
    require_admin(&user)?;

    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("igra-cli");

    let user_mgr =
        crate::core::UserManager::new(config_dir).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let users = user_mgr
        .load_users()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let user_infos: Vec<UserInfo> = users
        .into_iter()
        .map(|u| UserInfo {
            username: u.username,
            roles: u.roles.iter().map(|r| r.to_string()).collect(),
            enabled: u.enabled,
        })
        .collect();

    Ok(Json(ApiResponse::ok(user_infos)))
}

/// POST /api/users - Add a new user (admin only)
#[cfg(feature = "server")]
pub async fn add_user(
    auth_session: AuthSession<FileAuthBackend>,
    Json(payload): Json<AddUserRequest>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    let user = require_auth(auth_session).await?;
    require_admin(&user)?;

    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("igra-cli");

    let user_mgr =
        crate::core::UserManager::new(config_dir).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Parse roles
    let role_set: std::collections::HashSet<_> = payload
        .roles
        .iter()
        .filter_map(|r| r.parse().ok())
        .collect();

    // Hash password
    let password_hash = crate::core::user_manager::hash_password(&payload.password)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Create user
    let new_user = crate::core::User::new(payload.username.clone(), password_hash, role_set);
    user_mgr
        .add_user(new_user)
        .map_err(|_| StatusCode::CONFLICT)?;

    Ok(Json(ApiResponse::ok(format!(
        "User '{}' created successfully",
        payload.username
    ))))
}

/// DELETE /api/users/:username - Remove a user (admin only)
#[cfg(feature = "server")]
pub async fn delete_user(
    auth_session: AuthSession<FileAuthBackend>,
    axum::extract::Path(username): axum::extract::Path<String>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    let user = require_auth(auth_session).await?;
    require_admin(&user)?;

    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("igra-cli");

    let user_mgr =
        crate::core::UserManager::new(config_dir).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    user_mgr
        .remove_user(&username)
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(ApiResponse::ok(format!(
        "User '{}' deleted successfully",
        username
    ))))
}

/// PUT /api/users/:username/password - Reset user password (admin only)
#[cfg(feature = "server")]
pub async fn reset_user_password(
    auth_session: AuthSession<FileAuthBackend>,
    axum::extract::Path(username): axum::extract::Path<String>,
    Json(payload): Json<ResetPasswordRequest>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    let user = require_auth(auth_session).await?;
    require_admin(&user)?;

    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("igra-cli");

    let user_mgr =
        crate::core::UserManager::new(config_dir).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Hash password
    let password_hash = crate::core::user_manager::hash_password(&payload.password)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get existing user and update
    let existing_user_opt = user_mgr
        .get_user(&username)
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let mut existing_user = existing_user_opt.ok_or(StatusCode::NOT_FOUND)?;

    existing_user.password_hash = password_hash;
    existing_user.force_password_change = false; // Clear flag after password change

    user_mgr
        .update_user(&username, existing_user)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ApiResponse::ok(format!(
        "Password for '{}' reset successfully",
        username
    ))))
}

/// PUT /api/users/:username/roles - Update user roles (admin only)
#[cfg(feature = "server")]
pub async fn update_user_roles(
    auth_session: AuthSession<FileAuthBackend>,
    axum::extract::Path(username): axum::extract::Path<String>,
    Json(payload): Json<UpdateUserRolesRequest>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    let user = require_auth(auth_session).await?;
    require_admin(&user)?;

    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("igra-cli");

    let user_mgr =
        crate::core::UserManager::new(config_dir).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get existing user
    let existing_user_opt = user_mgr
        .get_user(&username)
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let existing_user = existing_user_opt.ok_or(StatusCode::NOT_FOUND)?;

    // Parse new roles
    let role_set: std::collections::HashSet<_> = payload
        .roles
        .iter()
        .filter_map(|r| r.parse().ok())
        .collect();

    // Create updated user with new roles
    let mut updated_user = crate::core::User::new(
        existing_user.username.clone(),
        existing_user.password_hash.clone(),
        role_set,
    );
    updated_user.force_password_change = existing_user.force_password_change;

    user_mgr
        .update_user(&username, updated_user)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ApiResponse::ok(format!(
        "Roles for '{}' updated successfully",
        username
    ))))
}

// ============================================================================
// Security Management Endpoints (Admin Only)
// ============================================================================

#[derive(Deserialize)]
pub struct AddNetworkRequest {
    pub network: String,
}

/// GET /api/security - Get security configuration (admin only)
#[cfg(feature = "server")]
pub async fn get_security_config(
    auth_session: AuthSession<FileAuthBackend>,
) -> Result<Json<ApiResponse<crate::core::IpAllowlist>>, StatusCode> {
    let user = require_auth(auth_session).await?;
    require_admin(&user)?;

    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("igra-cli");

    let security_mgr = crate::core::SecurityManager::new(config_dir)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let config = security_mgr
        .load_config()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ApiResponse::ok(config)))
}

/// POST /api/security/ips - Add IP network to allowlist (admin only)
#[cfg(feature = "server")]
pub async fn add_allowed_network(
    auth_session: AuthSession<FileAuthBackend>,
    Json(payload): Json<AddNetworkRequest>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    let user = require_auth(auth_session).await?;
    require_admin(&user)?;

    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("igra-cli");

    let security_mgr = crate::core::SecurityManager::new(config_dir)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    security_mgr
        .add_network(payload.network.clone())
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    Ok(Json(ApiResponse::ok(format!(
        "Added {} to allowlist",
        payload.network
    ))))
}

/// DELETE /api/security/ips/:network - Remove IP network from allowlist (admin only)
#[cfg(feature = "server")]
pub async fn remove_allowed_network(
    auth_session: AuthSession<FileAuthBackend>,
    axum::extract::Path(network): axum::extract::Path<String>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    let user = require_auth(auth_session).await?;
    require_admin(&user)?;

    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("igra-cli");

    let security_mgr = crate::core::SecurityManager::new(config_dir)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let removed = security_mgr
        .remove_network(&network)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if removed {
        Ok(Json(ApiResponse::ok(format!(
            "Removed {} from allowlist",
            network
        ))))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

// ============================================================================
// Audit Log Endpoints (Admin Only)
// ============================================================================

/// GET /api/audit - Get recent audit log entries (admin only)
#[cfg(feature = "server")]
pub async fn get_audit_logs(
    auth_session: AuthSession<FileAuthBackend>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<ApiResponse<Vec<crate::core::AuditEvent>>>, StatusCode> {
    let user = require_auth(auth_session).await?;
    require_admin(&user)?;

    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("igra-cli");

    let audit_logger =
        crate::core::AuditLogger::new(config_dir).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let limit = params
        .get("limit")
        .and_then(|l| l.parse().ok())
        .unwrap_or(50);

    let events = audit_logger
        .read_recent(limit)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ApiResponse::ok(events)))
}

/// GET /api/audit/export - Export all audit logs (admin only)
#[cfg(feature = "server")]
pub async fn export_audit_logs(
    auth_session: AuthSession<FileAuthBackend>,
) -> Result<Json<ApiResponse<Vec<crate::core::AuditEvent>>>, StatusCode> {
    let user = require_auth(auth_session).await?;
    require_admin(&user)?;

    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("igra-cli");

    let audit_logger =
        crate::core::AuditLogger::new(config_dir).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let events = audit_logger
        .export_all()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ApiResponse::ok(events)))
}

// ============================================================================
// Network Topology Connection Parsers
// ============================================================================

/// Parse nginx configuration files for proxy_pass directives
fn parse_nginx_proxies() -> Vec<DetectedConnection> {
    let mut connections = Vec::new();

    // Common nginx config locations
    let config_paths = vec![
        "/etc/nginx/nginx.conf",
        "/etc/nginx/conf.d",
        "/etc/nginx/sites-enabled",
    ];

    for config_path in config_paths {
        let path = std::path::Path::new(config_path);
        if !path.exists() {
            continue;
        }

        if path.is_file() {
            if let Ok(content) = std::fs::read_to_string(path) {
                connections.extend(extract_nginx_proxy_pass(&content));
            }
        } else if path.is_dir() {
            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries.flatten() {
                    if let Ok(content) = std::fs::read_to_string(entry.path()) {
                        connections.extend(extract_nginx_proxy_pass(&content));
                    }
                }
            }
        }
    }

    connections
}

/// Extract proxy_pass directives from nginx config content
fn extract_nginx_proxy_pass(content: &str) -> Vec<DetectedConnection> {
    let mut connections = Vec::new();

    // Parse proxy_pass lines: proxy_pass http://target:port;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("proxy_pass") {
            if let Some(url_part) = trimmed
                .strip_prefix("proxy_pass")
                .and_then(|s| s.trim().strip_suffix(';'))
            {
                let url = url_part.trim();
                if let Some((host, port, protocol)) = parse_url_value(url) {
                    connections.push(DetectedConnection {
                        source: "systemd_nginx".to_string(),
                        target: host,
                        port,
                        protocol,
                        connection_type: "http".to_string(),
                        label: Some("proxy_pass".to_string()),
                    });
                }
            }
        }
    }

    connections
}

/// Parse environment variables for connection URLs and hostnames
fn parse_env_connections(
    container_name: &str,
    env_vars: &HashMap<String, String>,
) -> Vec<DetectedConnection> {
    let mut connections = Vec::new();

    for (key, value) in env_vars {
        // Skip hidden/sensitive values
        if value == "***HIDDEN***" {
            continue;
        }

        // Parse URL patterns (*_URL, *_ADDR, *_HOST ending vars)
        if key.ends_with("_URL") || key.ends_with("_ADDR") || key.ends_with("_HOST") {
            if let Some(conn) = parse_url_value(value) {
                connections.push(DetectedConnection {
                    source: container_name.to_string(),
                    target: conn.0,
                    port: conn.1,
                    protocol: conn.2.clone(),
                    connection_type: if conn.2 == "ws" || conn.2 == "wss" {
                        "websocket".to_string()
                    } else {
                        "http".to_string()
                    },
                    label: Some(key.clone()),
                });
            }
        }
    }

    connections
}

/// Parse URL or host:port string
fn parse_url_value(value: &str) -> Option<(String, Option<u16>, String)> {
    if value.contains("://") {
        // URL format: http://host:port or ws://host:port
        if let Some((protocol, rest)) = value.split_once("://") {
            let parts: Vec<&str> = rest.split('/').next().unwrap_or(rest).split(':').collect();
            let host = parts.get(0)?.to_string();
            let port = parts.get(1).and_then(|p| p.parse().ok());
            return Some((host, port, protocol.to_string()));
        }
    } else if value.contains(':') {
        // host:port format
        if let Some((host, port_str)) = value.split_once(':') {
            let port = port_str.parse().ok();
            return Some((host.to_string(), port, "tcp".to_string()));
        }
    }
    None
}

/// Parse command arguments for connection strings
fn parse_arg_connections(
    container_name: &str,
    command: &Option<String>,
) -> Vec<DetectedConnection> {
    let mut connections = Vec::new();

    if let Some(cmd) = command {
        let args: Vec<&str> = cmd.split_whitespace().collect();
        let mut i = 0;

        while i < args.len() {
            let arg = args[i];

            // Pattern 1: --flag=value
            if arg.starts_with("--") && arg.contains('=') {
                if let Some((_, value)) = arg.split_once('=') {
                    if let Some(conn) = parse_url_value(value) {
                        connections.push(DetectedConnection {
                            source: container_name.to_string(),
                            target: conn.0,
                            port: conn.1,
                            protocol: conn.2.clone(),
                            connection_type: if conn.2 == "ws" {
                                "websocket".to_string()
                            } else {
                                "tcp".to_string()
                            },
                            label: Some(arg.split('=').next().unwrap().to_string()),
                        });
                    }
                }
            }
            // Pattern 2: --server ws://host:port (value in next arg)
            else if arg == "--server" && i + 1 < args.len() {
                if let Some(conn) = parse_url_value(args[i + 1]) {
                    connections.push(DetectedConnection {
                        source: container_name.to_string(),
                        target: conn.0,
                        port: conn.1,
                        protocol: conn.2.clone(),
                        connection_type: if conn.2 == "ws" {
                            "websocket".to_string()
                        } else {
                            "tcp".to_string()
                        },
                        label: Some("--server".to_string()),
                    });
                }
                i += 1;
            }

            i += 1;
        }
    }

    connections
}

/// Detect IPC socket connections via shared volumes
fn detect_ipc_connections(
    containers: &[(String, Vec<(String, String)>)],
) -> Vec<DetectedConnection> {
    let mut connections = Vec::new();
    let mut volume_map: HashMap<String, Vec<String>> = HashMap::new();

    // Group containers by shared volumes
    for (container_name, mounts) in containers {
        for (volume_name, _dest) in mounts {
            // Only consider named volumes or tmpfs mounts (like reth_ipc)
            if !volume_name.starts_with('/') {
                volume_map
                    .entry(volume_name.clone())
                    .or_insert_with(Vec::new)
                    .push(container_name.clone());
            }
        }
    }

    // Create IPC connections for shared volumes
    for (volume_name, container_names) in volume_map {
        if container_names.len() >= 2
            && (volume_name.contains("ipc") || volume_name.contains("sock"))
        {
            // Create bidirectional connections for IPC
            for i in 0..container_names.len() {
                for j in (i + 1)..container_names.len() {
                    connections.push(DetectedConnection {
                        source: container_names[i].clone(),
                        target: container_names[j].clone(),
                        port: None,
                        protocol: "ipc".to_string(),
                        connection_type: "ipc".to_string(),
                        label: Some(volume_name.clone()),
                    });
                }
            }
        }
    }

    connections
}

/// Parse Traefik labels for routing rules
fn parse_traefik_labels(
    container_name: &str,
    labels: &HashMap<String, String>,
) -> Vec<DetectedConnection> {
    let mut connections = Vec::new();

    // Check if traefik is enabled
    if labels.get("traefik.enable") != Some(&"true".to_string()) {
        return connections;
    }

    // Find router and service configurations
    // Format: traefik.http.routers.<name>.rule = "PathPrefix(/token)"
    //         traefik.http.services.<name>.loadbalancer.server.port = "8535"

    let mut service_port = None;
    let mut router_rule = None;

    for (key, value) in labels {
        if key.contains(".services.") && key.ends_with(".port") {
            service_port = value.parse().ok();
        }
        if key.contains(".routers.") && key.ends_with(".rule") {
            router_rule = Some(value.clone());
        }
    }

    // If we found traefik routing config, add a connection from traefik to this container
    if service_port.is_some() || router_rule.is_some() {
        // Parse the router rule to create a cleaner label
        let clean_label = router_rule.as_ref().and_then(|rule| {
            let mut parts = Vec::new();

            // Extract Host() value
            if let Some(start) = rule.find("Host(") {
                if let Some(end) = rule[start..].find(')') {
                    let host_part = &rule[start + 5..start + end];
                    // Remove quotes and backticks
                    let host = host_part.trim_matches(|c| c == '\'' || c == '`' || c == '"');
                    parts.push(host.to_string());
                }
            }

            // Extract PathPrefix() value
            if let Some(start) = rule.find("PathPrefix(") {
                if let Some(end) = rule[start..].find(')') {
                    let path_part = &rule[start + 11..start + end];
                    // Remove quotes and backticks
                    let path = path_part.trim_matches(|c| c == '\'' || c == '`' || c == '"');

                    // Truncate long paths (common with routing tokens)
                    let truncated_path = if path.len() > 30 {
                        // If it's a long token/hash, just show /{token}/...
                        if path.starts_with('/') && path.matches('/').count() >= 2 {
                            let first_segment = path.split('/').nth(1).unwrap_or("");
                            if first_segment.len() > 15 {
                                "/{token}/...".to_string()
                            } else {
                                format!("/{}/...", first_segment)
                            }
                        } else {
                            format!("{}...", &path[..27])
                        }
                    } else {
                        path.to_string()
                    };

                    parts.push(truncated_path);
                }
            }

            if !parts.is_empty() {
                Some(parts.join(""))
            } else {
                None
            }
        });

        connections.push(DetectedConnection {
            source: "traefik".to_string(),
            target: container_name.to_string(),
            port: service_port,
            protocol: "http".to_string(),
            connection_type: "http".to_string(),
            label: clean_label,
        });
    }

    connections
}

// ============================================================================
// Network Topology Handler
// ============================================================================

/// GET /api/network-topology - Get network topology visualization data
pub async fn get_network_topology() -> Result<Json<ApiResponse<NetworkTopology>>, StatusCode> {
    let docker = DockerManager::new()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut all_connections = Vec::new();
    let mut container_mounts: Vec<(String, Vec<(String, String)>)> = Vec::new();

    // Initialize data source modules
    use crate::core::{FirewallManager, NetworkInfoDetector, NginxParser, SecurityScanner};

    let firewall_mgr = FirewallManager::new();
    let nginx_parser = NginxParser::new();
    let network_detector = NetworkInfoDetector::new();
    let security_scanner = SecurityScanner::new();

    // Gather data from all sources
    let firewall_status = firewall_mgr.get_status().ok();
    let nginx_sites = nginx_parser.parse_sites().ok().unwrap_or_default();
    let network_info = network_detector.get_info();
    let nginx_proxy_targets = nginx_parser.get_proxy_targets(&nginx_sites);

    // Get Docker networks with CIDRs
    let docker_networks = docker.list_networks().await.ok().unwrap_or_default();

    // 1. Add router node in internet layer (external network access point)
    nodes.push(NetworkNode {
        id: "router".to_string(),
        label: "Router".to_string(),
        node_type: "gateway".to_string(),
        status: "active".to_string(),
        ports: vec![],
        ip_address: network_info
            .public_ipv4
            .clone()
            .or_else(|| Some("External Gateway".to_string())),
        layer: "internet".to_string(),
        metadata: {
            let mut m = HashMap::new();
            m.insert(
                "description".to_string(),
                "External router providing internet connectivity".to_string(),
            );
            if let Some(ref ipv4) = network_info.public_ipv4 {
                m.insert("public_ipv4".to_string(), ipv4.clone());
            }
            if let Some(ref ipv6) = network_info.public_ipv6 {
                m.insert("public_ipv6".to_string(), ipv6.clone());
            }
            if let Some(ref hostname) = network_info.hostname {
                m.insert("hostname".to_string(), hostname.clone());
            }
            m
        },
        warnings: vec![],
        domains: network_info.domains.iter().cloned().collect(),
    });

    // 2. Add firewall node in firewall layer (LAN gateway with firewall)
    nodes.push(NetworkNode {
        id: "firewall".to_string(),
        label: "Firewall (UFW)".to_string(),
        node_type: "gateway".to_string(),
        status: if firewall_status.is_some() {
            "active".to_string()
        } else {
            "unknown".to_string()
        },
        ports: vec![],
        ip_address: network_info
            .lan_ip
            .clone()
            .or_else(|| Some("LAN Gateway".to_string())),
        layer: "firewall".to_string(),
        metadata: {
            let mut m = HashMap::new();
            m.insert(
                "description".to_string(),
                "Firewall protecting internal network".to_string(),
            );
            if let Some(ref fw_status) = firewall_status {
                m.insert("ufw_active".to_string(), fw_status.active.to_string());
                m.insert("rule_count".to_string(), fw_status.rules.len().to_string());
            }
            if let Some(ref lan_ip) = network_info.lan_ip {
                m.insert("lan_ip".to_string(), lan_ip.clone());
            }
            m
        },
        warnings: vec![],
        domains: vec![],
    });

    // Add Router → Firewall edge
    edges.push(NetworkEdge {
        source: "router".to_string(),
        target: "firewall".to_string(),
        edge_type: "gateway".to_string(),
        label: Some("WAN → LAN".to_string()),
        protocol: Some("gateway".to_string()),
        metadata: HashMap::new(),
    });

    // 2. Get all Docker containers with full details
    let containers = docker
        .list_containers_filtered(true)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    for container in &containers {
        let mut metadata = HashMap::new();
        metadata.insert("image".to_string(), container.image.clone());
        if let Some(project) = &container.project_name {
            metadata.insert("project".to_string(), project.clone());
        }

        // Get detailed service info
        let details = docker.get_service_details(&container.name).await.ok();

        let ip_address = details
            .as_ref()
            .and_then(|d| d.networks.first())
            .map(|n| n.ip_address.clone());

        // Determine layer for Docker containers
        // Containers with nginx/traefik go in gateway layer, others in docker layer
        let is_gateway = container.name.contains("traefik")
            || container.name.contains("nginx")
            || container.image.contains("traefik")
            || container.image.contains("nginx");

        let layer = if is_gateway {
            "gateway".to_string()
        } else {
            "docker".to_string()
        };

        // Extract domains for gateway services from nginx config
        let mut container_domains = Vec::new();
        if is_gateway {
            for site in &nginx_sites {
                container_domains.extend(site.server_names.clone());
            }
        }

        // Collect exposed ports for security scanning
        let exposed_ports: Vec<(u16, String)> = container
            .ports
            .iter()
            .filter_map(|p| {
                if let Some(port_num) = p.split('/').next().and_then(|s| s.parse().ok()) {
                    Some((port_num, container.name.clone()))
                } else {
                    None
                }
            })
            .collect();

        // Scan for security issues
        let mut container_warnings = Vec::new();
        if !exposed_ports.is_empty() {
            let warnings = security_scanner.scan_public_ports(&exposed_ports);
            container_warnings.extend(
                warnings
                    .iter()
                    .map(|w| format!("{}: {}", w.severity, w.description)),
            );
        }

        nodes.push(NetworkNode {
            id: container.name.clone(),
            label: container.name.clone(),
            node_type: "container".to_string(),
            status: container.status.clone(),
            ports: container.ports.clone(),
            ip_address,
            layer,
            metadata,
            warnings: container_warnings,
            domains: container_domains,
        });

        // Parse connections from container details
        if let Some(ref d) = details {
            // Parse environment variables
            all_connections.extend(parse_env_connections(&container.name, &d.env_vars));

            // Parse command arguments
            all_connections.extend(parse_arg_connections(&container.name, &d.command));

            // Parse Traefik labels
            all_connections.extend(parse_traefik_labels(&container.name, &d.labels));

            // Collect mounts for IPC detection
            let mounts: Vec<(String, String)> = d
                .mounts
                .iter()
                .map(|m| (m.source.clone(), m.destination.clone()))
                .collect();
            container_mounts.push((container.name.clone(), mounts));
        }

        // Add Docker network membership edges
        if let Some(details) = &details {
            for network in &details.networks {
                let network_id = format!("network_{}", network.name);

                // Add network node if not present
                if !nodes.iter().any(|n| n.id == network_id) {
                    // Find CIDR for this network
                    let cidr = docker_networks
                        .iter()
                        .find(|n| n.name == network.name)
                        .and_then(|n| n.cidr.clone());

                    let mut network_label = network.name.clone();
                    if let Some(ref c) = cidr {
                        network_label = format!("{} ({})", network.name, c);
                    }

                    nodes.push(NetworkNode {
                        id: network_id.clone(),
                        label: network_label,
                        node_type: "network".to_string(),
                        status: "active".to_string(),
                        ports: vec![],
                        ip_address: Some(network.gateway.clone()),
                        layer: "docker".to_string(), // Networks are part of docker layer
                        metadata: {
                            let mut m = HashMap::new();
                            m.insert("gateway".to_string(), network.gateway.clone());
                            if let Some(ref c) = cidr {
                                m.insert("cidr".to_string(), c.clone());
                            }
                            m
                        },
                        warnings: vec![],
                        domains: vec![],
                    });
                }

                // Add network membership edge
                edges.push(NetworkEdge {
                    source: container.name.clone(),
                    target: network_id,
                    edge_type: "network".to_string(),
                    label: Some(network.ip_address.clone()),
                    protocol: Some("network".to_string()),
                    metadata: {
                        let mut m = HashMap::new();
                        m.insert("ip_address".to_string(), network.ip_address.clone());
                        m
                    },
                });
            }
        }
    }

    // 3. Detect IPC connections from shared volumes
    all_connections.extend(detect_ipc_connections(&container_mounts));

    // 3a. Parse nginx proxy_pass directives from common config locations
    all_connections.extend(parse_nginx_proxies());

    // 3b. Parse nginx proxy_pass connections
    for (site_name, proxy_targets) in &nginx_proxy_targets {
        for target in proxy_targets {
            // Extract host:port from proxy target
            let target_host = if target.starts_with("http://") {
                target.trim_start_matches("http://")
            } else if target.starts_with("https://") {
                target.trim_start_matches("https://")
            } else {
                target.as_str()
            };

            // Try to match to container name or service
            let target_container = target_host.split(':').next().unwrap_or(target_host);

            // Check if target matches any container
            if containers
                .iter()
                .any(|c| c.name.contains(target_container) || target_container.contains(&c.name))
            {
                all_connections.push(DetectedConnection {
                    source: format!("nginx_{}", site_name),
                    target: containers
                        .iter()
                        .find(|c| {
                            c.name.contains(target_container) || target_container.contains(&c.name)
                        })
                        .map(|c| c.name.clone())
                        .unwrap_or_else(|| target_container.to_string()),
                    port: target_host.split(':').nth(1).and_then(|p| p.parse().ok()),
                    protocol: "http".to_string(),
                    connection_type: "http".to_string(),
                    label: Some(format!("proxy → {}", target_host)),
                });
            }
        }
    }

    // 3c. Parse docker-compose depends_on relationships
    for container in &containers {
        if !container.depends_on.is_empty() {
            for dep_service in &container.depends_on {
                // Find the container matching this service name
                if let Some(dep_container) = containers.iter().find(|c| {
                    // Match by exact service name or container name containing service
                    c.name == *dep_service || c.name.contains(dep_service)
                }) {
                    all_connections.push(DetectedConnection {
                        source: container.name.clone(),
                        target: dep_container.name.clone(),
                        port: None,
                        protocol: "dependency".to_string(),
                        connection_type: "depends_on".to_string(),
                        label: Some("depends_on".to_string()),
                    });
                }
            }
        }
    }

    // 5. Convert detected connections to edges
    for conn in all_connections {
        let label = if let Some(ref l) = conn.label {
            l.clone()
        } else if let Some(port) = conn.port {
            format!(":{}", port)
        } else {
            conn.protocol.clone()
        };

        edges.push(NetworkEdge {
            source: conn.source,
            target: conn.target,
            edge_type: conn.connection_type,
            label: Some(label),
            protocol: Some(conn.protocol),
            metadata: HashMap::new(),
        });
    }

    // 6. Add published ports as firewall→service edges
    // Services with published ports (0.0.0.0:*) are accessible from home network via firewall
    for container in &containers {
        let has_published_ports = container.ports.iter().any(|p| p.contains("0.0.0.0"));

        if has_published_ports {
            for port_str in &container.ports {
                if let Some((left, right)) = port_str.split_once("->") {
                    if left.starts_with("0.0.0.0:") {
                        let host_port = left.rsplit_once(':').map(|(_, port)| port).unwrap_or("");
                        let container_port =
                            right.split_once('/').map(|(port, _)| port).unwrap_or(right);

                        if !host_port.is_empty() {
                            edges.push(NetworkEdge {
                                source: "firewall".to_string(),
                                target: container.name.clone(),
                                edge_type: "port_mapping".to_string(),
                                label: Some(format!(":{}", host_port)),
                                protocol: Some("tcp".to_string()),
                                metadata: {
                                    let mut m = HashMap::new();
                                    m.insert("host_port".to_string(), host_port.to_string());
                                    m.insert(
                                        "container_port".to_string(),
                                        container_port.to_string(),
                                    );
                                    m
                                },
                            });
                        }
                    }
                }
            }
        }
    }

    // 7. Add system services (nginx, kaspad, etc.)
    #[cfg(target_os = "linux")]
    {
        use crate::core::system_service::SystemServiceManager;
        let sys_manager = SystemServiceManager::new(false);

        if let Ok(services) = sys_manager.list_services().await {
            let filtered = sys_manager.filter_relevant_services(services);

            for service in filtered {
                if service.name.contains("nginx")
                    || (!service.ports.is_empty() && service.name.contains("kaspa"))
                {
                    let mut metadata = HashMap::new();
                    metadata.insert(
                        "service_type".to_string(),
                        format!("{:?}", service.service_type),
                    );

                    // Add category for grouping
                    if !service.category.is_empty() {
                        metadata.insert("category".to_string(), service.category.clone());
                    }

                    // System services go in systemd layer
                    // Special case: nginx is a gateway, so put it in gateway layer
                    let layer = if service.name.contains("nginx") {
                        "gateway".to_string()
                    } else {
                        "systemd".to_string()
                    };

                    // Extract domains for nginx services
                    let mut service_domains = Vec::new();
                    if service.name.contains("nginx") {
                        for site in &nginx_sites {
                            service_domains.extend(site.server_names.clone());
                        }
                    }

                    // Security scan for exposed ports
                    let mut service_warnings = Vec::new();
                    let exposed_ports: Vec<(u16, String)> = service
                        .ports
                        .iter()
                        .filter_map(|p| {
                            if let Some(port_num) = p.split('/').next().and_then(|s| s.parse().ok())
                            {
                                Some((port_num, service.name.clone()))
                            } else {
                                None
                            }
                        })
                        .collect();

                    if !exposed_ports.is_empty() {
                        let warnings = security_scanner.scan_public_ports(&exposed_ports);
                        service_warnings.extend(
                            warnings
                                .iter()
                                .map(|w| format!("{}: {}", w.severity, w.description)),
                        );
                    }

                    nodes.push(NetworkNode {
                        id: format!("systemd_{}", service.name),
                        label: service.display_name.clone(),
                        node_type: "service".to_string(),
                        status: format!("{:?}", service.status),
                        ports: service.ports.clone(),
                        ip_address: None,
                        layer,
                        metadata,
                        warnings: service_warnings,
                        domains: service_domains,
                    });

                    // Add firewall→service edge for public ports
                    for port_str in &service.ports {
                        if let Some(port_num) = port_str.split('/').next() {
                            edges.push(NetworkEdge {
                                source: "firewall".to_string(),
                                target: format!("systemd_{}", service.name),
                                edge_type: "port_mapping".to_string(),
                                label: Some(format!(":{}", port_num)),
                                protocol: Some("tcp".to_string()),
                                metadata: HashMap::new(),
                            });
                        }
                    }
                }
            }
        }
    }

    // Create nodes for external services (IPs referenced in edges but not in nodes)
    let existing_node_ids: std::collections::HashSet<String> =
        nodes.iter().map(|n| n.id.clone()).collect();
    let mut external_ips_to_add = std::collections::HashSet::new();

    // Find all edge targets that don't have corresponding nodes
    for edge in &edges {
        if !existing_node_ids.contains(&edge.target) {
            // Check if it looks like an IP address
            if edge.target.parse::<std::net::IpAddr>().is_ok() {
                external_ips_to_add.insert(edge.target.clone());
            }
        }
        // Also check source
        if !existing_node_ids.contains(&edge.source) {
            if edge.source.parse::<std::net::IpAddr>().is_ok() {
                external_ips_to_add.insert(edge.source.clone());
            }
        }
    }

    // Create nodes for external IPs
    for external_ip in external_ips_to_add {
        let mut metadata = HashMap::new();
        metadata.insert("type".to_string(), "external_service".to_string());

        nodes.push(NetworkNode {
            id: external_ip.clone(),
            label: format!("External\n{}", external_ip),
            node_type: "external_service".to_string(),
            status: "unknown".to_string(),
            ports: vec![],
            ip_address: Some(external_ip.clone()),
            layer: "internet".to_string(),
            metadata,
            warnings: vec![],
            domains: vec![],
        });
    }

    let topology = NetworkTopology { nodes, edges };
    Ok(Json(ApiResponse::ok(topology)))
}
