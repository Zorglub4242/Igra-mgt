use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

/// Main storage analysis result
#[derive(Debug, Clone, Serialize)]
pub struct StorageAnalysis {
    pub system_disk: DiskUsage,
    pub docker_images: DockerStorageInfo,
    pub docker_volumes: Vec<VolumeUsage>,
    pub docker_containers: DockerStorageInfo,
    pub docker_build_cache: DockerStorageInfo,
    pub container_logs: Vec<ContainerLogInfo>,
    pub reclaimable_space: u64,
    pub growth_rate: Option<GrowthRate>,
}

/// System disk usage information
#[derive(Debug, Clone, Serialize)]
pub struct DiskUsage {
    pub filesystem: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub use_percent: f64,
    pub mount_point: String,
}

/// Docker storage category info
#[derive(Debug, Clone, Serialize)]
pub struct DockerStorageInfo {
    pub total_bytes: u64,
    pub reclaimable_bytes: u64,
    pub active_count: usize,
    pub total_count: usize,
}

/// Individual Docker volume usage
#[derive(Debug, Clone, Serialize)]
pub struct VolumeUsage {
    pub name: String,
    pub size_bytes: u64,
    pub mount_point: String,
    pub in_use: bool,
    pub critical: bool, // Mark critical volumes like viaduct_data
}

/// Docker container log file information
#[derive(Debug, Clone, Serialize)]
pub struct ContainerLogInfo {
    pub container_id: String,
    pub container_name: String,
    pub log_size_bytes: u64,
    pub log_path: String,
}

/// Growth rate analysis
#[derive(Debug, Clone, Serialize)]
pub struct GrowthRate {
    pub bytes_per_day: f64,
    pub days_to_full: Option<u64>,
    pub trend: GrowthTrend,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum GrowthTrend {
    Growing,
    Stable,
    Declining,
}

/// Storage measurement for history tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageMeasurement {
    pub timestamp: DateTime<Utc>,
    pub total_used_bytes: u64,
    pub docker_volumes_bytes: u64,
    pub docker_images_bytes: u64,
}

/// Docker log rotation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRotationConfig {
    pub global: LogRotationSettings,
    pub overrides: std::collections::HashMap<String, LogRotationSettings>,
}

/// Log rotation settings for global or per-container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRotationSettings {
    pub driver: String,
    pub max_size: String,
    pub max_file: String,
}

/// Storage history file format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageHistory {
    pub measurements: Vec<StorageMeasurement>,
}

impl StorageHistory {
    pub fn new() -> Self {
        Self {
            measurements: Vec::new(),
        }
    }

    /// Load from file or create new
    pub fn load() -> Result<Self> {
        let path = Self::history_file_path()?;
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            Ok(serde_json::from_str(&content)?)
        } else {
            Ok(Self::new())
        }
    }

    /// Save to file
    pub fn save(&self) -> Result<()> {
        let path = Self::history_file_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    /// Add a new measurement
    pub fn add_measurement(&mut self, measurement: StorageMeasurement) {
        self.measurements.push(measurement);

        // Keep only last 90 days
        let cutoff = Utc::now() - chrono::Duration::days(90);
        self.measurements.retain(|m| m.timestamp > cutoff);
    }

    /// Get history file path
    fn history_file_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .context("Failed to get config directory")?
            .join("igra-cli");
        Ok(config_dir.join("storage_history.json"))
    }

    /// Get measurements filtered by number of days
    pub fn get_last_n_days(&self, days: u32) -> Vec<&StorageMeasurement> {
        let cutoff = Utc::now() - chrono::Duration::days(days as i64);
        self.measurements
            .iter()
            .filter(|m| m.timestamp > cutoff)
            .collect()
    }

    /// Downsample measurements to specified interval (in hours)
    /// Keeps the first measurement in each time bucket
    pub fn downsample_to_interval(&mut self, interval_hours: i64) {
        if self.measurements.is_empty() {
            return;
        }

        let mut downsampled = Vec::new();
        let mut current_bucket: Option<i64> = None;

        for measurement in &self.measurements {
            let timestamp_hours = measurement.timestamp.timestamp() / 3600;
            let bucket = timestamp_hours / interval_hours;

            if current_bucket != Some(bucket) {
                // New bucket - keep this measurement
                downsampled.push(measurement.clone());
                current_bucket = Some(bucket);
            }
            // else: same bucket, skip this measurement
        }

        self.measurements = downsampled;
    }

    /// Check if downsampling is needed (more than expected measurements for timespan)
    pub fn needs_downsampling(&self, interval_hours: i64) -> bool {
        if self.measurements.len() < 2 {
            return false;
        }

        let first = &self.measurements[0];
        let last = &self.measurements[self.measurements.len() - 1];
        let duration_hours = (last.timestamp - first.timestamp).num_hours();
        let expected_count = (duration_hours / interval_hours) + 1;

        // If we have more than 2x expected measurements, downsample
        self.measurements.len() > (expected_count * 2) as usize
    }
}

