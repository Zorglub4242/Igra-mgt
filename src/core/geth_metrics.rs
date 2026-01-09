/// Geth Prometheus metrics fetching and parsing
///
/// Fetches metrics from Geth execution layer and parses key performance indicators
use anyhow::Result;

#[derive(Debug, Clone, Default)]
pub struct GethMetrics {
    // Block/Chain metrics
    pub block_height: Option<u64>,
    pub finalized_block: Option<u64>,

    // Network metrics
    pub peers_connected: Option<u64>,
    pub network_ingress: Option<u64>, // bytes
    pub network_egress: Option<u64>,  // bytes

    // Transaction pool metrics
    pub txpool_pending: Option<u64>,
    pub txpool_queued: Option<u64>,
    pub txpool_local: Option<u64>,

    // Sync metrics
    pub sync_progress: Option<f64>, // 0.0 to 1.0

    // System metrics
    pub cpu_percent: Option<f64>,
    pub memory_allocs: Option<u64>,
    pub memory_used: Option<u64>,
}

/// Fetch Geth metrics from the Prometheus endpoint
///
/// Since the metrics port (6060) is only exposed within Docker network,
/// we use docker exec with curl or bash's /dev/tcp to fetch from inside the container
pub async fn fetch_geth_metrics(container_name: &str) -> Result<GethMetrics> {
    // Use docker exec with bash /dev/tcp to fetch metrics from inside the container
    let output = tokio::process::Command::new("docker")
        .args(&[
            "exec",
            container_name,
            "sh",
            "-c",
            "exec 3<>/dev/tcp/localhost/6060 && echo -e 'GET /debug/metrics/prometheus HTTP/1.0\\r\\n\\r\\n' >&3 && cat <&3"
        ])
        .output()
        .await?;

    if !output.status.success() {
        return Ok(GethMetrics::default());
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
fn parse_prometheus_metrics(text: &str) -> GethMetrics {
    let mut metrics = GethMetrics::default();

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

        let metric_name = parts[0].split('{').next().unwrap_or(parts[0]);

        if let Ok(value) = parts[parts.len() - 1].parse::<f64>() {
            match metric_name {
                // Block/Chain metrics
                "chain_head_block" => {
                    metrics.block_height = Some(value as u64);
                }
                "chain_head_header" if metrics.block_height.is_none() => {
                    // Fallback to header if block not available
                    metrics.block_height = Some(value as u64);
                }

                // Network metrics (p2p ingress/egress)
                "p2p_ingress" => {
                    metrics.network_ingress = Some(value as u64);
                }
                "p2p_egress" => {
                    metrics.network_egress = Some(value as u64);
                }
                "p2p_peers" => {
                    metrics.peers_connected = Some(value as u64);
                }

                // Transaction pool metrics
                "txpool_pending" => {
                    metrics.txpool_pending = Some(value as u64);
                }
                "txpool_queued" => {
                    metrics.txpool_queued = Some(value as u64);
                }
                "txpool_local" => {
                    metrics.txpool_local = Some(value as u64);
                }

                // System metrics
                "system_cpu_sysload" => {
                    metrics.cpu_percent = Some(value);
                }
                "system_memory_allocs" => {
                    metrics.memory_allocs = Some(value as u64);
                }
                "system_memory_used" => {
                    metrics.memory_used = Some(value as u64);
                }

                _ => {}
            }
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
# HELP chain_head_block Current block number
# TYPE chain_head_block gauge
chain_head_block 123456

# HELP p2p_peers Number of connected peers
# TYPE p2p_peers gauge
p2p_peers 25

# HELP txpool_pending Transactions in pending pool
# TYPE txpool_pending gauge
txpool_pending 10
"#;

        let metrics = parse_prometheus_metrics(sample);
        assert_eq!(metrics.block_height, Some(123456));
        assert_eq!(metrics.peers_connected, Some(25));
        assert_eq!(metrics.txpool_pending, Some(10));
    }

    #[test]
    fn test_parse_real_geth_metrics() {
        let sample = r#"chain_head_block 3008769
chain_head_finalized 3005700
chain_head_header 3008769
p2p_peers 15
txpool_pending 0
txpool_queued 2"#;

        let metrics = parse_prometheus_metrics(sample);
        assert_eq!(metrics.block_height, Some(3008769));
        assert_eq!(metrics.peers_connected, Some(15));
        assert_eq!(metrics.txpool_pending, Some(0));
        assert_eq!(metrics.txpool_queued, Some(2));
    }
}
