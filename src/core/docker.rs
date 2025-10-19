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

            Ok(Some(ContainerStats {
                cpu_percent,
                memory_usage,
                memory_limit,
                network_rx,
                network_tx,
            }))
        } else {
            Ok(None)
        }
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
                if s.contains("healthy") {
                    Some("healthy".to_string())
                } else if s.contains("unhealthy") {
                    Some("unhealthy".to_string())
                } else if s.contains("starting") {
                    Some("starting".to_string())
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