/// Analyze current storage usage
pub async fn analyze_storage() -> Result<StorageAnalysis> {
    let system_disk = get_system_disk_usage()?;
    let docker_summary = get_docker_system_df()?;
    let volumes = get_docker_volumes_usage()?;
    let container_logs = get_container_log_sizes().await?;

    let reclaimable = docker_summary.images_reclaimable
        + docker_summary.build_cache_total
        + docker_summary.volumes_reclaimable;

    // Load history and calculate growth rate
    let history = StorageHistory::load().unwrap_or_else(|_| StorageHistory::new());
    let growth_rate = calculate_growth_rate(&history, &system_disk);

    Ok(StorageAnalysis {
        system_disk,
        docker_images: DockerStorageInfo {
            total_bytes: docker_summary.images_total,
            reclaimable_bytes: docker_summary.images_reclaimable,
            active_count: docker_summary.images_active,
            total_count: docker_summary.images_count,
        },
        docker_volumes: volumes,
        docker_containers: DockerStorageInfo {
            total_bytes: docker_summary.containers_total,
            reclaimable_bytes: docker_summary.containers_reclaimable,
            active_count: docker_summary.containers_active,
            total_count: docker_summary.containers_count,
        },
        docker_build_cache: DockerStorageInfo {
            total_bytes: docker_summary.build_cache_total,
            reclaimable_bytes: docker_summary.build_cache_total, // 100% reclaimable
            active_count: 0,
            total_count: docker_summary.build_cache_count,
        },
        container_logs,
        reclaimable_space: reclaimable,
        growth_rate,
    })
}

#[derive(Debug)]
struct DockerSystemDfSummary {
    images_total: u64,
    images_reclaimable: u64,
    images_active: usize,
    images_count: usize,
    containers_total: u64,
    containers_reclaimable: u64,
    containers_active: usize,
    containers_count: usize,
    volumes_total: u64,
    volumes_reclaimable: u64,
    volumes_active: usize,
    volumes_count: usize,
    build_cache_total: u64,
    build_cache_count: usize,
}

/// Get system disk usage
fn get_system_disk_usage() -> Result<DiskUsage> {
    let output = Command::new("df")
        .arg("-B1") // Byte output
        .arg("/")
        .output()
        .context("Failed to run df command")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    if lines.len() < 2 {
        anyhow::bail!("Unexpected df output");
    }

    let parts: Vec<&str> = lines[1].split_whitespace().collect();
    if parts.len() < 6 {
        anyhow::bail!("Failed to parse df output");
    }

    let total = parts[1].parse::<u64>()?;
    let used = parts[2].parse::<u64>()?;
    let available = parts[3].parse::<u64>()?;
    let use_percent = parts[4].trim_end_matches('%').parse::<f64>()?;

    Ok(DiskUsage {
        filesystem: parts[0].to_string(),
        total_bytes: total,
        used_bytes: used,
        available_bytes: available,
        use_percent,
        mount_point: parts[5].to_string(),
    })
}

