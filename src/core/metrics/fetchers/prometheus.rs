/// Prometheus metrics fetcher
///
/// Fetches metrics from Prometheus-compatible HTTP endpoints using Docker exec with /dev/tcp

use anyhow::{Result, Context};
use super::MetricFetcher;

#[derive(Debug, Clone)]
pub struct PrometheusFetcher {
    pub port: u16,
    pub path: String,
}

impl PrometheusFetcher {
    pub fn new(port: u16, path: String) -> Self {
        Self { port, path }
    }

    /// Parse a Prometheus metric value from text output
    fn parse_metric_value(text: &str, metric_name: &str) -> Option<f64> {
        for line in text.lines() {
            // Skip comments and empty lines
            if line.starts_with('#') || line.trim().is_empty() {
                continue;
            }

            // Parse metric lines (format: metric_name value OR metric_name{labels} value)
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 2 {
                continue;
            }

            let line_metric_name = parts[0].split('{').next().unwrap_or(parts[0]);

            if line_metric_name == metric_name {
                if let Ok(value) = parts[parts.len() - 1].parse::<f64>() {
                    return Some(value);
                }
            }
        }

        None
    }
}

impl MetricFetcher for PrometheusFetcher {
    async fn fetch_raw(&self, container_name: &str) -> Result<String> {
        // Use docker exec with bash /dev/tcp to fetch metrics from inside the container
        // This avoids requiring curl/wget to be installed in the container
        let cmd = format!(
            "exec 3<>/dev/tcp/localhost/{} && echo -e 'GET {} HTTP/1.0\\r\\n\\r\\n' >&3 && cat <&3",
            self.port, self.path
        );

        let output = tokio::process::Command::new("docker")
            .args(&["exec", container_name, "bash", "-c", &cmd])
            .output()
            .await
            .context("Failed to execute docker exec command")?;

        if !output.status.success() {
            return Ok(String::new());
        }

        let metrics_text = String::from_utf8_lossy(&output.stdout);

        // Skip HTTP headers - metrics start after first blank line
        let metrics_only = metrics_text
            .split("\r\n\r\n")
            .nth(1)
            .unwrap_or(&metrics_text);

        Ok(metrics_only.to_string())
    }

    async fn fetch_metric(&self, container_name: &str, metric_name: &str) -> Result<Option<f64>> {
        let raw = self.fetch_raw(container_name).await?;
        Ok(Self::parse_metric_value(&raw, metric_name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_metric_value() {
        let sample = r#"
# HELP reth_blockchain_tree_canonical_chain_height Current canonical chain height
# TYPE reth_blockchain_tree_canonical_chain_height gauge
reth_blockchain_tree_canonical_chain_height 123456

# HELP reth_network_connected_peers Number of connected peers
# TYPE reth_network_connected_peers gauge
reth_network_connected_peers 42

# HELP some_metric_with_labels{label="value"} Some metric
# TYPE some_metric_with_labels gauge
some_metric_with_labels{label="value"} 99
"#;

        assert_eq!(
            PrometheusFetcher::parse_metric_value(sample, "reth_blockchain_tree_canonical_chain_height"),
            Some(123456.0)
        );
        assert_eq!(
            PrometheusFetcher::parse_metric_value(sample, "reth_network_connected_peers"),
            Some(42.0)
        );
        assert_eq!(
            PrometheusFetcher::parse_metric_value(sample, "some_metric_with_labels"),
            Some(99.0)
        );
        assert_eq!(
            PrometheusFetcher::parse_metric_value(sample, "nonexistent_metric"),
            None
        );
    }
}
