/// Pluggable metrics system for Docker containers
///
/// This module provides a flexible, plugin-based architecture for collecting and displaying
/// metrics from various Docker containers. Users can define custom metrics via TOML configuration
/// files without modifying Rust code.
///
/// # Architecture
///
/// - `plugin`: TOML configuration parsing and plugin definitions
/// - `registry`: Plugin loading, container matching, and metrics fetching
/// - `fetchers`: Different methods to fetch metrics (Prometheus, Docker exec, logs)
///
/// # Usage
///
/// ```rust
/// use crate::core::metrics::registry::PluginRegistry;
///
/// // Load all plugins from the plugins/ directory
/// let registry = PluginRegistry::new("./plugins").await?;
///
/// // Get condensed metrics for a container (for services list)
/// let (primary, secondary) = registry.get_condensed_metrics("execution-layer", "reth:latest").await?;
///
/// // Get all metrics for a container (for detail view)
/// let metrics = registry.fetch_all_metrics("execution-layer", "reth:latest").await?;
/// ```

pub mod plugin;
pub mod registry;
pub mod fetchers;

// Re-export commonly used types
pub use plugin::{PluginConfig, MetricDefinition, DisplayPriority};
pub use registry::PluginRegistry;
pub use fetchers::MetricValue;