/// Parse docker system df output
fn get_docker_system_df() -> Result<DockerSystemDfSummary> {
    let output = Command::new("docker")
        .args(&["system", "df", "--format", "{{.Type}}\t{{.TotalCount}}\t{{.Active}}\t{{.Size}}\t{{.Reclaimable}}"])
        .output()
        .context("Failed to run docker system df")?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    let mut summary = DockerSystemDfSummary {
        images_total: 0,
        images_reclaimable: 0,
        images_active: 0,
        images_count: 0,
        containers_total: 0,
        containers_reclaimable: 0,
        containers_active: 0,
        containers_count: 0,
        volumes_total: 0,
        volumes_reclaimable: 0,
        volumes_active: 0,
        volumes_count: 0,
        build_cache_total: 0,
        build_cache_count: 0,
    };

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 5 {
            continue;
        }

        let type_name = parts[0];
        let total_count: usize = parts[1].parse().unwrap_or(0);
        let active: usize = parts[2].parse().unwrap_or(0);
        let size = parse_size_string(parts[3]);
        let reclaimable = parse_size_from_reclaimable(parts[4]);

        match type_name {
            "Images" => {
                summary.images_total = size;
                summary.images_reclaimable = reclaimable;
                summary.images_active = active;
                summary.images_count = total_count;
            }
            "Containers" => {
                summary.containers_total = size;
                summary.containers_reclaimable = reclaimable;
                summary.containers_active = active;
                summary.containers_count = total_count;
            }
            "Local Volumes" => {
                summary.volumes_total = size;
                summary.volumes_reclaimable = reclaimable;
                summary.volumes_active = active;
                summary.volumes_count = total_count;
            }
            "Build Cache" => {
                summary.build_cache_total = size;
                summary.build_cache_count = total_count;
            }
            _ => {}
        }
    }

    Ok(summary)
}

/// Get individual Docker volume usage
fn get_docker_volumes_usage() -> Result<Vec<VolumeUsage>> {
    // Get list of volumes
    let list_output = Command::new("docker")
        .args(&["volume", "ls", "-q"])
        .output()
        .context("Failed to list docker volumes")?;

    let volume_names: Vec<String> = String::from_utf8_lossy(&list_output.stdout)
        .lines()
        .map(|s| s.to_string())
        .collect();

    let mut volumes = Vec::new();

    for name in volume_names {
        if name.is_empty() {
            continue;
        }

        // Get volume details
        let inspect_output = Command::new("docker")
            .args(&["volume", "inspect", &name, "--format", "{{.Mountpoint}}"])
            .output();

        let mount_point = if let Ok(output) = inspect_output {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        } else {
            String::new()
        };

        // Get size using du (requires sudo, might fail)
        let size_bytes = if !mount_point.is_empty() {
            let du_output = Command::new("sudo")
                .args(&["du", "-sb", &mount_point])
                .output();

            if let Ok(output) = du_output {
                let stdout = String::from_utf8_lossy(&output.stdout);
                stdout
                    .split_whitespace()
                    .next()
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(0)
            } else {
                0
            }
        } else {
            0
        };

        // Determine if volume is critical
        let critical = name.contains("viaduct") || name.contains("viaduct_data");

        // Check if volume is in use by running container
        let ps_output = Command::new("docker")
            .args(&["ps", "-a", "--filter", &format!("volume={}", name), "--format", "{{.ID}}"])
            .output();

        let in_use = if let Ok(output) = ps_output {
            !String::from_utf8_lossy(&output.stdout).trim().is_empty()
        } else {
            false
        };

        volumes.push(VolumeUsage {
            name,
            size_bytes,
            mount_point,
            in_use,
            critical,
        });
    }

    // Sort by size descending
    volumes.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));

    Ok(volumes)
}

/// Calculate growth rate from history
fn calculate_growth_rate(history: &StorageHistory, current: &DiskUsage) -> Option<GrowthRate> {
    if history.measurements.len() < 2 {
        return None;
    }

    // Get measurements from last 30 days
    let cutoff = Utc::now() - chrono::Duration::days(30);
    let recent: Vec<_> = history
        .measurements
        .iter()
        .filter(|m| m.timestamp > cutoff)
        .collect();

    if recent.len() < 2 {
        return None;
    }

    // Calculate daily growth rate (linear regression would be better, but simple average works)
    let first = recent.first()?;
    let last = recent.last()?;

    let days = (last.timestamp - first.timestamp).num_days() as f64;
    if days <= 0.0 {
        return None;
    }

    let bytes_diff = last.total_used_bytes as i64 - first.total_used_bytes as i64;
    let bytes_per_day = bytes_diff as f64 / days;

    // Determine trend
    let trend = if bytes_per_day > 1_000_000_000.0 {
        // Growing more than 1GB/day
        GrowthTrend::Growing
    } else if bytes_per_day < -100_000_000.0 {
        // Declining more than 100MB/day
        GrowthTrend::Declining
    } else {
        GrowthTrend::Stable
    };

    // Calculate days to full (when usage reaches 90%)
    let days_to_full = if bytes_per_day > 0.0 {
        let threshold_bytes = (current.total_bytes as f64 * 0.9) as u64;
        let bytes_remaining = threshold_bytes.saturating_sub(current.used_bytes);
        Some((bytes_remaining as f64 / bytes_per_day) as u64)
    } else {
        None
    };

    Some(GrowthRate {
        bytes_per_day,
        days_to_full,
        trend,
    })
}

