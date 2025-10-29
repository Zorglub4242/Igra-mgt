/// Plugin configuration parsing from TOML files
///
/// Defines the structure of metric plugins and provides parsing functionality.

use serde::{Deserialize, Serialize};
use anyhow::{Result, Context};
use std::path::Path;

/// Complete plugin configuration loaded from TOML file
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PluginConfig {
    pub plugin: PluginMeta,
    #[serde(rename = "match")]
    pub matchers: Vec<ContainerMatcher>,
    pub fetcher: FetcherConfig,
    pub metrics: Vec<MetricDefinition>,
}

/// Plugin metadata
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PluginMeta {
    pub name: String,
    pub description: String,
}

/// Container matching rules
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ContainerMatcher {
    #[serde(rename = "type")]
    pub match_type: MatchType,
    pub value: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MatchType {
    ImageContains,
    ImageEquals,
    NameEquals,
    NameContains,
    // System service matchers
    ServiceNameEquals,
    ServiceNameContains,
}

/// Fetcher configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FetcherConfig {
    #[serde(rename = "type")]
    pub fetcher_type: FetcherType,
    #[serde(default)]
    pub method: Option<FetchMethod>,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub log_pattern: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FetcherType {
    Prometheus,
    Http,
    Logs,
    // System service fetchers
    Systemd,      // Fetch metrics from systemctl status/show
    SystemLogs,   // Fetch metrics from journalctl logs
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FetchMethod {
    DockerExec,
    HttpDirect,
}

/// Metric definition
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MetricDefinition {
    pub name: String,
    #[serde(default)]
    pub prometheus_metric: Option<String>,
    #[serde(default)]
    pub json_path: Option<String>,
    #[serde(default)]
    pub regex_pattern: Option<String>,
    pub display_format: String,
    pub display_priority: DisplayPriority,
    #[serde(default)]
    pub category: Option<String>,
    /// How often to refresh this metric (in seconds). Default: 5 seconds
    #[serde(default = "default_refresh_interval")]
    pub refresh_interval_secs: u64,
    /// How long to cache this metric value (in seconds). Default: same as refresh_interval
    #[serde(default)]
    pub cache_duration_secs: Option<u64>,
}

fn default_refresh_interval() -> u64 {
    5
}

impl MetricDefinition {
    /// Get the effective cache duration (uses refresh_interval if cache_duration not set)
    pub fn cache_duration(&self) -> u64 {
        self.cache_duration_secs.unwrap_or(self.refresh_interval_secs)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DisplayPriority {
    Primary,    // Shown in condensed view (services list) as primary metric
    Secondary,  // Shown in condensed view (services list) as secondary metric
    Detail,     // Only shown in detail view
}

impl PluginConfig {
    /// Load plugin configuration from TOML file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read plugin file: {}", path.as_ref().display()))?;

        let config: PluginConfig = toml::from_str(&content)
            .with_context(|| format!("Failed to parse TOML from: {}", path.as_ref().display()))?;

        Ok(config)
    }

    /// Check if this plugin matches the given container
    pub fn matches_container(&self, container_name: &str, container_image: &str) -> bool {
        let image_lower = container_image.to_lowercase();
        let name_lower = container_name.to_lowercase();

        for matcher in &self.matchers {
            let value_lower = matcher.value.to_lowercase();

            let is_match = match matcher.match_type {
                MatchType::ImageContains => image_lower.contains(&value_lower),
                MatchType::ImageEquals => image_lower == value_lower,
                MatchType::NameEquals => name_lower == value_lower,
                MatchType::NameContains => name_lower.contains(&value_lower),
                // System service matchers - not applicable for container matching
                MatchType::ServiceNameEquals | MatchType::ServiceNameContains => false,
            };

            if is_match {
                return true;
            }
        }

        false
    }

    /// Check if this plugin matches the given system service
    pub fn matches_service(&self, service_name: &str) -> bool {
        let name_lower = service_name.to_lowercase();

        for matcher in &self.matchers {
            let value_lower = matcher.value.to_lowercase();

            let is_match = match matcher.match_type {
                // For system services, also check container name matchers for backward compatibility
                MatchType::NameEquals => name_lower == value_lower,
                MatchType::NameContains => name_lower.contains(&value_lower),
                MatchType::ServiceNameEquals => name_lower == value_lower,
                MatchType::ServiceNameContains => name_lower.contains(&value_lower),
                // Image matchers not applicable for system services
                MatchType::ImageContains | MatchType::ImageEquals => false,
            };

            if is_match {
                return true;
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_matches_container() {
        let config = PluginConfig {
            plugin: PluginMeta {
                name: "reth".to_string(),
                description: "Reth client".to_string(),
            },
            matchers: vec![
                ContainerMatcher {
                    match_type: MatchType::ImageContains,
                    value: "reth".to_string(),
                },
                ContainerMatcher {
                    match_type: MatchType::NameEquals,
                    value: "execution-layer".to_string(),
                },
            ],
            fetcher: FetcherConfig {
                fetcher_type: FetcherType::Prometheus,
                method: Some(FetchMethod::DockerExec),
                port: Some(9001),
                path: Some("/metrics".to_string()),
                log_pattern: None,
            },
            metrics: vec![],
        };

        // Should match by image
        assert!(config.matches_container("some-container", "igranetwork/reth:latest"));

        // Should match by name
        assert!(config.matches_container("execution-layer", "custom/image:tag"));

        // Should not match
        assert!(!config.matches_container("other-service", "nginx:latest"));
    }

    #[test]
    fn test_case_insensitive_matching() {
        let config = PluginConfig {
            plugin: PluginMeta {
                name: "test".to_string(),
                description: "Test".to_string(),
            },
            matchers: vec![
                ContainerMatcher {
                    match_type: MatchType::ImageContains,
                    value: "RETH".to_string(),
                },
            ],
            fetcher: FetcherConfig {
                fetcher_type: FetcherType::Prometheus,
                method: None,
                port: None,
                path: None,
                log_pattern: None,
            },
            metrics: vec![],
        };

        // Should match regardless of case
        assert!(config.matches_container("test", "reth:latest"));
        assert!(config.matches_container("test", "RETH:latest"));
        assert!(config.matches_container("test", "Reth:latest"));
    }
}
