/// API Request Handlers
/// Reuses core business logic from existing modules

use axum::{
    extract::{Path, Query},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use sysinfo::System;

use crate::core::{
    ConfigManager, DockerManager,
    wallet::WalletManager,
    storage,
    log_parser,
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

    fn error(msg: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(msg),
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
}

// WalletInfo is now imported from crate::core::wallet module

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
    level: Option<String>,  // Filter: ERROR, WARN, INFO, DEBUG, TRACE
    #[serde(default)]
    module: Option<String>, // Filter by module name
}

// ============================================================================
// Service Management Handlers
// ============================================================================

pub async fn get_services() -> Result<Json<ApiResponse<Vec<ServiceInfo>>>, StatusCode> {
    let docker = DockerManager::new().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let containers = docker.list_containers().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Process all containers in parallel for speed
    let tasks: Vec<_> = containers.into_iter().map(|c| {
        tokio::spawn(async move {
            let docker = DockerManager::new().await.ok()?;

            // Get stats for resource metrics
            let stats = docker.get_container_stats(&c.name).await.ok().flatten();

            let (cpu_percent, memory_mb, network_rx_mb, network_tx_mb, container_size_mb, volume_size_mb) = if let Some(s) = stats {
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
            let ports: Vec<PortMapping> = c.ports.iter().filter_map(|p| {
                if p.contains("->") {
                    if let Some((left, right)) = p.split_once("->") {
                        let host_port = left.rsplit_once(':').map(|(_, port)| port).unwrap_or("");
                        let container_port = right.split_once('/').map(|(port, _)| port).unwrap_or(right);

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
            }).collect();

            // Fetch last 30 lines of logs and parse metrics (fast - only for key services)
            let (status_text, primary_metric, secondary_metric, is_healthy_metric) = if c.status.contains("Up") {
                let logs = docker.get_logs(&c.name, Some(30)).await.unwrap_or_default();
                let metrics = log_parser::parse_service_logs(&c.name, &logs);
                (metrics.status_text, metrics.primary_metric, metrics.secondary_metric, metrics.is_healthy)
            } else {
                (None, None, None, true)
            };

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
            })
        })
    }).collect();

    // Wait for all parallel tasks
    let mut services = Vec::new();
    for task in tasks {
        if let Ok(Some(service)) = task.await {
            services.push(service);
        }
    }

    Ok(Json(ApiResponse::ok(services)))
}

pub async fn start_service(
    Path(name): Path<String>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    let docker = DockerManager::new().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    docker.start_service(&name).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ApiResponse::ok(format!("Service {} started", name))))
}

pub async fn stop_service(
    Path(name): Path<String>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    let docker = DockerManager::new().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    docker.stop_service(&name).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ApiResponse::ok(format!("Service {} stopped", name))))
}

pub async fn restart_service(
    Path(name): Path<String>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    let docker = DockerManager::new().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    docker.restart_service(&name).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ApiResponse::ok(format!("Service {} restarted", name))))
}

pub async fn get_logs(
    Path(name): Path<String>,
    Query(params): Query<LogsQuery>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    let docker = DockerManager::new().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let logs = docker.get_logs(&name, Some(params.tail)).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ApiResponse::ok(logs)))
}

pub async fn get_logs_parsed(
    Path(name): Path<String>,
    Query(params): Query<ParsedLogsQuery>,
) -> Result<Json<ApiResponse<Vec<ParsedLogLine>>>, StatusCode> {
    let docker = DockerManager::new().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let logs = docker.get_logs(&name, Some(params.tail)).await
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

pub async fn get_wallets() -> Result<Json<ApiResponse<Vec<crate::core::wallet::WalletInfo>>>, StatusCode> {
    let wallet_manager = WalletManager::new()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let wallets = wallet_manager.list_wallets().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ApiResponse::ok(wallets)))
}

pub async fn get_wallet_balance(
    Path(id): Path<usize>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    let wallet_manager = WalletManager::new()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let balance = wallet_manager.get_balance(id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ApiResponse::ok(format!("{:.2} KAS", balance))))
}

pub async fn get_wallet_detail(
    Path(id): Path<usize>,
) -> Result<Json<ApiResponse<Vec<crate::core::wallet::UtxoInfo>>>, StatusCode> {
    let wallet_manager = WalletManager::new()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let utxos = wallet_manager.get_utxos(id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ApiResponse::ok(utxos)))
}

// ============================================================================
// Storage Handlers
// ============================================================================

#[derive(Serialize)]
pub struct StorageInfo {
    pub system_disk_used_percent: f64,
    pub system_disk_total_gb: f64,
    pub system_disk_used_gb: f64,
    pub docker_images_gb: f64,
    pub docker_volumes_gb: f64,
    pub docker_containers_gb: f64,
    pub docker_build_cache_gb: f64,
    pub reclaimable_gb: f64,
}