/// Parse size string like "4.236GB" to bytes
fn parse_size_string(s: &str) -> u64 {
    let s = s.trim();
    if s == "0B" || s.is_empty() {
        return 0;
    }

    let (num_str, unit) = if s.ends_with("GB") {
        (s.trim_end_matches("GB"), 1_000_000_000u64)
    } else if s.ends_with("MB") {
        (s.trim_end_matches("MB"), 1_000_000u64)
    } else if s.ends_with("KB") || s.ends_with("kB") {
        (s.trim_end_matches("KB").trim_end_matches("kB"), 1_000u64)
    } else if s.ends_with('B') {
        (s.trim_end_matches('B'), 1u64)
    } else {
        return 0;
    };

    num_str.parse::<f64>().unwrap_or(0.0) as u64 * unit
}

/// Parse reclaimable from string like "983MB (17%)"
fn parse_size_from_reclaimable(s: &str) -> u64 {
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.is_empty() {
        return 0;
    }
    parse_size_string(parts[0])
}

/// Get container log sizes for all running containers
pub async fn get_container_log_sizes() -> Result<Vec<ContainerLogInfo>> {
    use bollard::Docker;
    use bollard::container::ListContainersOptions;
    use std::collections::HashMap;

    let docker = Docker::connect_with_local_defaults()
        .context("Failed to connect to Docker daemon")?;

    let mut filters = HashMap::new();
    filters.insert("status".to_string(), vec!["running".to_string(), "exited".to_string(), "paused".to_string()]);

    let options = Some(ListContainersOptions {
        all: true,
        filters,
        ..Default::default()
    });

    let containers = docker.list_containers(options).await?;
    let mut log_infos = Vec::new();

    for container in containers {
        let container_id = match &container.id {
            Some(id) => id.clone(),
            None => continue,
        };

        let container_name = container
            .names
            .as_ref()
            .and_then(|names| names.first())
            .map(|n| n.trim_start_matches('/').to_string())
            .unwrap_or_else(|| "unknown".to_string());

        // Construct log file path: /var/lib/docker/containers/{id}/{id}-json.log
        let log_path = format!("/var/lib/docker/containers/{}/{}-json.log", container_id, container_id);

        // Get log file size using sudo stat command (requires sudo access)
        let log_size_bytes = Command::new("sudo")
            .args(&["stat", "-c", "%s", &log_path])
            .output()
            .ok()
            .and_then(|output| {
                if output.status.success() {
                    String::from_utf8_lossy(&output.stdout)
                        .trim()
                        .parse::<u64>()
                        .ok()
                } else {
                    None
                }
            })
            .unwrap_or(0);

        log_infos.push(ContainerLogInfo {
            container_id,
            container_name,
            log_size_bytes,
            log_path,
        });
    }

    // Sort by size descending (largest first)
    log_infos.sort_by(|a, b| b.log_size_bytes.cmp(&a.log_size_bytes));

    Ok(log_infos)
}

/// Truncate a container's log file (requires sudo privileges)
pub async fn truncate_container_log(container_id: &str) -> Result<()> {
    let log_path = format!("/var/lib/docker/containers/{}/{}-json.log", container_id, container_id);

    // Verify log file exists
    if !std::path::Path::new(&log_path).exists() {
        anyhow::bail!("Log file not found: {}", log_path);
    }

    // Use sudo truncate command to reset log file to 0 bytes
    // This preserves the file (important for Docker log rotation config)
    let output = Command::new("sudo")
        .args(&["truncate", "-s", "0", &log_path])
        .output()
        .context("Failed to execute truncate command")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to truncate log file: {}", stderr);
    }

    Ok(())
}

/// Format bytes to human-readable string
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1_000;
    const MB: u64 = 1_000_000;
    const GB: u64 = 1_000_000_000;
    const TB: u64 = 1_000_000_000_000;

    if bytes >= TB {
        format!("{:.1}TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.1}GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.0}MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.0}KB", bytes as f64 / KB as f64)
    } else {
        format!("{}B", bytes)
    }
}

