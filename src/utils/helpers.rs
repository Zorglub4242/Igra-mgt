/// Helper utilities for the IGRA CLI

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use chrono::{DateTime, Local};

/// Auto-detect project root from running Docker containers
fn detect_from_docker() -> Option<PathBuf> {
    use std::process::Command;

    // Try to find IGRA Orchestra containers - check multiple containers
    let container_names = ["traefik", "kaswallet-0", "execution-layer", "kaspad", "viaduct"];

    for container_name in &container_names {
        let output = Command::new("docker")
            .args(&["ps", "--filter", &format!("name={}", container_name), "--format", "{{.ID}}"])
            .output()
            .ok()?;

        let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if container_id.is_empty() {
            continue;
        }

        // Inspect the container to find mounts
        let output = Command::new("docker")
            .args(&["inspect", &container_id, "--format", "{{.Mounts}}"])
            .output()
            .ok()?;

        let mounts = String::from_utf8_lossy(&output.stdout);

        // Parse mount entries - format: [{bind  /host/path /container/path  rw true rprivate} ...]
        // Look for bind mounts (not volumes) that point to the host filesystem
        for segment in mounts.split("bind") {
            // Extract paths between spaces
            let parts: Vec<&str> = segment.trim().split_whitespace().collect();
            if parts.len() >= 2 {
                let host_path = parts[0];
                // Skip if it starts with volume or other non-path indicators
                if host_path.starts_with('/') && !host_path.starts_with("/var/lib/docker") {
                    let path = PathBuf::from(host_path);

                    // Check if this path or its parent contains docker-compose.yml
                    let mut check_path = Some(path.as_path());
                    while let Some(p) = check_path {
                        if p.join("docker-compose.yml").exists() {
                            return Some(p.to_path_buf());
                        }
                        check_path = p.parent();
                    }
                }
            }
        }
    }

    None
}

/// Get the project root directory (where docker-compose.yml is located)
pub fn get_project_root() -> Result<PathBuf> {
    use crate::utils::AppConfig;

    // 1. Check saved configuration
    if let Ok(config) = AppConfig::load() {
        if let Some(root) = config.project_root {
            let path = PathBuf::from(&root);
            if path.join("docker-compose.yml").exists() {
                return Ok(path);
            }
        }
    }

    // 2. Check environment variable
    if let Ok(project_root) = std::env::var("IGRA_PROJECT_ROOT") {
        let path = PathBuf::from(project_root);
        if path.join("docker-compose.yml").exists() {
            // Save to config for future use
            if let Ok(mut config) = AppConfig::load() {
                let _ = config.set_project_root(path.clone());
            }
            return Ok(path);
        }
    }

    // 3. Try to auto-detect from running Docker containers
    if let Some(detected_path) = detect_from_docker() {
        eprintln!("✓ Auto-detected IGRA Orchestra at: {}", detected_path.display());
        eprintln!("  Saving to ~/.config/igra-cli/config.toml");

        // Save to config
        if let Ok(mut config) = AppConfig::load() {
            let _ = config.set_project_root(detected_path.clone());
        }

        return Ok(detected_path);
    }

    // 4. Search for docker-compose.yml in current and parent directories
    let current_dir = std::env::current_dir()
        .context("Failed to get current directory")?;

    let mut dir = current_dir.as_path();
    loop {
        let compose_file = dir.join("docker-compose.yml");
        if compose_file.exists() {
            // Save to config
            if let Ok(mut config) = AppConfig::load() {
                let _ = config.set_project_root(dir.to_path_buf());
            }
            return Ok(dir.to_path_buf());
        }

        match dir.parent() {
            Some(parent) => dir = parent,
            None => break,
        }
    }

    // 5. Try common locations for IGRA Orchestra
    let common_paths = [
        PathBuf::from("/home/kaspa/igra2/igra-orchestra-public"),
        PathBuf::from("/home/kaspa/igra-orchestra-public"),
        current_dir.join("igra-orchestra-public"),
        current_dir.join("../igra-orchestra-public"),
    ];

    for path in &common_paths {
        if path.join("docker-compose.yml").exists() {
            eprintln!("✓ Found IGRA Orchestra at: {}", path.display());
            eprintln!("  Saving to ~/.config/igra-cli/config.toml");

            // Save to config
            if let Ok(mut config) = AppConfig::load() {
                let _ = config.set_project_root(path.clone());
            }

            return Ok(path.clone());
        }
    }

    // 6. Not found - show helpful error
    anyhow::bail!(
        "Could not find IGRA Orchestra installation\n\n\
        Auto-detection failed. Please specify the location:\n\n\
        Option 1 - Set environment variable:\n\
          export IGRA_PROJECT_ROOT=/path/to/igra-orchestra-public\n\
          igra-cli\n\n\
        Option 2 - Run from the project directory:\n\
          cd /path/to/igra-orchestra-public\n\
          igra-cli\n\n\
        Option 3 - Manually configure:\n\
          mkdir -p ~/.config/igra-cli\n\
          echo 'project_root = \"/path/to/igra-orchestra-public\"' > ~/.config/igra-cli/config.toml\n\
          igra-cli"
    )
}

/// Format bytes to human-readable size
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.2} {}", size, UNITS[unit_index])
    }
}

/// Format duration to human-readable string
pub fn format_duration(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    if days > 0 {
        format!("{}d {}h", days, hours)
    } else if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, secs)
    } else {
        format!("{}s", secs)
    }
}

