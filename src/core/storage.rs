use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

/// Main storage analysis result
#[derive(Debug, Clone)]
pub struct StorageAnalysis {
    pub system_disk: DiskUsage,
    pub docker_images: DockerStorageInfo,
    pub docker_volumes: Vec<VolumeUsage>,
    pub docker_containers: DockerStorageInfo,
    pub docker_build_cache: DockerStorageInfo,
    pub reclaimable_space: u64,
    pub growth_rate: Option<GrowthRate>,
}

/// System disk usage information
#[derive(Debug, Clone)]
pub struct DiskUsage {
    pub filesystem: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub use_percent: f64,
    pub mount_point: String,
}

/// Docker storage category info
#[derive(Debug, Clone)]
pub struct DockerStorageInfo {
    pub total_bytes: u64,
    pub reclaimable_bytes: u64,
    pub active_count: usize,
    pub total_count: usize,
}

/// Individual Docker volume usage
#[derive(Debug, Clone)]
pub struct VolumeUsage {
    pub name: String,
    pub size_bytes: u64,
    pub mount_point: String,
    pub in_use: bool,
    pub critical: bool, // Mark critical volumes like viaduct_data
}

/// Growth rate analysis
#[derive(Debug, Clone)]
pub struct GrowthRate {
    pub bytes_per_day: f64,
    pub days_to_full: Option<u64>,
    pub trend: GrowthTrend,
}

#[derive(Debug, Clone, PartialEq)]
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
}

/// Analyze current storage usage
pub async fn analyze_storage() -> Result<StorageAnalysis> {
    let system_disk = get_system_disk_usage()?;
    let docker_summary = get_docker_system_df()?;
    let volumes = get_docker_volumes_usage()?;

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
