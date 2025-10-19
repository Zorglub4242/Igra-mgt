/// Reth Prometheus metrics fetching and parsing
///
/// Fetches metrics from Reth execution layer and parses key performance indicators

use anyhow::Result;
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct RethMetrics {
    // Block/Chain metrics
    pub blocks_processed: Option<u64>,
    pub canonical_chain_height: Option<u64>,
    pub headers_synced: Option<u64>,
    pub sync_stage: Option<String>,
    pub sync_checkpoint: Option<u64>,

    // Network metrics
    pub peers_connected: Option<u64>,
    pub peers_tracked: Option<u64>,

    // Transaction metrics
    pub transactions_total: Option<u64>,
    pub transactions_pending: Option<u64>,
    pub transactions_blob: Option<u64>,
    pub transactions_inserted: Option<u64>,
    pub tps: Option<f64>,

    // Performance metrics
    pub memory_bytes: Option<u64>,
    pub gas_processed: Option<u64>,
    pub payloads_initiated: Option<u64>,

    // Blockchain tree metrics
    pub in_mem_blocks: Option<u64>,
    pub reorgs_total: Option<u64>,
    pub reorg_depth: Option<u64>,
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
                    // Block/Chain metrics
                    "reth_payloads_resolved_block" => {
                        metrics.blocks_processed = Some(value as u64);
                    }
                    "reth_blockchain_tree_canonical_chain_height" => {
                        metrics.canonical_chain_height = Some(value as u64);
                    }
                    "reth_static_files_segment_entries" => {
                        if line.contains("segment=\"headers\"") {
                            metrics.headers_synced = Some(value as u64);
                        } else if line.contains("segment=\"transactions\"") {
                            metrics.transactions_total = Some(value as u64);
                        }
                    }
                    "reth_sync_checkpoint" => {
                        if line.contains("stage=\"Finish\"") {
                            metrics.sync_checkpoint = Some(value as u64);
                        }
                    }

                    // Network metrics
                    "reth_network_connected_peers" => {
                        metrics.peers_connected = Some(value as u64);
                    }
                    "reth_network_tracked_peers" => {
                        metrics.peers_tracked = Some(value as u64);
                    }

                    // Transaction metrics
                    "reth_transaction_pool_pending_pool_transactions" => {
                        metrics.transactions_pending = Some(value as u64);
                    }
                    "reth_transaction_pool_blob_pool_transactions" => {
                        metrics.transactions_blob = Some(value as u64);
                    }
                    "reth_transaction_pool_inserted_transactions" => {
                        metrics.transactions_inserted = Some(value as u64);
                    }

                    // Performance metrics
                    "reth_process_resident_memory_bytes" => {
                        metrics.memory_bytes = Some(value as u64);
                    }
                    "reth_sync_execution_gas_processed_total" => {
                        metrics.gas_processed = Some(value as u64);
                    }
                    "reth_payloads_initiated_jobs" => {
                        metrics.payloads_initiated = Some(value as u64);
                    }

                    // Blockchain tree metrics
                    "reth_blockchain_tree_in_mem_state_num_blocks" => {
                        metrics.in_mem_blocks = Some(value as u64);
                    }
                    "reth_blockchain_tree_reorgs" => {
                        metrics.reorgs_total = Some(value as u64);
                    }
                    "reth_blockchain_tree_latest_reorg_depth" => {
                        metrics.reorg_depth = Some(value as u64);
                    }

                    _ => {}
                }
            }
        }
    }

    // Detect sync stage based on checkpoint
    if metrics.sync_checkpoint.is_some() {
        metrics.sync_stage = Some("Synced".to_string());
    } else if metrics.blocks_processed.is_some() {
        metrics.sync_stage = Some("Active".to_string());
    }

    metrics
}

/// Calculate TPS from two metrics snapshots
pub fn calculate_tps(
    current: &RethMetrics,
    previous: &RethMetrics,
    elapsed_secs: f64,
) -> Option<f64> {
    if elapsed_secs <= 0.0 {
        return None;
    }

    let current_tx = current.transactions_inserted?;
    let previous_tx = previous.transactions_inserted?;

    if current_tx >= previous_tx {
        let delta = current_tx - previous_tx;
        Some(delta as f64 / elapsed_secs)
    } else {
        None
    }
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
