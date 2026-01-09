/// Metric fetchers for different data sources
///
/// Provides different methods to fetch metrics from containers:
/// - Prometheus: Fetch from Prometheus-compatible HTTP endpoints
/// - Docker Exec: Execute commands inside containers to fetch metrics
/// - Logs: Parse metrics from container logs
use anyhow::Result;

pub mod docker_exec;
pub mod logs;
pub mod prometheus;

pub use docker_exec::DockerExecFetcher;
pub use logs::LogsFetcher;
pub use prometheus::PrometheusFetcher;

/// A metric value with its name, raw value, and formatted display string
#[derive(Debug, Clone, serde::Serialize)]
pub struct MetricValue {
    pub name: String,
    pub value: f64,
    pub formatted: String,
    pub category: Option<String>,
}

/// Enum of all metric fetcher types
#[derive(Debug, Clone)]
pub enum AnyFetcher {
    Prometheus(PrometheusFetcher),
    DockerExec(DockerExecFetcher),
    Logs(LogsFetcher),
}

impl AnyFetcher {
    pub async fn fetch_metric(
        &self,
        container_name: &str,
        metric_name: &str,
    ) -> Result<Option<f64>> {
        match self {
            AnyFetcher::Prometheus(f) => f.fetch_metric(container_name, metric_name).await,
            AnyFetcher::DockerExec(f) => f.fetch_metric(container_name, metric_name).await,
            AnyFetcher::Logs(f) => f.fetch_metric(container_name, metric_name).await,
        }
    }

    pub async fn fetch_raw(&self, container_name: &str) -> Result<String> {
        match self {
            AnyFetcher::Prometheus(f) => f.fetch_raw(container_name).await,
            AnyFetcher::DockerExec(f) => f.fetch_raw(container_name).await,
            AnyFetcher::Logs(f) => f.fetch_raw(container_name).await,
        }
    }
}

/// Trait for all metric fetchers
pub trait MetricFetcher: Send + Sync {
    /// Fetch a specific metric value
    fn fetch_metric(
        &self,
        container_name: &str,
        metric_name: &str,
    ) -> impl std::future::Future<Output = Result<Option<f64>>> + Send;

    /// Fetch all metrics as raw Prometheus text (if applicable)
    fn fetch_raw(
        &self,
        container_name: &str,
    ) -> impl std::future::Future<Output = Result<String>> + Send;
}

/// Format a metric value using a display format template
///
/// Supported placeholders:
/// - {value} - Raw numeric value
/// - {value_k} - Value divided by 1,000
/// - {value_m} - Value divided by 1,000,000
/// - {value_mb} - Value in megabytes (bytes / 1024^2)
/// - {value_gb} - Value in gigabytes (bytes / 1024^3)
/// - {value_pct} - Value as percentage (0-100)
pub fn format_metric(template: &str, value: f64) -> String {
    template
        .replace("{value}", &format_number(value))
        .replace("{value_k}", &format_number(value / 1_000.0))
        .replace("{value_m}", &format_number(value / 1_000_000.0))
        .replace("{value_mb}", &format_number(value / (1024.0 * 1024.0)))
        .replace(
            "{value_gb}",
            &format_number(value / (1024.0 * 1024.0 * 1024.0)),
        )
        .replace("{value_pct}", &format!("{:.1}%", value * 100.0))
}

/// Format a number with appropriate precision
fn format_number(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1_000_000_000.0 {
        format!("{:.0}", n)
    } else if n.abs() < 10.0 {
        format!("{:.2}", n)
    } else if n.abs() < 100.0 {
        format!("{:.1}", n)
    } else {
        format!("{:.0}", n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_metric() {
        assert_eq!(format_metric("Block #{value}", 123456.0), "Block #123456");
        assert_eq!(format_metric("{value} peers", 15.0), "15 peers");
        assert_eq!(format_metric("{value_mb} MB", 1048576.0), "1 MB");
        assert_eq!(format_metric("{value_gb} GB", 1073741824.0), "1 GB");
        assert_eq!(
            format_metric("Progress: {value_pct}", 0.857),
            "Progress: 85.7%"
        );
    }

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(123456.0), "123456");
        assert_eq!(format_number(1.5), "1.50");
        assert_eq!(format_number(12.34), "12.3");
        assert_eq!(format_number(123.4), "123");
    }
}