/// Check if snapshot is needed and save if necessary
/// Returns true if snapshot was saved
pub async fn check_and_save_snapshot_if_needed() -> bool {
    const SNAPSHOT_INTERVAL_HOURS: i64 = 12;

    // Load history
    let mut history = match StorageHistory::load() {
        Ok(h) => h,
        Err(_) => StorageHistory::new(),
    };

    // Check if downsampling needed (migrate old high-frequency data)
    if history.needs_downsampling(SNAPSHOT_INTERVAL_HOURS) {
        history.downsample_to_interval(SNAPSHOT_INTERVAL_HOURS);
        let _ = history.save();
    }

    // Check if snapshot is needed
    let should_save = match history.measurements.last() {
        Some(last) => {
            let elapsed = chrono::Utc::now() - last.timestamp;
            elapsed.num_hours() >= SNAPSHOT_INTERVAL_HOURS
        }
        None => true, // First time - save immediately
    };

    if !should_save {
        return false;
    }

    // Take snapshot
    match analyze_storage().await {
        Ok(analysis) => {
            let measurement = StorageMeasurement {
                timestamp: chrono::Utc::now(),
                total_used_bytes: analysis.system_disk.used_bytes,
                docker_volumes_bytes: analysis.docker_volumes.iter().map(|v| v.size_bytes).sum(),
                docker_images_bytes: analysis.docker_images.total_bytes,
            };

            history.add_measurement(measurement);
            let _ = history.save();
            true
        }
        Err(_) => false,
    }
}

/// Get current log rotation configuration from environment and docker-compose.yml
pub fn get_log_rotation_config() -> Result<LogRotationConfig> {
    use std::collections::HashMap;

    // Read global settings from environment variables
    let driver = std::env::var("LOGGING_DRIVER").unwrap_or_else(|_| "json-file".to_string());
    let max_size = std::env::var("LOG_MAX_SIZE").unwrap_or_else(|_| "100m".to_string());
    let max_file = std::env::var("LOG_MAX_FILE").unwrap_or_else(|_| "3".to_string());

    let global = LogRotationSettings {
        driver,
        max_size,
        max_file,
    };

    // Parse docker-compose.yml to find per-container overrides
    let overrides = parse_compose_logging_overrides()?;

    Ok(LogRotationConfig {
        global,
        overrides,
    })
}

/// Parse docker-compose.yml to extract per-container logging configurations
fn parse_compose_logging_overrides() -> Result<std::collections::HashMap<String, LogRotationSettings>> {
    use std::collections::HashMap;
    use std::fs;

    let mut overrides = HashMap::new();

    // Find docker-compose.yml
    let compose_path = std::path::Path::new("docker-compose.yml");
    if !compose_path.exists() {
        // Not an error - just no overrides
        return Ok(overrides);
    }

    // Read and parse YAML
    let content = fs::read_to_string(compose_path)?;
    let yaml: serde_yaml::Value = serde_yaml::from_str(&content)
        .context("Failed to parse docker-compose.yml")?;

    // Extract services
    if let Some(services) = yaml.get("services").and_then(|s| s.as_mapping()) {
        for (service_name, service_config) in services.iter() {
            let name = service_name.as_str().unwrap_or_default().to_string();

            // Check if this service has custom logging configuration
            if let Some(logging) = service_config.get("logging").and_then(|l| l.as_mapping()) {
                // Check if it's not using the anchor (custom config)
                let driver = logging.get("driver")
                    .and_then(|d| d.as_str())
                    .unwrap_or("json-file")
                    .to_string();

                let options = logging.get("options").and_then(|o| o.as_mapping());

                if let Some(opts) = options {
                    let max_size = opts.get("max-size")
                        .and_then(|s| s.as_str())
                        .unwrap_or("100m")
                        .to_string();

                    let max_file = opts.get("max-file")
                        .and_then(|f| f.as_str())
                        .map(|s| s.to_string())
                        .or_else(|| opts.get("max-file").and_then(|f| f.as_i64()).map(|i| i.to_string()))
                        .unwrap_or_else(|| "3".to_string());

                    overrides.insert(name, LogRotationSettings {
                        driver,
                        max_size,
                        max_file,
                    });
                }
            }
        }
    }

    Ok(overrides)
}

