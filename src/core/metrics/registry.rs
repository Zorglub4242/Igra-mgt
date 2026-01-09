/// Plugin registry for loading and matching metric plugins
///
/// Manages a collection of plugins, matches containers to appropriate plugins,
/// and fetches metrics using the configured fetchers.
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;
use std::sync::RwLock;
use std::time::{Duration, Instant};

use super::fetchers::{format_metric, AnyFetcher, LogsFetcher, MetricValue, PrometheusFetcher};
use super::plugin::{DisplayPriority, FetcherType, PluginConfig};

/// Cached metric value with timestamp
#[derive(Debug, Clone)]
struct CachedMetric {
    value: f64,
    formatted: String,
    fetched_at: Instant,
}

/// Plugin registry that manages all loaded plugins with caching
pub struct PluginRegistry {
    plugins: Vec<PluginConfig>,
    /// Cache: (container_name, metric_name) -> CachedMetric
    /// Using RwLock for thread-safe interior mutability to allow cache updates through &self
    cache: RwLock<HashMap<(String, String), CachedMetric>>,
}

impl PluginRegistry {
    /// Create a new empty plugin registry
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
            cache: RwLock::new(HashMap::new()),
        }
    }

    /// Load all plugins from a directory
    pub fn load_from_directory<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut plugins = Vec::new();

        let dir_path = path.as_ref();
        if !dir_path.exists() {
            eprintln!(
                "[WARN] Plugins directory does not exist: {}",
                dir_path.display()
            );
            return Ok(Self {
                plugins,
                cache: RwLock::new(HashMap::new()),
            });
        }

        for entry in std::fs::read_dir(dir_path)
            .with_context(|| format!("Failed to read plugins directory: {}", dir_path.display()))?
        {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                match PluginConfig::load_from_file(&path) {
                    Ok(config) => {
                        eprintln!(
                            "[INFO] Loaded plugin: {} from {}",
                            config.plugin.name,
                            path.display()
                        );
                        plugins.push(config);
                    }
                    Err(e) => {
                        eprintln!(
                            "[ERROR] Failed to load plugin from {}: {}",
                            path.display(),
                            e
                        );
                    }
                }
            }
        }

        Ok(Self {
            plugins,
            cache: RwLock::new(HashMap::new()),
        })
    }

    /// Load plugins from standard system locations
    ///
    /// Tries the following locations in order:
    /// - Linux: ~/.config/igra-cli/plugins/, /etc/igra-cli/plugins/, ./plugins/
    /// - Windows: %APPDATA%/igra-cli/plugins/, %PROGRAMDATA%/igra-cli/plugins/, ./plugins/
    ///
    /// If no plugin directory exists, embedded plugins will be extracted to the user config directory.
    pub fn load_from_standard_locations() -> Result<Self> {
        use crate::core::metrics::embedded;
        use dirs::config_dir;

        let mut search_paths: Vec<String> = Vec::new();
        let mut user_config_path: Option<String> = None;

        // Get platform-specific config directories
        #[cfg(target_os = "windows")]
        {
            // Windows: %APPDATA%/igra-cli/plugins and %PROGRAMDATA%/igra-cli/plugins
            if let Some(config_dir) = config_dir() {
                let user_path = config_dir.join("igra-cli").join("plugins");
                user_config_path = Some(user_path.display().to_string());
                search_paths.push(user_path.display().to_string());
            }

            // %PROGRAMDATA%/igra-cli/plugins (system-wide)
            if let Ok(programdata) = std::env::var("PROGRAMDATA") {
                search_paths.push(format!("{}\\igra-cli\\plugins", programdata));
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            // Linux/Unix: ~/.config/igra-cli/plugins and /etc/igra-cli/plugins
            if let Some(config_dir) = config_dir() {
                let user_path = config_dir.join("igra-cli").join("plugins");
                user_config_path = Some(user_path.display().to_string());
                search_paths.push(user_path.display().to_string());
            }

            // Fallback to HOME-based path if dirs crate fails
            if let Ok(home) = std::env::var("HOME") {
                if user_config_path.is_none() {
                    user_config_path = Some(format!("{}/.config/igra-cli/plugins", home));
                }
            }

            search_paths.push("/etc/igra-cli/plugins".to_string());
        }

        // Development fallback (current directory)
        search_paths.push("./plugins".to_string());

        // Try to find existing plugin directory
        for path in &search_paths {
            let p = Path::new(path);
            if p.exists() && p.is_dir() && embedded::has_plugins(p) {
                eprintln!("[INFO] Loading plugins from: {}", path);
                return Self::load_from_directory(path);
            }
        }

        // No plugin directory found - extract embedded plugins to user config
        if let Some(ref config_path) = user_config_path {
            eprintln!("[INFO] No plugin directory found, extracting embedded plugins...");
            eprintln!("[INFO] Target directory: {}", config_path);

            match embedded::extract_plugins_to_dir(config_path) {
                Ok(count) => {
                    eprintln!("[INFO] Extracted {} plugin(s) to {}", count, config_path);
                    return Self::load_from_directory(config_path);
                }
                Err(e) => {
                    eprintln!("[WARN] Failed to extract embedded plugins: {}", e);
                }
            }
        }

        eprintln!("[WARN] No plugin directory found and could not extract embedded plugins");
        eprintln!("[WARN] Tried: {}", search_paths.join(", "));

        // Return empty registry if no directories found
        Ok(Self::new())
    }

    /// Load built-in plugins from embedded resources (deprecated)
    ///
    /// This method is deprecated in favor of load_from_standard_locations().
    /// It remains for backward compatibility.
    #[deprecated(note = "Use load_from_standard_locations() instead")]
    pub fn load_builtin() -> Result<Self> {
        let mut plugins = Vec::new();

        // Built-in plugin configurations (embedded)
        let builtin_configs = vec![
            ("reth", include_str!("../../../plugins/reth.toml")),
            ("geth", include_str!("../../../plugins/geth.toml")),
            ("kaspad", include_str!("../../../plugins/kaspad.toml")),
            (
                "block-builder",
                include_str!("../../../plugins/block-builder.toml"),
            ),
            ("kaswallet", include_str!("../../../plugins/kaswallet.toml")),
            (
                "rpc-provider",
                include_str!("../../../plugins/rpc-provider.toml"),
            ),
            ("traefik", include_str!("../../../plugins/traefik.toml")),
            ("viaduct", include_str!("../../../plugins/viaduct.toml")),
        ];

        for (name, toml_content) in builtin_configs {
            match toml::from_str::<PluginConfig>(toml_content) {
                Ok(config) => {
                    eprintln!("[INFO] Loaded built-in plugin: {}", name);
                    plugins.push(config);
                }
                Err(e) => {
                    eprintln!("[ERROR] Failed to parse built-in plugin {}: {}", name, e);
                }
            }
        }

        Ok(Self {
            plugins,
            cache: RwLock::new(HashMap::new()),
        })
    }

    /// Find the first plugin that matches the given container
    pub fn find_plugin(
        &self,
        container_name: &str,
        container_image: &str,
    ) -> Option<&PluginConfig> {
        self.plugins
            .iter()
            .find(|plugin| plugin.matches_container(container_name, container_image))
    }

    /// Find the first plugin that matches the given system service
    pub fn find_service_plugin(&self, service_name: &str) -> Option<&PluginConfig> {
        self.plugins
            .iter()
            .find(|plugin| plugin.matches_service(service_name))
    }

    /// Check if a cached metric is still valid
    fn is_cache_valid(&self, container_name: &str, metric_name: &str, cache_duration: u64) -> bool {
        if let Ok(cache) = self.cache.read() {
            if let Some(cached) = cache.get(&(container_name.to_string(), metric_name.to_string()))
            {
                return cached.fetched_at.elapsed() < Duration::from_secs(cache_duration);
            }
        }
        false
    }

    /// Get condensed metrics for services list (primary and secondary)
    pub async fn get_condensed_metrics(
        &self,
        container_name: &str,
        container_image: &str,
    ) -> Result<(Option<String>, Option<String>)> {
        let plugin = match self.find_plugin(container_name, container_image) {
            Some(p) => p,
            None => return Ok((None, None)),
        };

        let fetcher = self.create_fetcher(&plugin.fetcher)?;

        let mut primary_metric = None;
        let mut secondary_metric = None;

        for metric_def in &plugin.metrics {
            // Only fetch metrics marked for condensed display
            if metric_def.display_priority != DisplayPriority::Primary
                && metric_def.display_priority != DisplayPriority::Secondary
            {
                continue;
            }

            let cache_key = (container_name.to_string(), metric_def.name.clone());

            // Check if we have a valid cached value
            let (value, formatted) = if self.is_cache_valid(
                container_name,
                &metric_def.name,
                metric_def.cache_duration(),
            ) {
                // Use cached value
                if let Ok(cache) = self.cache.read() {
                    if let Some(cached) = cache.get(&cache_key) {
                        (Some(cached.value), cached.formatted.clone())
                    } else {
                        (None, String::new())
                    }
                } else {
                    (None, String::new())
                }
            } else {
                // Fetch new value
                let value = if let Some(prom_metric) = &metric_def.prometheus_metric {
                    // Prometheus-based metric
                    fetcher.fetch_metric(container_name, prom_metric).await?
                } else if let Some(regex_pattern) = &metric_def.regex_pattern {
                    // Log-based metric with per-metric pattern
                    let raw_logs = fetcher.fetch_raw(container_name).await?;
                    LogsFetcher::parse_with_regex(&raw_logs, regex_pattern)
                } else {
                    None
                };

                if let Some(val) = value {
                    let formatted = format_metric(&metric_def.display_format, val);

                    // Update cache
                    if let Ok(mut cache) = self.cache.write() {
                        cache.insert(
                            cache_key,
                            CachedMetric {
                                value: val,
                                formatted: formatted.clone(),
                                fetched_at: Instant::now(),
                            },
                        );
                    }

                    (Some(val), formatted)
                } else {
                    (None, String::new())
                }
            };

            if value.is_some() {
                match metric_def.display_priority {
                    DisplayPriority::Primary => {
                        primary_metric = Some(formatted);
                    }
                    DisplayPriority::Secondary => {
                        secondary_metric = Some(formatted);
                    }
                    _ => {}
                }
            }
        }

        Ok((primary_metric, secondary_metric))
    }

    /// Fetch all metrics for a container (for detail view)
    pub async fn fetch_all_metrics(
        &self,
        container_name: &str,
        container_image: &str,
    ) -> Result<Vec<MetricValue>> {
        let plugin = match self.find_plugin(container_name, container_image) {
            Some(p) => p,
            None => return Ok(Vec::new()),
        };

        let fetcher = self.create_fetcher(&plugin.fetcher)?;
        let mut metrics = Vec::new();

        for metric_def in &plugin.metrics {
            let cache_key = (container_name.to_string(), metric_def.name.clone());

            // Check if we have a valid cached value
            let (value, formatted) = if self.is_cache_valid(
                container_name,
                &metric_def.name,
                metric_def.cache_duration(),
            ) {
                // Use cached value
                if let Ok(cache) = self.cache.read() {
                    if let Some(cached) = cache.get(&cache_key) {
                        (Some(cached.value), cached.formatted.clone())
                    } else {
                        (None, String::new())
                    }
                } else {
                    (None, String::new())
                }
            } else {
                // Fetch new value
                let value = if let Some(prom_metric) = &metric_def.prometheus_metric {
                    // Prometheus-based metric
                    fetcher.fetch_metric(container_name, prom_metric).await?
                } else if let Some(regex_pattern) = &metric_def.regex_pattern {
                    // Log-based metric with per-metric pattern
                    let raw_logs = fetcher.fetch_raw(container_name).await?;
                    LogsFetcher::parse_with_regex(&raw_logs, regex_pattern)
                } else {
                    None
                };

                if let Some(val) = value {
                    let formatted = format_metric(&metric_def.display_format, val);

                    // Update cache
                    if let Ok(mut cache) = self.cache.write() {
                        cache.insert(
                            cache_key,
                            CachedMetric {
                                value: val,
                                formatted: formatted.clone(),
                                fetched_at: Instant::now(),
                            },
                        );
                    }

                    (Some(val), formatted)
                } else {
                    (None, String::new())
                }
            };

            if let Some(val) = value {
                metrics.push(MetricValue {
                    name: metric_def.name.clone(),
                    value: val,
                    formatted,
                    category: metric_def.category.clone(),
                });
            }
        }

        Ok(metrics)
    }

    /// Fetch all metrics for a system service (for detail view)
    pub async fn fetch_service_metrics(
        &self,
        service_name: &str,
        plugin: &PluginConfig,
    ) -> Result<Vec<MetricValue>> {
        // For system services, we fetch logs using journalctl
        // This is different from Docker containers which use docker logs or prometheus

        let mut metrics = Vec::new();

        // Fetch service logs using systemctl/journalctl
        // We'll use the SystemServiceManager interface to get logs
        use crate::core::system_service::SystemServiceManager;
        let system_service = SystemServiceManager::new(true); // use sudo

        // Get recent logs (last 100 lines should be enough for metric extraction)
        let service_full_name = if service_name.ends_with(".service") {
            service_name.to_string()
        } else {
            format!("{}.service", service_name)
        };

        let raw_logs: String = match system_service.get_logs(&service_full_name, 100).await {
            Ok(logs) => logs,
            Err(e) => {
                eprintln!("[WARN] Failed to fetch logs for {}: {}", service_name, e);
                return Ok(Vec::new());
            }
        };

        // Parse metrics using the plugin's metric definitions
        for metric_def in &plugin.metrics {
            let cache_key = (service_name.to_string(), metric_def.name.clone());

            // Check if we have a valid cached value
            let (value, formatted) =
                if self.is_cache_valid(service_name, &metric_def.name, metric_def.cache_duration())
                {
                    // Use cached value
                    if let Ok(cache) = self.cache.read() {
                        if let Some(cached) = cache.get(&cache_key) {
                            (Some(cached.value), cached.formatted.clone())
                        } else {
                            (None, String::new())
                        }
                    } else {
                        (None, String::new())
                    }
                } else {
                    // Fetch new value from logs using regex pattern
                    let value = if let Some(regex_pattern) = &metric_def.regex_pattern {
                        // Log-based metric with per-metric pattern
                        LogsFetcher::parse_with_regex(&raw_logs, regex_pattern)
                    } else {
                        None
                    };

                    if let Some(val) = value {
                        let formatted = format_metric(&metric_def.display_format, val);

                        // Update cache
                        if let Ok(mut cache) = self.cache.write() {
                            cache.insert(
                                cache_key,
                                CachedMetric {
                                    value: val,
                                    formatted: formatted.clone(),
                                    fetched_at: Instant::now(),
                                },
                            );
                        }

                        (Some(val), formatted)
                    } else {
                        (None, String::new())
                    }
                };

            if let Some(val) = value {
                metrics.push(MetricValue {
                    name: metric_def.name.clone(),
                    value: val,
                    formatted,
                    category: metric_def.category.clone(),
                });
            }
        }

        Ok(metrics)
    }

    /// Create a fetcher based on the configuration
    fn create_fetcher(&self, config: &super::plugin::FetcherConfig) -> Result<AnyFetcher> {
        match config.fetcher_type {
            FetcherType::Prometheus => {
                let port = config.port.context("Prometheus fetcher requires port")?;
                let path = config.path.as_deref().unwrap_or("/metrics").to_string();

                Ok(AnyFetcher::Prometheus(PrometheusFetcher::new(port, path)))
            }
            FetcherType::Http => {
                // For now, HTTP and Prometheus use the same fetcher
                let port = config.port.context("HTTP fetcher requires port")?;
                let path = config.path.as_deref().unwrap_or("/").to_string();

                Ok(AnyFetcher::Prometheus(PrometheusFetcher::new(port, path)))
            }
            FetcherType::Logs => {
                let lines = 100; // Default to last 100 lines

                // Support both global pattern and per-metric patterns
                if let Some(pattern) = &config.log_pattern {
                    // Global pattern for all metrics
                    Ok(AnyFetcher::Logs(LogsFetcher::new(pattern.clone(), lines)))
                } else {
                    // No global pattern - metrics will use their own regex_pattern
                    Ok(AnyFetcher::Logs(LogsFetcher::new_without_pattern(lines)))
                }
            }
            // System service fetchers - not implemented yet, return an error
            FetcherType::Systemd | FetcherType::SystemLogs => Err(anyhow::anyhow!(
                "System service fetchers not yet implemented"
            )),
        }
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_plugin() {
        let mut registry = PluginRegistry::new();

        // Add a test plugin
        let config = PluginConfig {
            plugin: super::super::plugin::PluginMeta {
                name: "reth".to_string(),
                description: "Reth client".to_string(),
            },
            matchers: vec![super::super::plugin::ContainerMatcher {
                match_type: super::super::plugin::MatchType::ImageContains,
                value: "reth".to_string(),
            }],
            fetcher: super::super::plugin::FetcherConfig {
                fetcher_type: FetcherType::Prometheus,
                method: Some(super::super::plugin::FetchMethod::DockerExec),
                port: Some(9001),
                path: Some("/metrics".to_string()),
                log_pattern: None,
            },
            metrics: vec![],
        };

        registry.plugins.push(config);

        // Should find plugin by image
        assert!(registry.find_plugin("test", "reth:latest").is_some());

        // Should not find plugin for non-matching container
        assert!(registry.find_plugin("test", "nginx:latest").is_none());
    }
}
