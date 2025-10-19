/// Reth Prometheus metrics fetching and parsing
///
/// Fetches metrics from Reth execution layer and parses key performance indicators

use anyhow::Result;
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct RethMetrics {
    pub blocks_processed: Option<u64>,
    pub sync_stage: Option<String>,
    pub peers_connected: Option<u64>,
    pub gas_used: Option<u64>,
    pub transactions_processed: Option<u64>,
}

/// Fetch Reth metrics from the Prometheus endpoint
///
/// Since the metrics port (9001) is only exposed within Docker network,
/// we use docker exec with bash's /dev/tcp to fetch from inside the container
pub async fn fetch_reth_metrics() -> Result<RethMetrics> {
    // Use docker exec with bash /dev/tcp to fetch metrics from inside the container
    // This avoids requiring curl/wget to be installed in the container
    let output = tokio::process::Command::new("docker")
        .args(&[
            "exec",
            "execution-layer",
            "bash",
            "-c",
            "exec 3<>/dev/tcp/localhost/9001 && echo -e 'GET /metrics HTTP/1.0\\r\\n\\r\\n' >&3 && cat <&3"
        ])
        .output()
        .await?;

    if !output.status.success() {
        return Ok(RethMetrics::default());
    }

    let metrics_text = String::from_utf8_lossy(&output.stdout);
    // Skip HTTP headers - metrics start after first blank line
    let metrics_only = metrics_text
        .split("\r\n\r\n")
        .nth(1)
        .unwrap_or(&metrics_text);

    Ok(parse_prometheus_metrics(metrics_only))
}

/// Parse Prometheus format metrics
fn parse_prometheus_metrics(text: &str) -> RethMetrics {
    let mut metrics = RethMetrics::default();

    for line in text.lines() {
        // Skip comments and empty lines
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }

        // Parse metric lines (format: metric_name{labels} value)
        if let Some((metric_name, rest)) = line.split_once(' ') {
            let metric_name = metric_name.split('{').next().unwrap_or(metric_name);

            if let Ok(value) = rest.trim().parse::<f64>() {
                match metric_name {
                    // Block processing metrics - use payloads_resolved_block for latest block
                    "reth_payloads_resolved_block" => {
                        metrics.blocks_processed = Some(value as u64);
                    }
                    // Also check static file entries for headers as fallback
                    "reth_static_files_segment_entries" => {
                        if line.contains("segment=\"headers\"") && metrics.blocks_processed.is_none() {
                            metrics.blocks_processed = Some(value as u64);
                        }
                    }
                    // Peer count - tracked peers
                    "reth_network_tracked_peers" => {
                        metrics.peers_connected = Some(value as u64);
                    }
                    // Transaction pool
                    "reth_transaction_pool_transactions" => {
                        metrics.transactions_processed = Some(value as u64);
                    }
                    _ => {}
                }
            }
        }
    }

    // Detect sync stage based on whether we have blocks
    if let Some(blocks) = metrics.blocks_processed {
        if blocks > 0 {
            metrics.sync_stage = Some("Active".to_string());
        }
    }

    metrics
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_prometheus_metrics() {
        let sample = r#"
# HELP reth_sync_block_number Current block number
# TYPE reth_sync_block_number gauge
reth_sync_block_number 123456

# HELP reth_network_connected_peers Number of connected peers
# TYPE reth_network_connected_peers gauge
reth_network_connected_peers 42

# HELP reth_transaction_pool_transactions Transactions in pool
# TYPE reth_transaction_pool_transactions gauge
reth_transaction_pool_transactions 10
"#;

        let metrics = parse_prometheus_metrics(sample);
        assert_eq!(metrics.blocks_processed, Some(123456));
        assert_eq!(metrics.peers_connected, Some(42));
        assert_eq!(metrics.transactions_processed, Some(10));
    }
}