/// Update global log rotation configuration in .env file
/// Note: Requires container restart to take effect
pub fn update_global_log_rotation(settings: &LogRotationSettings) -> Result<()> {
    use std::fs;
    use std::io::{BufRead, BufReader, Write};

    // Find .env file
    let env_path = std::path::Path::new(".env");
    if !env_path.exists() {
        anyhow::bail!(".env file not found");
    }

    // Read current .env file
    let file = fs::File::open(env_path)?;
    let reader = BufReader::new(file);
    let mut lines: Vec<String> = reader.lines().collect::<Result<_, _>>()?;

    // Update or add configuration lines
    let mut found_driver = false;
    let mut found_max_size = false;
    let mut found_max_file = false;

    for line in &mut lines {
        if line.starts_with("LOGGING_DRIVER=") {
            *line = format!("LOGGING_DRIVER={}", settings.driver);
            found_driver = true;
        } else if line.starts_with("LOG_MAX_SIZE=") {
            *line = format!("LOG_MAX_SIZE={}", settings.max_size);
            found_max_size = true;
        } else if line.starts_with("LOG_MAX_FILE=") {
            *line = format!("LOG_MAX_FILE={}", settings.max_file);
            found_max_file = true;
        }
    }

    // Add missing lines
    if !found_driver {
        lines.push(format!("LOGGING_DRIVER={}", settings.driver));
    }
    if !found_max_size {
        lines.push(format!("LOG_MAX_SIZE={}", settings.max_size));
    }
    if !found_max_file {
        lines.push(format!("LOG_MAX_FILE={}", settings.max_file));
    }

    // Write back to file
    let mut file = fs::File::create(env_path)?;
    for line in lines {
        writeln!(file, "{}", line)?;
    }

    Ok(())
}

/// Update per-container log rotation configuration in docker-compose.yml
/// If settings is None, removes the override (container will use global settings)
pub fn update_container_log_rotation(container_name: &str, settings: Option<&LogRotationSettings>) -> Result<()> {
    use std::fs;

    let compose_path = std::path::Path::new("docker-compose.yml");
    if !compose_path.exists() {
        anyhow::bail!("docker-compose.yml not found");
    }

    // Read and parse YAML
    let content = fs::read_to_string(compose_path)?;
    let mut yaml: serde_yaml::Value = serde_yaml::from_str(&content)
        .context("Failed to parse docker-compose.yml")?;

    // Navigate to services
    if let Some(services) = yaml.get_mut("services").and_then(|s| s.as_mapping_mut()) {
        // Find the service
        if let Some(service_name_key) = services.keys().find(|k| k.as_str() == Some(container_name)).cloned() {
            if let Some(service_config) = services.get_mut(&service_name_key).and_then(|s| s.as_mapping_mut()) {
                if let Some(settings) = settings {
                    // Add or update logging configuration
                    let mut logging_map = serde_yaml::Mapping::new();
                    logging_map.insert(
                        serde_yaml::Value::String("driver".to_string()),
                        serde_yaml::Value::String(settings.driver.clone())
                    );

                    let mut options_map = serde_yaml::Mapping::new();
                    options_map.insert(
                        serde_yaml::Value::String("max-size".to_string()),
                        serde_yaml::Value::String(settings.max_size.clone())
                    );
                    options_map.insert(
                        serde_yaml::Value::String("max-file".to_string()),
                        serde_yaml::Value::String(settings.max_file.clone())
                    );

                    logging_map.insert(
                        serde_yaml::Value::String("options".to_string()),
                        serde_yaml::Value::Mapping(options_map)
                    );

                    service_config.insert(
                        serde_yaml::Value::String("logging".to_string()),
                        serde_yaml::Value::Mapping(logging_map)
                    );
                } else {
                    // Remove logging configuration (use global)
                    service_config.remove(&serde_yaml::Value::String("logging".to_string()));

                    // Add back the anchor reference
                    service_config.insert(
                        serde_yaml::Value::String("logging".to_string()),
                        serde_yaml::Value::String("*default-logging".to_string())
                    );
                }
            } else {
                anyhow::bail!("Service '{}' not found in docker-compose.yml", container_name);
            }
        } else {
            anyhow::bail!("Service '{}' not found in docker-compose.yml", container_name);
        }
    } else {
        anyhow::bail!("No services section found in docker-compose.yml");
    }

    // Write back to file (preserving formatting as much as possible)
    let yaml_str = serde_yaml::to_string(&yaml)?;
    fs::write(compose_path, yaml_str)?;

    Ok(())
}

/// Get log rotation settings for a specific container
pub fn get_container_log_rotation(container_name: &str) -> Result<LogRotationSettings> {
    let config = get_log_rotation_config()?;

    // Check if there's an override for this container
    if let Some(override_settings) = config.overrides.get(container_name) {
        Ok(override_settings.clone())
    } else {
        // Use global settings
        Ok(config.global.clone())
    }
}