pub async fn get_storage() -> Result<Json<ApiResponse<StorageInfo>>, StatusCode> {
    let analysis = storage::analyze_storage().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let info = StorageInfo {
        system_disk_used_percent: analysis.system_disk.use_percent,
        system_disk_total_gb: analysis.system_disk.total_bytes as f64 / 1024.0 / 1024.0 / 1024.0,
        system_disk_used_gb: analysis.system_disk.used_bytes as f64 / 1024.0 / 1024.0 / 1024.0,
        docker_images_gb: analysis.docker_images.total_bytes as f64 / 1024.0 / 1024.0 / 1024.0,
        docker_volumes_gb: analysis.docker_volumes.iter().map(|v| v.size_bytes).sum::<u64>() as f64 / 1024.0 / 1024.0 / 1024.0,
        docker_containers_gb: analysis.docker_containers.total_bytes as f64 / 1024.0 / 1024.0 / 1024.0,
        docker_build_cache_gb: analysis.docker_build_cache.total_bytes as f64 / 1024.0 / 1024.0 / 1024.0,
        reclaimable_gb: analysis.reclaimable_space as f64 / 1024.0 / 1024.0 / 1024.0,
    };

    Ok(Json(ApiResponse::ok(info)))
}

pub async fn get_storage_history() -> Result<Json<ApiResponse<Vec<storage::StorageMeasurement>>>, StatusCode> {
    let history = storage::StorageHistory::load()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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
        Ok(Json(ApiResponse::ok(format!("Prune completed: {}", stdout))))
    } else {
        Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

// ============================================================================
// Configuration Handlers
// ============================================================================

pub async fn get_config() -> Result<Json<ApiResponse<HashMap<String, String>>>, StatusCode> {
    let config_manager = ConfigManager::load_from_project()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let config = config_manager.to_map();

    Ok(Json(ApiResponse::ok(config)))
}

pub async fn get_system_info() -> Result<Json<ApiResponse<crate::app::SystemResources>>, StatusCode> {
    let system_resources = crate::app::App::collect_system_resources();
    Ok(Json(ApiResponse::ok(system_resources)))
}

#[derive(Serialize)]
pub struct RpcToken {
    pub index: usize,
    pub token: Option<String>,
}

pub async fn get_rpc_tokens() -> Result<Json<ApiResponse<Vec<RpcToken>>>, StatusCode> {
    let config = ConfigManager::load_from_project()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let tokens: Vec<RpcToken> = config.get_rpc_tokens()
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
    let config = ConfigManager::load_from_project()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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
    let docker = DockerManager::new().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get system metrics (simplified - you can expand this)
    let containers = docker.list_containers().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let metrics = MetricsInfo {
        system_cpu: 0.0, // TODO: Implement with sysinfo
        system_memory_percent: 0.0,
        system_disk_percent: 0.0,
        docker_containers_running: containers.len(),
        docker_images: 0, // TODO: Get from Docker
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
    let docker = DockerManager::new().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let containers = docker.list_containers().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let active_profiles = DockerManager::get_active_profiles_from_list(&containers);

    // Define all available profiles and their services
    let all_profiles = vec![
        ("kaspad", vec!["kaspad", "kaspa-miner"]),
        ("backend", vec!["execution-layer", "block-builder", "viaduct"]),
        ("frontend-w1", vec!["traefik", "rpc-provider-0", "kaswallet-0"]),
        ("frontend-w2", vec!["rpc-provider-1", "kaswallet-1"]),
        ("frontend-w3", vec!["rpc-provider-2", "kaswallet-2"]),
        ("frontend-w4", vec!["rpc-provider-3", "kaswallet-3"]),
        ("frontend-w5", vec!["rpc-provider-4", "kaswallet-4"]),
        ("kaswallets", vec!["kaswallet-0", "kaswallet-1", "kaswallet-2", "kaswallet-3", "kaswallet-4"]),
        ("rpc-providers", vec!["rpc-provider-0", "rpc-provider-1", "rpc-provider-2", "rpc-provider-3", "rpc-provider-4"]),
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
    let docker = DockerManager::new().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    docker.start_profile(&name).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ApiResponse::ok(format!("Profile {} started", name))))
}

pub async fn stop_profile(
    Path(name): Path<String>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    let docker = DockerManager::new().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    docker.stop_profile(&name).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ApiResponse::ok(format!("Profile {} stopped", name))))
}

// ============================================================================
// Transaction Monitoring Handlers
// ============================================================================

use crate::core::l2_monitor::{TransactionMonitor, TransactionInfo as L2TransactionInfo, Statistics};

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
    let monitor = TransactionMonitor::new().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let transactions = monitor.poll_new_transactions().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Convert and filter
    let mut converted: Vec<TransactionInfo> = transactions
        .into_iter()
        .map(|tx| tx.into())
        .collect();

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
    let monitor = TransactionMonitor::new().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let stats = monitor.get_statistics().await;

    Ok(Json(ApiResponse::ok(stats.into())))
}
