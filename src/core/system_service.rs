/// System Service Management
///
/// Manages native system services (systemd, processes) alongside Docker containers
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ServiceType {
    Systemd,
    Process,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ServiceStatus {
    Running,
    Stopped,
    Failed,
    Inactive,
    Activating,
    Deactivating,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemServiceInfo {
    pub name: String,
    pub display_name: String,
    pub service_type: ServiceType,
    pub status: ServiceStatus,
    pub category: String,
    pub description: String,
    pub pid: Option<u32>,
    pub memory: Option<u64>,
    pub cpu: Option<f64>,
    pub uptime: Option<Duration>,
    pub network_rx: Option<u64>,
    pub network_tx: Option<u64>,
    pub auto_restart: bool,
    pub enabled: bool,
    pub config_files: Vec<PathBuf>,
    pub log_paths: Vec<PathBuf>,
    pub dependencies: Vec<String>,
    pub ports: Vec<String>,   // Listening ports (e.g., ["80/tcp", "443/tcp"])
    pub project_name: String, // "System Services"
    pub loaded: bool,
    pub active: bool,
    pub sub_state: String,
    // Parsed metrics from logs (similar to Docker services)
    pub status_text: Option<String>,
    pub primary_metric: Option<String>,
    pub secondary_metric: Option<String>,
    pub is_healthy_metric: bool,
    // Flag indicating if metrics are available (has plugin)
    pub has_metrics: bool,
    // Detailed metrics from plugin system
    #[serde(skip_deserializing)]
    pub metrics: Vec<crate::core::metrics::fetchers::MetricValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemServiceMetrics {
    pub cpu_percent: f64,
    pub memory_usage: u64,
    pub memory_percent: f64,
    pub uptime_seconds: u64,
    pub restarts: u32,
}

pub struct SystemServiceManager {
    use_sudo: bool,
}

impl SystemServiceManager {
    pub fn new(use_sudo: bool) -> Self {
        Self { use_sudo }
    }

    /// List all system services
    pub async fn list_services(&self) -> Result<Vec<SystemServiceInfo>> {
        let output = Command::new("systemctl")
            .args(&[
                "list-units",
                "--type=service",
                "--all",
                "--no-pager",
                "--no-legend",
            ])
            .output()
            .context("Failed to list systemd services")?;

        if !output.status.success() {
            return Err(anyhow!("systemctl list-units failed"));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut services = Vec::new();

        for line in stdout.lines() {
            if let Some(mut service) = self.parse_service_line(line).await {
                // Enrich with detailed stats if service is running
                if service.status == ServiceStatus::Running {
                    if let Ok(details) = self.get_service_details(&service.name).await {
                        service.pid = details.pid;
                        service.memory = details.memory;
                        service.cpu = details.cpu;
                        service.uptime = details.uptime;
                        service.network_rx = details.network_rx;
                        service.network_tx = details.network_tx;
                        service.ports = details.ports;
                        service.status_text = details.status_text;
                        service.primary_metric = details.primary_metric;
                        service.secondary_metric = details.secondary_metric;
                        service.is_healthy_metric = details.is_healthy_metric;
                    }
                }
                services.push(service);
            }
        }

        Ok(services)
    }

    /// Get detailed information about a specific service
    pub async fn get_service_details(&self, name: &str) -> Result<SystemServiceInfo> {
        let output = Command::new("systemctl")
            .args(&["show", name, "--no-pager"])
            .output()
            .context("Failed to get service details")?;

        if !output.status.success() {
            return Err(anyhow!("systemctl show failed for {}", name));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        self.parse_service_details(name, &stdout).await
    }

    /// Start a service
    pub async fn start_service(&self, name: &str) -> Result<()> {
        let mut cmd = if self.use_sudo {
            let mut c = Command::new("sudo");
            c.arg("systemctl").arg("start").arg(name);
            c
        } else {
            let mut c = Command::new("systemctl");
            c.arg("start").arg(name);
            c
        };

        let output = cmd.output().context("Failed to start service")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to start {}: {}", name, stderr));
        }

        Ok(())
    }

    /// Stop a service
    pub async fn stop_service(&self, name: &str) -> Result<()> {
        let mut cmd = if self.use_sudo {
            let mut c = Command::new("sudo");
            c.arg("systemctl").arg("stop").arg(name);
            c
        } else {
            let mut c = Command::new("systemctl");
            c.arg("stop").arg(name);
            c
        };

        let output = cmd.output().context("Failed to stop service")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to stop {}: {}", name, stderr));
        }

        Ok(())
    }

    /// Restart a service
    pub async fn restart_service(&self, name: &str) -> Result<()> {
        let mut cmd = if self.use_sudo {
            let mut c = Command::new("sudo");
            c.arg("systemctl").arg("restart").arg(name);
            c
        } else {
            let mut c = Command::new("systemctl");
            c.arg("restart").arg(name);
            c
        };

        let output = cmd.output().context("Failed to restart service")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to restart {}: {}", name, stderr));
        }

        Ok(())
    }

    /// Get service logs
    pub async fn get_logs(&self, name: &str, lines: usize) -> Result<String> {
        let output = Command::new("journalctl")
            .args(&["-u", name, "-n", &lines.to_string(), "--no-pager"])
            .output()
            .context("Failed to get service logs")?;

        if !output.status.success() {
            return Err(anyhow!("journalctl failed for {}", name));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Get service metrics
    pub async fn get_metrics(&self, name: &str) -> Result<SystemServiceMetrics> {
        let details = self.get_service_details(name).await?;

        let mut metrics = SystemServiceMetrics {
            cpu_percent: details.cpu.unwrap_or(0.0),
            memory_usage: details.memory.unwrap_or(0),
            memory_percent: 0.0,
            uptime_seconds: details.uptime.map(|d| d.as_secs()).unwrap_or(0),
            restarts: 0,
        };

        // Get additional metrics from systemctl show
        let output = Command::new("systemctl")
            .args(&["show", name, "--no-pager"])
            .output()?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if let Some((key, value)) = line.split_once('=') {
                    match key {
                        "MemoryCurrent" => {
                            if let Ok(mem) = value.parse::<u64>() {
                                metrics.memory_usage = mem;
                            }
                        }
                        "NRestarts" => {
                            if let Ok(restarts) = value.parse::<u32>() {
                                metrics.restarts = restarts;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(metrics)
    }

    /// Check if service is enabled
    pub async fn is_enabled(&self, name: &str) -> Result<bool> {
        let output = Command::new("systemctl")
            .args(&["is-enabled", name])
            .output()
            .context("Failed to check if service is enabled")?;

        Ok(output.status.success())
    }

    /// Enable a service
    pub async fn enable_service(&self, name: &str) -> Result<()> {
        let mut cmd = if self.use_sudo {
            let mut c = Command::new("sudo");
            c.arg("systemctl").arg("enable").arg(name);
            c
        } else {
            let mut c = Command::new("systemctl");
            c.arg("enable").arg(name);
            c
        };

        let output = cmd.output().context("Failed to enable service")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to enable {}: {}", name, stderr));
        }

        Ok(())
    }

    /// Disable a service
    pub async fn disable_service(&self, name: &str) -> Result<()> {
        let mut cmd = if self.use_sudo {
            let mut c = Command::new("sudo");
            c.arg("systemctl").arg("disable").arg(name);
            c
        } else {
            let mut c = Command::new("systemctl");
            c.arg("disable").arg(name);
            c
        };

        let output = cmd.output().context("Failed to disable service")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to disable {}: {}", name, stderr));
        }

        Ok(())
    }

    /// Detect listening ports for a given PID
    fn get_listening_ports(&self, pid: u32) -> Vec<String> {
        // Use ss command to find listening sockets for this PID
        let output = Command::new("ss").args(&["-ltnp"]).output();

        let Ok(output) = output else {
            return Vec::new();
        };

        if !output.status.success() {
            return Vec::new();
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut ports = Vec::new();

        for line in stdout.lines() {
            // Skip header line
            if line.starts_with("State") || line.starts_with("Netid") {
                continue;
            }

            // Check if this line contains our PID
            if !line.contains(&format!("pid={}", pid)) {
                continue;
            }

            // Parse the line to extract the local address
            // Format: LISTEN 0 128 0.0.0.0:80 0.0.0.0:* users:(("nginx",pid=1234,fd=6))
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 4 {
                continue;
            }

            // The local address is typically in the 4th column (index 3)
            let local_addr = parts[3];

            // Extract port from address (format: "0.0.0.0:80" or "[::]:80")
            if let Some(port_str) = local_addr.split(':').last() {
                if let Ok(port) = port_str.parse::<u16>() {
                    // Format as "port/tcp" to match Docker format
                    let port_entry = format!("{}/tcp", port);
                    if !ports.contains(&port_entry) {
                        ports.push(port_entry);
                    }
                }
            }
        }

        // Sort ports numerically
        ports.sort_by_key(|p| {
            p.split('/')
                .next()
                .and_then(|s| s.parse::<u16>().ok())
                .unwrap_or(0)
        });

        ports
    }

    /// Get CPU usage percentage for a process
    fn get_process_cpu(&self, pid: u32) -> Option<f64> {
        // Read /proc/[pid]/stat for CPU times
        let stat_path = format!("/proc/{}/stat", pid);
        let stat_content = fs::read_to_string(&stat_path).ok()?;

        // Parse stat file: fields are space-separated, we need fields 14-17 (utime, stime, cutime, cstime)
        let parts: Vec<&str> = stat_content.split_whitespace().collect();
        if parts.len() < 17 {
            return None;
        }

        let utime: u64 = parts[13].parse().ok()?; // user time
        let stime: u64 = parts[14].parse().ok()?; // system time

        // Get system uptime to calculate CPU percentage
        let uptime_content = fs::read_to_string("/proc/uptime").ok()?;
        let system_uptime: f64 = uptime_content.split_whitespace().next()?.parse().ok()?;

        // Get process start time (field 22, in clock ticks since boot)
        let starttime: u64 = parts[21].parse().ok()?;

        // Clock ticks per second (usually 100)
        let clk_tck = 100.0; // sysconf(_SC_CLK_TCK)

        // Calculate process uptime in seconds
        let process_uptime = system_uptime - (starttime as f64 / clk_tck);

        if process_uptime <= 0.0 {
            return None;
        }

        // Total CPU time in seconds
        let total_cpu_time = (utime + stime) as f64 / clk_tck;

        // CPU percentage = (total_cpu_time / process_uptime) * 100
        Some((total_cpu_time / process_uptime) * 100.0)
    }

    /// Get process RSS memory (resident set size - actual physical RAM used)
    fn get_process_memory(&self, pid: u32) -> Option<u64> {
        let status_path = format!("/proc/{}/status", pid);
        let content = fs::read_to_string(&status_path).ok()?;

        for line in content.lines() {
            if line.starts_with("VmRSS:") {
                // Format: "VmRSS:    25921880 kB"
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(kb) = parts[1].parse::<u64>() {
                        return Some(kb * 1024); // Convert KB to bytes
                    }
                }
            }
        }
        None
    }

    /// Get process uptime in seconds
    fn get_process_uptime(&self, pid: u32) -> Option<Duration> {
        let stat_path = format!("/proc/{}/stat", pid);
        let stat_content = fs::read_to_string(&stat_path).ok()?;

        let parts: Vec<&str> = stat_content.split_whitespace().collect();
        if parts.len() < 22 {
            return None;
        }

        // Get process start time (field 22, in clock ticks since boot)
        let starttime: u64 = parts[21].parse().ok()?;

        // Get system uptime
        let uptime_content = fs::read_to_string("/proc/uptime").ok()?;
        let system_uptime: f64 = uptime_content.split_whitespace().next()?.parse().ok()?;

        // Clock ticks per second (usually 100)
        let clk_tck = 100.0;

        // Calculate process uptime in seconds
        let process_uptime = system_uptime - (starttime as f64 / clk_tck);

        if process_uptime < 0.0 {
            return None;
        }

        Some(Duration::from_secs_f64(process_uptime))
    }

    /// Get network statistics for a process by reading /proc/[pid]/net/dev
    fn get_process_network_stats(&self, pid: u32) -> Option<(u64, u64)> {
        let net_dev_path = format!("/proc/{}/net/dev", pid);
        let content = fs::read_to_string(&net_dev_path).ok()?;

        let mut total_rx = 0u64;
        let mut total_tx = 0u64;

        // Skip first 2 header lines
        for line in content.lines().skip(2) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 10 {
                continue;
            }

            // Skip loopback interface
            if parts[0].starts_with("lo:") {
                continue;
            }

            // Parse RX bytes (column 1) and TX bytes (column 9)
            if let Ok(rx) = parts[1].parse::<u64>() {
                total_rx += rx;
            }
            if let Ok(tx) = parts[9].parse::<u64>() {
                total_tx += tx;
            }
        }

        Some((total_rx, total_tx))
    }

    // Parse service line from systemctl list-units
    async fn parse_service_line(&self, line: &str) -> Option<SystemServiceInfo> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            return None;
        }

        // Find the service name (ends with .service or is a .service file)
        let mut name_idx = None;
        for (i, part) in parts.iter().enumerate() {
            if part.ends_with(".service") {
                name_idx = Some(i);
                break;
            }
        }

        // If no service name found, skip this line
        let name_idx = match name_idx {
            Some(idx) => idx,
            None => return None,
        };

        // Ensure we have enough parts after the service name (loaded, active, sub_state)
        if parts.len() < name_idx + 4 {
            return None;
        }

        let name = parts[name_idx].to_string();
        let loaded = parts[name_idx + 1] == "loaded" || parts[name_idx + 1] == "not-found";
        let active = parts[name_idx + 2] == "active";
        let sub_state = parts[name_idx + 3].to_string();

        // Determine status
        let status = match (active, sub_state.as_str()) {
            (true, "running") => ServiceStatus::Running,
            (true, "exited") => ServiceStatus::Stopped,
            (false, "dead") => ServiceStatus::Stopped,
            (false, "failed") => ServiceStatus::Failed,
            (true, "activating") => ServiceStatus::Activating,
            (true, "deactivating") => ServiceStatus::Deactivating,
            _ => ServiceStatus::Unknown,
        };

        Some(SystemServiceInfo {
            name: name.clone(),
            display_name: name.trim_end_matches(".service").to_string(),
            service_type: ServiceType::Systemd,
            status,
            category: "Uncategorized".to_string(),
            description: String::new(),
            pid: None,
            memory: None,
            cpu: None,
            uptime: None,
            network_rx: None,
            network_tx: None,
            auto_restart: false,
            enabled: false,
            config_files: Vec::new(),
            log_paths: Vec::new(),
            dependencies: Vec::new(),
            ports: Vec::new(), // Will be populated when detailed info is fetched
            project_name: "System Services".to_string(),
            loaded,
            active,
            sub_state,
            status_text: None,
            primary_metric: None,
            secondary_metric: None,
            is_healthy_metric: true,
            has_metrics: false,
            metrics: Vec::new(),
        })
    }

    // Parse detailed service information from systemctl show
    async fn parse_service_details(&self, name: &str, output: &str) -> Result<SystemServiceInfo> {
        let mut props: HashMap<String, String> = HashMap::new();

        for line in output.lines() {
            if let Some((key, value)) = line.split_once('=') {
                props.insert(key.to_string(), value.to_string());
            }
        }

        let active_state = props
            .get("ActiveState")
            .map(|s| s.as_str())
            .unwrap_or("unknown");
        let sub_state = props
            .get("SubState")
            .map(|s| s.as_str())
            .unwrap_or("unknown");
        let loaded = props
            .get("LoadState")
            .map(|s| s == "loaded")
            .unwrap_or(false);

        let status = match active_state {
            "active" if sub_state == "running" => ServiceStatus::Running,
            "active" if sub_state == "exited" => ServiceStatus::Stopped,
            "inactive" => ServiceStatus::Stopped,
            "failed" => ServiceStatus::Failed,
            "activating" => ServiceStatus::Activating,
            "deactivating" => ServiceStatus::Deactivating,
            _ => ServiceStatus::Unknown,
        };

        let pid = props.get("MainPID").and_then(|s| s.parse().ok());
        let enabled = props
            .get("UnitFileState")
            .map(|s| s == "enabled")
            .unwrap_or(false);
        let description = props.get("Description").cloned().unwrap_or_default();

        // Get CPU, memory, uptime, and ports from /proc if PID is available
        let memory = pid.and_then(|p| self.get_process_memory(p));
        let cpu = pid.and_then(|p| self.get_process_cpu(p));
        let uptime = pid.and_then(|p| self.get_process_uptime(p));
        let (network_rx, network_tx) = pid
            .and_then(|p| self.get_process_network_stats(p))
            .map(|(rx, tx)| (Some(rx), Some(tx)))
            .unwrap_or((None, None));
        let ports = pid
            .map(|p| self.get_listening_ports(p))
            .unwrap_or_else(Vec::new);

        // Parse logs to extract metrics (similar to Docker services)
        let (status_text, primary_metric, secondary_metric, is_healthy_metric) =
            if active_state == "active" {
                match self.get_logs(name, 30).await {
                    Ok(logs) => {
                        let service_base_name = name.trim_end_matches(".service");
                        let metrics =
                            crate::core::log_parser::parse_service_logs(service_base_name, &logs);
                        (
                            metrics.status_text,
                            metrics.primary_metric,
                            metrics.secondary_metric,
                            metrics.is_healthy,
                        )
                    }
                    Err(_) => (None, None, None, true),
                }
            } else {
                (None, None, None, true)
            };

        Ok(SystemServiceInfo {
            name: name.to_string(),
            display_name: name.trim_end_matches(".service").to_string(),
            service_type: ServiceType::Systemd,
            status,
            category: "Uncategorized".to_string(),
            description,
            pid,
            memory,
            cpu,
            uptime,
            network_rx,
            network_tx,
            auto_restart: props.get("Restart").map(|s| s != "no").unwrap_or(false),
            enabled,
            config_files: Vec::new(),
            log_paths: Vec::new(),
            dependencies: Vec::new(),
            ports,
            project_name: "System Services".to_string(),
            loaded,
            active: active_state == "active",
            sub_state: sub_state.to_string(),
            status_text,
            primary_metric,
            secondary_metric,
            is_healthy_metric,
            has_metrics: false,  // Will be set by handler if plugin exists
            metrics: Vec::new(), // Will be populated by handler
        })
    }

    /// Filter services by relevance (exclude system/core services)
    pub fn filter_relevant_services(
        &self,
        services: Vec<SystemServiceInfo>,
    ) -> Vec<SystemServiceInfo> {
        let excluded_prefixes = vec![
            "systemd-",
            "user@",
            "getty@",
            "dbus",
            "polkit",
            "rtkit",
            "colord",
            "cups",
            "avahi",
            "bluetooth",
            "udisks2",
            "upower",
            "wpa_supplicant",
            "NetworkManager",
            "ModemManager",
        ];

        let excluded_names = vec![
            "cron.service",
            "rsyslog.service",
            "snapd.service",
            "ssh.service",
            "packagekit.service",
            "fwupd.service",
        ];

        services
            .into_iter()
            .filter(|s| {
                // Check excluded prefixes
                !excluded_prefixes.iter().any(|prefix| s.name.starts_with(prefix)) &&
                // Check excluded exact names
                !excluded_names.contains(&s.name.as_str())
            })
            .collect()
    }
}
