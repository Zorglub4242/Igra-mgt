/// Plugin registry for loading and matching metric plugins
///
/// Manages a collection of plugins, matches containers to appropriate plugins,
/// and fetches metrics using the configured fetchers.

use anyhow::{Result, Context};
use std::path::Path;
use std::collections::HashMap;
use std::time::{Instant, Duration};
use std::sync::RwLock;

use super::plugin::{PluginConfig, DisplayPriority, FetcherType};
use super::fetchers::{MetricValue, AnyFetcher, PrometheusFetcher, LogsFetcher, format_metric};

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
            eprintln!("[WARN] Plugins directory does not exist: {}", dir_path.display());
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
                        eprintln!("[INFO] Loaded plugin: {} from {}", config.plugin.name, path.display());
                        plugins.push(config);
                    }
                    Err(e) => {
                        eprintln!("[ERROR] Failed to load plugin from {}: {}", path.display(), e);
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
    /// 1. ~/.config/l2-mgt/plugins/
    /// 2. /etc/l2-mgt/plugins/
    /// 3. ./plugins/ (development fallback)
    pub fn load_from_standard_locations() -> Result<Self> {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());

        let search_paths = vec![
            format!("{}/.config/l2-mgt/plugins", home),
            "/etc/l2-mgt/plugins".to_string(),
            "./plugins".to_string(),
        ];

        for path in &search_paths {
            let p = Path::new(path);
            if p.exists() && p.is_dir() {
                eprintln!("[INFO] Loading plugins from: {}", path);
                return Self::load_from_directory(path);
            }
        }

        eprintln!("[WARN] No plugin directory found in standard locations");
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
            ("block-builder", include_str!("../../../plugins/block-builder.toml")),
            ("kaswallet", include_str!("../../../plugins/kaswallet.toml")),
            ("rpc-provider", include_str!("../../../plugins/rpc-provider.toml")),
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
    pub fn find_plugin(&self, container_name: &str, container_image: &str) -> Option<&PluginConfig> {
        self.plugins
            .iter()
            .find(|plugin| plugin.matches_container(container_name, container_image))
    }

    /// Check if a cached metric is still valid
    fn is_cache_valid(&self, container_name: &str, metric_name: &str, cache_duration: u64) -> bool {
        if let Ok(cache) = self.cache.read() {
            if let Some(cached) = cache.get(&(container_name.to_string(), metric_name.to_string())) {
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
            let (value, formatted) = if self.is_cache_valid(container_name, &metric_def.name, metric_def.cache_duration()) {
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
                        cache.insert(cache_key, CachedMetric {
                            value: val,
                            formatted: formatted.clone(),
                            fetched_at: Instant::now(),
                        });
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
            let (value, formatted) = if self.is_cache_valid(container_name, &metric_def.name, metric_def.cache_duration()) {
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
                        cache.insert(cache_key, CachedMetric {
                            value: val,
                            formatted: formatted.clone(),
                            fetched_at: Instant::now(),
                        });
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
            matchers: vec![
                super::super::plugin::ContainerMatcher {
                    match_type: super::super::plugin::MatchType::ImageContains,
                    value: "reth".to_string(),
                },
            ],
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