/// Format timestamp to human-readable string
pub fn format_timestamp(timestamp: i64) -> String {
    let dt = DateTime::from_timestamp(timestamp, 0)
        .unwrap_or_else(|| DateTime::from_timestamp(0, 0).unwrap());
    let local: DateTime<Local> = dt.into();
    local.format("%Y-%m-%d %H:%M:%S").to_string()
}

/// Truncate string with ellipsis
pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

/// Mask sensitive data (show only first and last N characters)
pub fn mask_sensitive(value: &str, visible_chars: usize) -> String {
    if value.len() <= visible_chars * 2 {
        "*".repeat(value.len())
    } else {
        let start = &value[..visible_chars];
        let end = &value[value.len() - visible_chars..];
        format!("{}...{}", start, end)
    }
}

/// Generate a random hex string of specified length
pub fn generate_hex_string(length: usize) -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..length)
        .map(|_| format!("{:x}", rng.gen::<u8>() % 16))
        .collect()
}

/// Validate hex string
pub fn is_valid_hex(s: &str) -> bool {
    s.chars().all(|c| c.is_ascii_hexdigit())
}

/// Validate domain name (basic check)
pub fn is_valid_domain(domain: &str) -> bool {
    let parts: Vec<&str> = domain.split('.').collect();
    if parts.len() < 2 {
        return false;
    }

    parts.iter().all(|part| {
        !part.is_empty()
        && part.chars().all(|c| c.is_alphanumeric() || c == '-')
        && !part.starts_with('-')
        && !part.ends_with('-')
    })
}

/// Validate email address (basic check)
pub fn is_valid_email(email: &str) -> bool {
    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 {
        return false;
    }

    !parts[0].is_empty() && is_valid_domain(parts[1])
}

/// Check if a file exists and is readable
pub fn is_file_readable<P: AsRef<Path>>(path: P) -> bool {
    path.as_ref().exists() && path.as_ref().is_file()
}

/// Check if a directory exists and is writable
pub fn is_dir_writable<P: AsRef<Path>>(path: P) -> bool {
    if let Ok(metadata) = std::fs::metadata(&path) {
        metadata.is_dir() && !metadata.permissions().readonly()
    } else {
        false
    }
}

/// Parse Docker container status to simplified state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerState {
    Running,
    Stopped,
    Paused,
    Restarting,
    Dead,
    Unknown,
}

impl From<&str> for ContainerState {
    fn from(status: &str) -> Self {
        let status_lower = status.to_lowercase();
        if status_lower.contains("up") || status_lower.contains("running") {
            ContainerState::Running
        } else if status_lower.contains("paused") {
            ContainerState::Paused
        } else if status_lower.contains("restarting") {
            ContainerState::Restarting
        } else if status_lower.contains("dead") || status_lower.contains("removing") {
            ContainerState::Dead
        } else if status_lower.contains("exited") || status_lower.contains("stopped") {
            ContainerState::Stopped
        } else {
            ContainerState::Unknown
        }
    }
}

impl ContainerState {
    pub fn is_running(&self) -> bool {
        matches!(self, ContainerState::Running)
    }

    pub fn to_string(&self) -> &'static str {
        match self {
            ContainerState::Running => "Running",
            ContainerState::Stopped => "Stopped",
            ContainerState::Paused => "Paused",
            ContainerState::Restarting => "Restarting",
            ContainerState::Dead => "Dead",
            ContainerState::Unknown => "Unknown",
        }
    }

    /// Get color for terminal display
    pub fn color(&self) -> &'static str {
        match self {
            ContainerState::Running => "green",
            ContainerState::Stopped => "gray",
            ContainerState::Paused => "yellow",
            ContainerState::Restarting => "cyan",
            ContainerState::Dead => "red",
            ContainerState::Unknown => "white",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1048576), "1.00 MB");
        assert_eq!(format_bytes(1073741824), "1.00 GB");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(30), "30s");
        assert_eq!(format_duration(90), "1m 30s");
        assert_eq!(format_duration(3661), "1h 1m");
        assert_eq!(format_duration(86400), "1d 0h");
    }

    #[test]
    fn test_mask_sensitive() {
        let token = "5e7f294e4c92a9aa661fae8d347d832d";
        let masked = mask_sensitive(token, 4);
        assert_eq!(masked, "5e7f...832d");
    }

    #[test]
    fn test_is_valid_hex() {
        assert!(is_valid_hex("deadbeef"));
        assert!(is_valid_hex("123456"));
        assert!(!is_valid_hex("ghij"));
        assert!(!is_valid_hex("hello"));
    }

    #[test]
    fn test_is_valid_domain() {
        assert!(is_valid_domain("example.com"));
        assert!(is_valid_domain("sub.example.com"));
        assert!(!is_valid_domain("invalid"));
        assert!(!is_valid_domain(".com"));
    }

    #[test]
    fn test_is_valid_email() {
        assert!(is_valid_email("user@example.com"));
        assert!(!is_valid_email("invalid.email"));
        assert!(!is_valid_email("@example.com"));
    }

    #[test]
    fn test_container_state() {
        assert_eq!(ContainerState::from("Up 2 hours"), ContainerState::Running);
        assert_eq!(ContainerState::from("Exited (0)"), ContainerState::Stopped);
        assert!(ContainerState::Running.is_running());
        assert!(!ContainerState::Stopped.is_running());
    }
}
