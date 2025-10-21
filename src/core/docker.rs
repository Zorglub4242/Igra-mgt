/// Docker and Docker Compose integration
///
/// Manages Docker containers, images, and docker-compose operations

use anyhow::{anyhow, Context, Result};
use bollard::Docker;
use bollard::container::{ListContainersOptions, StatsOptions};
use bollard::models::ContainerSummary;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::utils::{get_project_root, ContainerState};
use crate::core::log_parser::{parse_service_logs, ServiceMetrics};

#[derive(Debug, Clone)]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub image: String,
    pub status: String,
    pub state: ContainerState,
    pub health: Option<String>,
    pub created: i64,
    pub ports: Vec<String>,
    pub metrics: ServiceMetrics,
}

#[derive(Debug, Clone)]
pub struct ContainerStats {
    pub cpu_percent: f64,
    pub memory_usage: u64,
    pub memory_limit: u64,
    pub network_rx: u64,
    pub network_tx: u64,
    pub container_size: u64,    // Container filesystem size in bytes
    pub volume_size: u64,       // Total volume data size in bytes
}

/// Service configuration from docker-compose.yml
#[derive(Debug, Clone)]
pub struct ComposeServiceConfig {
    pub image: Option<String>,
    pub environment: HashMap<String, String>,
    pub volumes: Vec<String>,
    pub ports: Vec<String>,
    pub networks: Vec<String>,
    pub profiles: Vec<String>,
    pub restart: Option<String>,
    pub command: Option<String>,
    pub entrypoint: Option<String>,
    pub depends_on: Vec<String>,
}

/// Running container configuration from Docker inspect
#[derive(Debug, Clone)]
pub struct RunningServiceConfig {
    pub image: String,
    pub env_vars: Vec<(String, String)>,
    pub volumes: Vec<String>,
    pub ports: Vec<String>,
    pub networks: Vec<String>,
    pub restart_policy: String,
    pub command: Option<Vec<String>>,
    pub entrypoint: Option<Vec<String>>,
    pub status: String,
    pub uptime: String,
}

/// Comparison between YAML config and running state
#[derive(Debug, Clone)]
pub struct ServiceConfigComparison {
    pub service_name: String,
    pub yaml_config: ComposeServiceConfig,
    pub running_config: Option<RunningServiceConfig>,
    pub config_drift: Vec<String>,  // Human-readable drift descriptions
}

#[derive(Clone)]
pub struct DockerManager {
    docker: Docker,
    project_root: PathBuf,
    compose_file: PathBuf,
    network: String,
}

impl DockerManager {
    /// Create a new Docker manager (synchronous version for App initialization)
    pub fn new_sync() -> Result<Self> {
        let docker = Docker::connect_with_local_defaults()
            .context("Failed to connect to Docker daemon. Is Docker running?")?;

        let project_root = get_project_root()?;
        let compose_file = project_root.join("docker-compose.yml");

        if !compose_file.exists() {
            return Err(anyhow!(
                "docker-compose.yml not found at {}",
                compose_file.display()
            ));
        }

        // Read network from .env file
        let env_file = project_root.join(".env");
        let network = if env_file.exists() {
            std::fs::read_to_string(&env_file)?
                .lines()
                .find(|line| line.trim().starts_with("NETWORK="))
                .and_then(|line| line.split('=').nth(1))
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| "testnet".to_string())
        } else {
            "testnet".to_string()
        };

