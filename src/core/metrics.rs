/// Metrics collection for system resources
///
/// System metrics are collected and displayed in the TUI dashboard.
///
/// Implemented metrics:
/// - System resources: CPU, Memory, Disk (via shell commands in app.rs)
/// - Container stats: CPU, Memory, Network I/O (via Docker Stats API in docker.rs)
/// - Real-time monitoring with 2-second refresh interval
///
/// Metrics are displayed in:
/// - TUI Dashboard header (system-wide resources)
/// - Services screen table (per-container resources)
/// - Color-coded alerts for high usage (>80% red, >60% yellow)
///
/// This module is not currently used as metrics are collected directly
/// in app.rs (system) and docker.rs (containers) for performance reasons.

use anyhow::Result;

#[allow(dead_code)]
pub struct MetricsCollector;

#[allow(dead_code)]
impl MetricsCollector {
    pub fn new() -> Self {
        Self
    }

    pub async fn collect(&self) -> Result<()> {
        // Metrics collection is implemented in:
        // - app.rs: collect_system_resources() for CPU/Memory/Disk
        // - docker.rs: get_container_stats() for container metrics
        Ok(())
    }
}