        Ok(Self {
            docker,
            project_root,
            compose_file,
            network,
        })
    }

    /// Create a new Docker manager (async wrapper for compatibility)
    pub async fn new() -> Result<Self> {
        Self::new_sync()
    }

    /// Get project root directory
    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    /// Get network name (testnet/mainnet)
    pub fn network(&self) -> &str {
        &self.network
    }

    /// List all IGRA Orchestra containers
    pub async fn list_containers(&self) -> Result<Vec<ContainerInfo>> {
        let mut filters = HashMap::new();
        filters.insert(
            "label".to_string(),
            vec![format!("com.docker.compose.project=igra-orchestra-{}", self.network)],
        );

        let options = Some(ListContainersOptions {
            all: true,
            size: true,  // Enable size information
            filters,
            ..Default::default()
        });

        let containers = self.docker.list_containers(options).await?;

        let mut container_infos: Vec<ContainerInfo> = containers
            .into_iter()
            .map(|c| self.container_summary_to_info(c))
            .collect();

        // Enrich with metrics by parsing logs (in parallel for performance)
        // Collect running container names
        let running_names: Vec<String> = container_infos
            .iter()
            .filter(|c| c.state == ContainerState::Running)
            .map(|c| c.name.clone())
            .collect();

        // Fetch logs in parallel (much faster than sequential)
        use futures::future::join_all;
        let log_futures = running_names.iter().map(|name| {
            let name = name.clone();
            async move {
                // Fetch last 20 lines - enough for parsing, faster than 50
                self.get_logs(&name, Some(20)).await.ok().map(|logs| (name.clone(), logs))
            }
        });

        let metrics_results = join_all(log_futures).await;

        // Parse logs and apply metrics to containers
        for result in metrics_results.into_iter().flatten() {
            let (name, logs) = result;
            let mut metrics = parse_service_logs(&name, &logs);

            // For execution-layer, also fetch Reth Prometheus metrics
            if name == "execution-layer" {
                if let Ok(reth_metrics) = crate::core::reth_metrics::fetch_reth_metrics().await {
                    // Enhance metrics with Reth-specific data
                    if let Some(blocks) = reth_metrics.blocks_processed {
                        metrics.primary_metric = Some(format!("Block #{}", blocks));
                    }
                    if let Some(peers) = reth_metrics.peers_connected {
                        metrics.secondary_metric = Some(format!("{} peers", peers));
                    }
                    if reth_metrics.blocks_processed.is_some() {
                        metrics.status_text = Some("Synced".to_string());
                        metrics.is_healthy = true;
                    }
                }
            }

            if let Some(container) = container_infos.iter_mut().find(|c| c.name == name) {
                container.metrics = metrics;
            }
        }

        Ok(container_infos)
    }

    /// Get container info by name
    pub async fn get_container(&self, name: &str) -> Result<Option<ContainerInfo>> {
        let containers = self.list_containers().await?;
        Ok(containers.into_iter().find(|c| c.name == name))
    }

    /// Get container stats
    pub async fn get_container_stats(&self, name: &str) -> Result<Option<ContainerStats>> {
        let container_id = match self.get_container(name).await? {
            Some(info) => info.id,
            None => return Ok(None),
        };

        let mut stats_stream = self.docker.stats(
            &container_id,
            Some(StatsOptions {
                stream: false,
                one_shot: true,
            }),
        );

        use futures::StreamExt;
        if let Some(Ok(stats)) = stats_stream.next().await {
            let cpu_delta = stats.cpu_stats.cpu_usage.total_usage
                - stats.precpu_stats.cpu_usage.total_usage;
            let system_delta = stats.cpu_stats.system_cpu_usage.unwrap_or(0)
                - stats.precpu_stats.system_cpu_usage.unwrap_or(0);
            let num_cpus = stats.cpu_stats.online_cpus.unwrap_or(1) as u64;

            let cpu_percent = if system_delta > 0 {
                (cpu_delta as f64 / system_delta as f64) * num_cpus as f64 * 100.0
            } else {
                0.0
            };

            let memory_usage = stats.memory_stats.usage.unwrap_or(0);
            let memory_limit = stats.memory_stats.limit.unwrap_or(0);

            let (network_rx, network_tx) = stats
                .networks
                .as_ref()
                .and_then(|networks| networks.get("eth0"))
                .map(|net| (net.rx_bytes, net.tx_bytes))
                .unwrap_or((0, 0));

            // Get container virtual size (image + container layers)
            let container_size = if let Ok(output) = tokio::process::Command::new("docker")
                .args(&["ps", "--size", "--filter", &format!("id={}", container_id), "--format", "{{.Size}}"])
                .output()
                .await
            {
                let size_str = String::from_utf8_lossy(&output.stdout);
                // Format is like "408MB (virtual 558MB)" or "0B (virtual 923MB)"
                // We want the virtual size (total image+container)
                if let Some(virtual_part) = size_str.trim().split(" (virtual ").nth(1) {
                    let virtual_str = virtual_part.trim_end_matches(')');
                    Self::parse_size_string(virtual_str)
                } else {
                    0
                }
            } else {
                0
            };

            // Get actual volume sizes by inspecting mounts and querying docker system df
            let volume_size = self.get_container_volume_size(&container_id).await.unwrap_or(0);

            Ok(Some(ContainerStats {
                cpu_percent,
                memory_usage,
                memory_limit,
                network_rx,
                network_tx,
                container_size,
                volume_size,
            }))
        } else {
            Ok(None)
        }
    }

    /// Parse Docker size string to bytes (e.g. "408MB", "6.15kB", "1.5GB")
    fn parse_size_string(size_str: &str) -> u64 {
        let size_str = size_str.trim();

        // Extract number and unit
        let mut num_str = String::new();
        let mut unit_str = String::new();

        for ch in size_str.chars() {
            if ch.is_numeric() || ch == '.' {
                num_str.push(ch);
            } else if ch.is_alphabetic() {
                unit_str.push(ch);
            }
        }

        let num: f64 = num_str.parse().unwrap_or(0.0);
        let unit = unit_str.to_uppercase();

        let multiplier: u64 = match unit.as_str() {
            "B" => 1,
            "KB" => 1024,
            "MB" => 1024 * 1024,
            "GB" => 1024 * 1024 * 1024,
            "TB" => 1024 * 1024 * 1024 * 1024,
            _ => 1,
        };

        (num * multiplier as f64) as u64
    }

    /// Get total volume size for a container by inspecting its mounts
    async fn get_container_volume_size(&self, container_id: &str) -> Result<u64> {
        // Get container's volume mounts
        let inspect = self.docker.inspect_container(container_id, None).await?;

        let mut total_volume_size: u64 = 0;

        // Get all volumes from mounts
        if let Some(mounts) = inspect.mounts {
            // Build a map of volume names to sizes using docker system df
            let volume_sizes = self.get_all_volume_sizes().await?;

            for mount in mounts {
                if let Some(volume_name) = mount.name {
                    if let Some(&size) = volume_sizes.get(&volume_name) {
                        total_volume_size += size;
                    }
                }
            }
        }

        Ok(total_volume_size)
    }

    /// Get all volume sizes using docker system df -v
    async fn get_all_volume_sizes(&self) -> Result<HashMap<String, u64>> {
        let output = tokio::process::Command::new("docker")
            .args(&["system", "df", "-v"])
            .output()
            .await?;

        let output_str = String::from_utf8_lossy(&output.stdout);
        let mut volume_sizes = HashMap::new();

        // Parse output like:
        // VOLUME NAME                                    LINKS     SIZE
        // igra-orchestra-testnet_execution_layer_data    1         7.066GB
        let mut in_volumes_section = false;
        for line in output_str.lines() {
            if line.contains("VOLUME NAME") {
                in_volumes_section = true;
                continue;
            }

            if in_volumes_section {
                // Stop at next section
                if line.is_empty() || line.starts_with("Build cache") {
                    break;
                }

                // Parse volume line
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    let volume_name = parts[0].to_string();
                    let size_str = parts[2];
                    let size = Self::parse_size_string(size_str);
                    volume_sizes.insert(volume_name, size);
                }
            }
        }

        Ok(volume_sizes)
    }

    /// Execute docker-compose command
    pub async fn compose_command(&self, args: &[&str]) -> Result<String> {
        let mut cmd = Command::new("docker");
        cmd.arg("compose")
            .args(args)
            .current_dir(&self.project_root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let output = cmd.output()
            .context("Failed to execute docker compose command")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Docker compose command failed: {}", stderr));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Start services with a specific profile
    pub async fn start_profile(&self, profile: &str) -> Result<()> {
        self.compose_command(&["--profile", profile, "up", "-d"])
            .await?;
        Ok(())
    }

    /// Stop services from a specific profile
    pub async fn stop_profile(&self, profile: &str) -> Result<()> {
        self.compose_command(&["--profile", profile, "stop"])
            .await?;
        Ok(())
    }

    /// Stop all services
    pub async fn stop_all(&self) -> Result<()> {
        self.compose_command(&["down"]).await?;
        Ok(())
    }

    /// Stop specific service
    pub async fn stop_service(&self, service: &str) -> Result<()> {
        self.compose_command(&["stop", service]).await?;
        Ok(())
    }

    /// Start specific service
    pub async fn start_service(&self, service: &str) -> Result<()> {
        self.compose_command(&["start", service]).await?;
        Ok(())
    }

    /// Restart specific service
    pub async fn restart_service(&self, service: &str) -> Result<()> {
        self.compose_command(&["restart", service]).await?;
        Ok(())
    }

    /// Get logs for a service
    pub async fn get_logs(&self, service: &str, tail: Option<usize>) -> Result<String> {
        let mut args = vec!["logs"];
        let tail_str;
        if let Some(n) = tail {
            tail_str = n.to_string();
            args.push("--tail");
            args.push(&tail_str);
        }
        args.push(service);

        self.compose_command(&args).await
    }

    /// Get logs since a specific timestamp (RFC3339 format or relative time like "1s")
    pub async fn get_logs_since(&self, service: &str, since: &str) -> Result<String> {
        let args = vec!["logs", "--since", since, service];
        self.compose_command(&args).await
    }

    /// Stream logs for a service (returns async stream)
    pub async fn follow_logs(&self, service: &str) -> Result<tokio::process::Child> {
        let child = tokio::process::Command::new("docker")
            .arg("compose")
            .arg("logs")
            .arg("-f")
            .arg(service)
            .current_dir(&self.project_root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn docker compose logs")?;

        Ok(child)
    }

    /// Pull latest images
    pub async fn pull_images(&self) -> Result<()> {
        self.compose_command(&["pull"]).await?;
        Ok(())
    }

    /// Get active profiles from container list (synchronous, no Docker API calls)
    pub fn get_active_profiles_from_list(containers: &[ContainerInfo]) -> Vec<String> {
        let mut profiles = Vec::new();

        // Determine profiles based on running services
        let has_kaspad = containers.iter().any(|c| c.name == "kaspad" && c.state.is_running());
        let has_backend = containers.iter().any(|c| {
            ["execution-layer", "block-builder", "viaduct"].contains(&c.name.as_str())
                && c.state.is_running()
        });

        let rpc_count = containers
            .iter()
            .filter(|c| c.name.starts_with("rpc-provider-") && c.state.is_running())
            .count();

        if has_kaspad {
            profiles.push("kaspad".to_string());
        }
        if has_backend {
            profiles.push("backend".to_string());
        }
        if rpc_count > 0 {
            let profile = match rpc_count {
                1 => "frontend-w1",
                2 => "frontend-w2",
                3 => "frontend-w3",
                4 => "frontend-w4",
                5 => "frontend-w5",
                _ => "frontend-w5",
            };
            profiles.push(profile.to_string());
        }

        profiles
    }

    /// Get current profile(s) running (async version that fetches containers)
    pub async fn get_active_profiles(&self) -> Result<Vec<String>> {
        let containers = self.list_containers().await?;
        Ok(Self::get_active_profiles_from_list(&containers))
    }

    /// Check if Docker daemon is accessible
    pub async fn check_docker(&self) -> Result<bool> {
        match self.docker.ping().await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Convert ContainerSummary to ContainerInfo
    fn container_summary_to_info(&self, summary: ContainerSummary) -> ContainerInfo {
        let name = summary
            .names
            .as_ref()
            .and_then(|names| names.first())
            .map(|n| n.trim_start_matches('/').to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let status = summary.status.clone().unwrap_or_else(|| "unknown".to_string());
        let state = summary
            .state
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("unknown")
            .into();

        let health = summary
            .status
            .as_ref()
            .and_then(|s| {
                // Check unhealthy BEFORE healthy (unhealthy contains "healthy" as substring)
                if s.contains("unhealthy") {
                    Some("unhealthy".to_string())
                } else if s.contains("starting") {
                    Some("starting".to_string())
                } else if s.contains("healthy") {
                    Some("healthy".to_string())
                } else {
                    None
                }
            });

        let ports = summary
            .ports
            .as_ref()
            .map(|ports| {
                ports
                    .iter()
                    .filter_map(|p| {
                        p.public_port.map(|pub_port| {
                            format!(
                                "{}:{}->{}",
                                p.ip.as_deref().unwrap_or("0.0.0.0"),
                                pub_port,
                                p.private_port
                            )
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        ContainerInfo {
            id: summary.id.unwrap_or_default(),
            name,
            image: summary.image.unwrap_or_else(|| "unknown".to_string()),
            status,
            state,
            health,
            created: summary.created.unwrap_or(0),
            ports,
            metrics: ServiceMetrics::default(),
        }
    }

    /// Parse docker-compose.yml and extract service configurations
    pub fn parse_compose_file(&self) -> Result<HashMap<String, ComposeServiceConfig>> {
        use serde_yaml::Value;

        let compose_content = std::fs::read_to_string(&self.compose_file)
            .context("Failed to read docker-compose.yml")?;

        let yaml: Value = serde_yaml::from_str(&compose_content)
            .context("Failed to parse docker-compose.yml")?;

        let mut services = HashMap::new();

        // Extract services section
        if let Some(services_map) = yaml.get("services").and_then(|s| s.as_mapping()) {
            for (service_name, service_config) in services_map {
                let name = service_name.as_str().unwrap_or("unknown").to_string();

                let image = service_config.get("image")
                    .and_then(|i| i.as_str())
                    .map(|s| s.to_string());

                // Parse environment variables
                let mut environment = HashMap::new();
                if let Some(env) = service_config.get("environment") {
                    if let Some(env_map) = env.as_mapping() {
                        for (k, v) in env_map {
                            if let (Some(key), Some(val)) = (k.as_str(), v.as_str()) {
                                environment.insert(key.to_string(), val.to_string());
                            }
                        }
                    } else if let Some(env_seq) = env.as_sequence() {
                        for item in env_seq {
                            if let Some(s) = item.as_str() {
                                if let Some((k, v)) = s.split_once('=') {
                                    environment.insert(k.to_string(), v.to_string());
                                }
                            }
                        }
                    }
                }

                // Parse volumes
                let volumes = service_config.get("volumes")
                    .and_then(|v| v.as_sequence())
                    .map(|seq| {
                        seq.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();

                // Parse ports
                let ports = service_config.get("ports")
                    .and_then(|p| p.as_sequence())
                    .map(|seq| {
                        seq.iter()
                            .filter_map(|p| {
                                if let Some(s) = p.as_str() {
                                    Some(s.to_string())
                                } else if let Some(i) = p.as_i64() {
                                    Some(i.to_string())
                                } else {
                                    None
                                }
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                // Parse networks
                let networks = service_config.get("networks")
                    .and_then(|n| n.as_sequence())
                    .map(|seq| {
                        seq.iter()
                            .filter_map(|n| n.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();

                // Parse profiles
                let profiles = service_config.get("profiles")
                    .and_then(|p| p.as_sequence())
                    .map(|seq| {
                        seq.iter()
                            .filter_map(|p| p.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();

                // Parse restart policy
                let restart = service_config.get("restart")
                    .and_then(|r| r.as_str())
                    .map(|s| s.to_string());

                // Parse command
                let command = service_config.get("command")
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string());

                // Parse entrypoint
                let entrypoint = service_config.get("entrypoint")
                    .and_then(|e| e.as_str())
                    .map(|s| s.to_string());

                // Parse depends_on
                let depends_on = service_config.get("depends_on")
                    .and_then(|d| d.as_sequence())
                    .map(|seq| {
                        seq.iter()
                            .filter_map(|d| d.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();

                services.insert(name.clone(), ComposeServiceConfig {
                    image,
                    environment,
                    volumes,
                    ports,
                    networks,
                    profiles,
                    restart,
                    command,
                    entrypoint,
                    depends_on,
                });
            }
        }

        Ok(services)
    }

    /// Get service configuration comparison (YAML + Running state)
    pub async fn get_service_config_comparison(&self, service_name: &str) -> Result<ServiceConfigComparison> {
        // 1. Parse YAML config
        let compose_configs = self.parse_compose_file()?;
        let yaml_config = compose_configs.get(service_name)
            .ok_or_else(|| anyhow!("Service '{}' not found in docker-compose.yml", service_name))?
            .clone();

        // 2. Try to inspect running container
        let running_config = match self.docker.inspect_container(service_name, None).await {
            Ok(inspect) => {
                // Extract image
                let image = inspect.config
                    .as_ref()
                    .and_then(|c| c.image.as_ref())
                    .map(|s| s.clone())
                    .unwrap_or_default();

                // Extract and filter environment variables
                let env_vars = inspect.config
                    .as_ref()
                    .and_then(|c| c.env.as_ref())
                    .map(|env| Self::filter_sensitive_env(env.clone()))
                    .unwrap_or_default();

                // Extract volumes
                let volumes = inspect.mounts
                    .as_ref()
                    .map(|mounts| {
                        mounts.iter()
                            .filter_map(|m| {
                                if let (Some(src), Some(dst)) = (&m.source, &m.destination) {
                                    Some(format!("{} -> {}", src, dst))
                                } else {
                                    None
                                }
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                // Extract ports
                let ports = inspect.network_settings
                    .as_ref()
                    .and_then(|ns| ns.ports.as_ref())
                    .map(|ports_map| {
                        ports_map.iter()
                            .filter_map(|(container_port, host_bindings)| {
                                if let Some(bindings) = host_bindings {
                                    bindings.iter()
                                        .filter_map(|binding| {
                                            if let (Some(ip), Some(port)) = (&binding.host_ip, &binding.host_port) {
                                                Some(format!("{}:{} -> {}", ip, port, container_port))
                                            } else {
                                                None
                                            }
                                        })
                                        .next()
                                } else {
                                    None
                                }
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                // Extract networks
                let networks = inspect.network_settings
                    .as_ref()
                    .and_then(|ns| ns.networks.as_ref())
                    .map(|nets| nets.keys().cloned().collect())
                    .unwrap_or_default();

                // Extract restart policy
                let restart_policy = inspect.host_config
                    .as_ref()
                    .and_then(|hc| hc.restart_policy.as_ref())
                    .and_then(|rp| rp.name.as_ref())
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| "no".to_string());

                // Extract command
                let command = inspect.config
                    .as_ref()
                    .and_then(|c| c.cmd.clone());

                // Extract entrypoint
                let entrypoint = inspect.config
                    .as_ref()
                    .and_then(|c| c.entrypoint.clone());

                // Get status
                let status = inspect.state
                    .as_ref()
                    .and_then(|s| s.status.as_ref())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "unknown".to_string());

                // Calculate uptime
                let uptime = if let Some(state) = inspect.state.as_ref() {
                    if let Some(started_at) = state.started_at.as_ref() {
                        // Parse and calculate uptime
                        "TODO".to_string() // We'll implement this properly later
                    } else {
                        "N/A".to_string()
                    }
                } else {
                    "N/A".to_string()
                };

                Some(RunningServiceConfig {
                    image,
                    env_vars,
                    volumes,
                    ports,
                    networks,
                    restart_policy,
                    command,
                    entrypoint,
                    status,
                    uptime,
                })
            }
            Err(_) => None, // Container not running
        };

        // 3. Detect drift
        let config_drift = Self::detect_config_drift(&yaml_config, &running_config);

        Ok(ServiceConfigComparison {
            service_name: service_name.to_string(),
            yaml_config,
            running_config,
            config_drift,
        })
    }

    /// Filter sensitive environment variables
    fn filter_sensitive_env(env: Vec<String>) -> Vec<(String, String)> {
        env.into_iter()
            .filter_map(|e| {
                let parts: Vec<&str> = e.splitn(2, '=').collect();
                if parts.len() == 2 {
                    let key = parts[0];
                    let value = parts[1];
                    // Filter out sensitive keys
                    if key.contains("PASSWORD") || key.contains("SECRET")
                        || key.contains("KEY") || key.contains("TOKEN")
                        || key.contains("API_KEY") {
                        Some((key.to_string(), "***HIDDEN***".to_string()))
                    } else {
                        Some((key.to_string(), value.to_string()))
                    }
                } else {
                    None
                }
            })
            .collect()
    }

    /// Detect configuration drift between YAML and running state
    fn detect_config_drift(yaml: &ComposeServiceConfig, running: &Option<RunningServiceConfig>) -> Vec<String> {
        let mut drift = Vec::new();

        if let Some(running) = running {
            // Compare image
            if let Some(yaml_image) = &yaml.image {
                if yaml_image != &running.image {
                    drift.push(format!("Image: YAML='{}' ≠ Running='{}'", yaml_image, running.image));
                }
            }

            // Note: We won't compare all fields exhaustively for now
            // Just the most important ones
        } else {
            drift.push("Container is not running".to_string());
        }

        drift
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_docker_manager_creation() {
        // This test requires Docker to be running
        if let Ok(manager) = DockerManager::new().await {
            assert!(manager.project_root().exists());
            assert!(manager.compose_file.exists());
        }
    }
}
