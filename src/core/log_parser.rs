/// Log parsing for extracting meaningful status information from service logs
///
/// This module provides intelligent parsing of container logs to extract:
/// - Sync status (synced, syncing, stalled)
/// - Performance metrics (TPS, latency, block rate)
/// - Block heights and numbers
/// - Transaction throughput
/// - Health indicators

use regex::Regex;
use std::sync::OnceLock;

/// Service-specific metrics extracted from logs
#[derive(Debug, Clone, Default)]
pub struct ServiceMetrics {
    /// Current status indicator (e.g., "Synced", "Syncing", "Building...")
    pub status_text: Option<String>,

    /// Primary metric (e.g., block number, TPS, DAA score)
    pub primary_metric: Option<String>,

    /// Secondary metric (e.g., latency, queue length, peers)
    pub secondary_metric: Option<String>,

    /// Health indicator: true = healthy, false = issue detected
    pub is_healthy: bool,
}

/// Parse service logs based on service name
pub fn parse_service_logs(service_name: &str, logs: &str) -> ServiceMetrics {
    match service_name {
        s if s.contains("kaspad") => parse_kaspad_logs(logs),
        s if s.contains("execution-layer") => parse_execution_layer_logs(logs),
        s if s.contains("viaduct") => parse_viaduct_logs(logs),
        s if s.contains("block-builder") => parse_block_builder_logs(logs),
        s if s.contains("rpc-provider") => parse_rpc_provider_logs(logs),
        s if s.contains("kaswallet") => parse_kaswallet_logs(logs),
        s if s.contains("node-health-check") => parse_health_check_logs(logs),
        s if s.contains("traefik") => parse_traefik_logs(logs),
        _ => ServiceMetrics::default(),
    }
}

/// Parse kaspad logs
/// Indicators:
/// - "Accepted X blocks ... via relay" = Synced
/// - "Processed X blocks and Y headers" = Syncing
/// - "Tx throughput stats: X.XX u-tps" = Transaction rate
fn parse_kaspad_logs(logs: &str) -> ServiceMetrics {
    static ACCEPTED_RE: OnceLock<Regex> = OnceLock::new();
    static THROUGHPUT_RE: OnceLock<Regex> = OnceLock::new();
    static PROCESSED_RE: OnceLock<Regex> = OnceLock::new();

    let accepted_re = ACCEPTED_RE.get_or_init(|| {
        Regex::new(r"Accepted (\d+) blocks.*via relay").unwrap()
    });

    let throughput_re = THROUGHPUT_RE.get_or_init(|| {
        Regex::new(r"Tx throughput stats: ([\d.]+) u-tps").unwrap()
    });

    let processed_re = PROCESSED_RE.get_or_init(|| {
        Regex::new(r"Processed (\d+) blocks and (\d+) headers").unwrap()
    });

    let mut metrics = ServiceMetrics {
        is_healthy: true,
        ..Default::default()
    };

    // Check if synced (accepting blocks via relay)
    if accepted_re.is_match(logs) {
        metrics.status_text = Some("Synced".to_string());

        // Extract TPS if available
        if let Some(caps) = throughput_re.captures(logs) {
            if let Some(tps) = caps.get(1) {
                metrics.primary_metric = Some(format!("{} TPS", tps.as_str()));
            }
        }
    }
    // Check if syncing (processing batches)
    else if let Some(caps) = processed_re.captures(logs) {
        if let (Some(blocks), Some(headers)) = (caps.get(1), caps.get(2)) {
            metrics.status_text = Some("Syncing".to_string());
            metrics.primary_metric = Some(format!("{} blk/10s", blocks.as_str()));
            metrics.secondary_metric = Some(format!("{} hdr", headers.as_str()));
            metrics.is_healthy = true; // Syncing is healthy
        }
    }
    // Check for errors
    else if logs.contains("ERROR") || logs.contains("WARN") {
        metrics.status_text = Some("Warning".to_string());
        metrics.is_healthy = false;
    }

    metrics
}

/// Parse execution-layer (reth) logs
/// Indicators:
/// - "Block added to canonical chain number=X" = Current block
/// - "txs=X" = Transaction count
/// - "peers=X" = Peer count
/// - "gas_used=X" = Gas usage
fn parse_execution_layer_logs(logs: &str) -> ServiceMetrics {
    static BLOCK_RE: OnceLock<Regex> = OnceLock::new();
    static TX_RE: OnceLock<Regex> = OnceLock::new();
    static PEERS_RE: OnceLock<Regex> = OnceLock::new();

    let block_re = BLOCK_RE.get_or_init(|| {
        Regex::new(r"Block added to canonical chain.*number=(\d+)").unwrap()
    });

    let tx_re = TX_RE.get_or_init(|| {
        Regex::new(r"txs=(\d+)").unwrap()
    });

    let peers_re = PEERS_RE.get_or_init(|| {
        Regex::new(r"peers=(\d+)").unwrap()
    });

    let mut metrics = ServiceMetrics {
        is_healthy: true,
        ..Default::default()
    };

    // Extract block number
    if let Some(caps) = block_re.captures(logs) {
        if let Some(block_num) = caps.get(1) {
            metrics.status_text = Some("Active".to_string());
            metrics.primary_metric = Some(format!("#{}", block_num.as_str()));

            // Extract transaction count
            if let Some(tx_caps) = tx_re.captures(logs) {
                if let Some(tx_count) = tx_caps.get(1) {
                    metrics.secondary_metric = Some(format!("{} txs", tx_count.as_str()));
                }
            }
            // Or peer count
            else if let Some(peer_caps) = peers_re.captures(logs) {
                if let Some(peers) = peer_caps.get(1) {
                    metrics.secondary_metric = Some(format!("{} peers", peers.as_str()));
                }
            }
        }
    }

    metrics
}

/// Parse viaduct logs
/// Indicators:
/// - "Adding block X accepting [...]" = Processing
/// - "with score Y to the queue" = DAA score
/// - "Sending took X ms" = Latency
fn parse_viaduct_logs(logs: &str) -> ServiceMetrics {
    static DAA_RE: OnceLock<Regex> = OnceLock::new();
    static LATENCY_RE: OnceLock<Regex> = OnceLock::new();
    static QUEUE_RE: OnceLock<Regex> = OnceLock::new();

    let daa_re = DAA_RE.get_or_init(|| {
        Regex::new(r"with score (\d+) to the queue").unwrap()
    });

    let latency_re = LATENCY_RE.get_or_init(|| {
        Regex::new(r"Sending took (\d+) ms").unwrap()
    });

    let queue_re = QUEUE_RE.get_or_init(|| {
        Regex::new(r"len now (\d+)").unwrap()
    });

    let mut metrics = ServiceMetrics {
        is_healthy: true,
        status_text: Some("Active".to_string()),
        ..Default::default()
    };

    // Extract DAA score
    if let Some(caps) = daa_re.captures(logs) {
        if let Some(daa) = caps.get(1) {
            let daa_str = daa.as_str();
            // Format large numbers with commas
            if let Ok(daa_num) = daa_str.parse::<u64>() {
                metrics.primary_metric = Some(format!("DAA:{}", format_large_number(daa_num)));
            }
        }
    }

    // Extract latency
    if let Some(caps) = latency_re.captures(logs) {
        if let Some(latency) = caps.get(1) {
            let lat_ms: u32 = latency.as_str().parse().unwrap_or(0);
            metrics.secondary_metric = Some(format!("{}ms", lat_ms));

            // Alert on high latency
            if lat_ms > 100 {
                metrics.is_healthy = false;
            }
        }
    }
    // Or queue length
    else if let Some(caps) = queue_re.captures(logs) {
        if let Some(queue) = caps.get(1) {
            metrics.secondary_metric = Some(format!("Q:{}", queue.as_str()));
        }
    }

    metrics
}

/// Parse block-builder logs
/// Indicators:
/// - "Block built with X transactions" = Success
/// - "Building payload" = In progress
fn parse_block_builder_logs(logs: &str) -> ServiceMetrics {
    static BUILT_RE: OnceLock<Regex> = OnceLock::new();
    static BUILDING_RE: OnceLock<Regex> = OnceLock::new();

    let built_re = BUILT_RE.get_or_init(|| {
        Regex::new(r"Block built with (\d+) transactions").unwrap()
    });

    let building_re = BUILDING_RE.get_or_init(|| {
        Regex::new(r"Building payload on parent").unwrap()
    });

    let mut metrics = ServiceMetrics {
        is_healthy: true,
        ..Default::default()
    };

    if let Some(caps) = built_re.captures(logs) {
        metrics.status_text = Some("Built".to_string());
        if let Some(tx_count) = caps.get(1) {
            metrics.primary_metric = Some(format!("{} txs", tx_count.as_str()));
        }
    } else if building_re.is_match(logs) {
        metrics.status_text = Some("Building".to_string());
        metrics.primary_metric = Some("...".to_string());
    }

    metrics
}

/// Parse rpc-provider logs
/// Indicators:
/// - "RPC REQUEST ... method=X" = Request type
/// - "time=Xµs" or "time=Xms" = Latency
fn parse_rpc_provider_logs(logs: &str) -> ServiceMetrics {
    static REQUEST_RE: OnceLock<Regex> = OnceLock::new();
    static TIME_US_RE: OnceLock<Regex> = OnceLock::new();
    static TIME_MS_RE: OnceLock<Regex> = OnceLock::new();

    let request_re = REQUEST_RE.get_or_init(|| {
        Regex::new(r"RPC REQUEST.*method=(\w+)").unwrap()
    });

    let time_us_re = TIME_US_RE.get_or_init(|| {
        Regex::new(r"time=([\d.]+)µs").unwrap()
    });

    let time_ms_re = TIME_MS_RE.get_or_init(|| {
        Regex::new(r"time=([\d.]+)ms").unwrap()
    });

    let mut metrics = ServiceMetrics {
        is_healthy: true,
        status_text: Some("Serving".to_string()),
        ..Default::default()
    };

    // Count recent requests
    let request_count = request_re.find_iter(logs).count();
    if request_count > 0 {
        metrics.primary_metric = Some(format!("{} req/s", request_count / 10)); // Assuming 10s of logs
    }

    // Extract average latency
    let mut total_us = 0.0;
    let mut count = 0;

    for caps in time_us_re.captures_iter(logs) {
        if let Some(time) = caps.get(1) {
            if let Ok(us) = time.as_str().parse::<f64>() {
                total_us += us;
                count += 1;
            }
        }
    }

    for caps in time_ms_re.captures_iter(logs) {
        if let Some(time) = caps.get(1) {
            if let Ok(ms) = time.as_str().parse::<f64>() {
                total_us += ms * 1000.0;
                count += 1;
            }
        }
    }

    if count > 0 {
        let avg_us = total_us / count as f64;
        if avg_us < 1000.0 {
            metrics.secondary_metric = Some(format!("{:.0}µs", avg_us));
        } else {
            metrics.secondary_metric = Some(format!("{:.1}ms", avg_us / 1000.0));
        }
    }

    metrics
}

/// Parse kaswallet logs
/// Indicators:
/// - "Connected to kaspa node successfully" = Connected
/// - "Finished initial sync" = Synced
fn parse_kaswallet_logs(logs: &str) -> ServiceMetrics {
    let mut metrics = ServiceMetrics {
        is_healthy: true,
        ..Default::default()
    };

    if logs.contains("Finished initial sync") {
        metrics.status_text = Some("Synced".to_string());
        metrics.primary_metric = Some("Ready".to_string());
    } else if logs.contains("Connected to kaspa node successfully") {
        metrics.status_text = Some("Syncing".to_string());
        metrics.primary_metric = Some("...".to_string());
    } else if logs.contains("Starting wallet server") {
        metrics.status_text = Some("Starting".to_string());
    }

    metrics
}

/// Parse node-health-check-client logs
/// Indicators:
/// - "Successfully pushed checkpoint block X (latest: Y)" = Sync status
fn parse_health_check_logs(logs: &str) -> ServiceMetrics {
    static CHECKPOINT_RE: OnceLock<Regex> = OnceLock::new();

    let checkpoint_re = CHECKPOINT_RE.get_or_init(|| {
        Regex::new(r"checkpoint block (\d+).*latest: (\d+)").unwrap()
    });

    let mut metrics = ServiceMetrics {
        is_healthy: true,
        ..Default::default()
    };

    if let Some(caps) = checkpoint_re.captures(logs) {
        if let (Some(checkpoint), Some(latest)) = (caps.get(1), caps.get(2)) {
            let cp: u64 = checkpoint.as_str().parse().unwrap_or(0);
            let lt: u64 = latest.as_str().parse().unwrap_or(0);
            let lag = if lt > cp { lt - cp } else { 0 };

            if lag == 0 {
                metrics.status_text = Some("Synced".to_string());
                metrics.is_healthy = true;
            } else if lag < 5 {
                metrics.status_text = Some("OK".to_string());
                metrics.is_healthy = true;
            } else if lag < 10 {
                metrics.status_text = Some("Lagging".to_string());
                metrics.is_healthy = true;
            } else {
                metrics.status_text = Some("Behind".to_string());
                metrics.is_healthy = false;
            }

            metrics.primary_metric = Some(format!("-{} blk", lag));
        }
    }

    metrics
}

/// Parse traefik logs
/// Indicators:
/// - "No ACME certificate generation required" = SSL OK
/// - "ERR" = Configuration error
fn parse_traefik_logs(logs: &str) -> ServiceMetrics {
    let mut metrics = ServiceMetrics {
        is_healthy: true,
        ..Default::default()
    };

    if logs.contains("No ACME certificate generation required") {
        metrics.status_text = Some("SSL OK".to_string());
        metrics.primary_metric = Some("Active".to_string());
    }

    let error_count = logs.matches("ERR").count();
    if error_count > 0 {
        metrics.is_healthy = false;
        metrics.secondary_metric = Some(format!("{} err", error_count));
    }

    metrics
}

/// Format large numbers with comma separators
fn format_large_number(num: u64) -> String {
    let s = num.to_string();
    let mut result = String::new();
    let mut count = 0;

    for c in s.chars().rev() {
        if count == 3 {
            result.push(',');
            count = 0;
        }
        result.push(c);
        count += 1;
    }

    result.chars().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kaspad_synced() {
        let logs = "2025-10-18 20:45:37.476+00:00 [INFO ] Accepted 7 blocks ...0f7b via relay\n\
                    2025-10-18 20:45:46.689+00:00 [INFO ] Tx throughput stats: 5.00 u-tps";

        let metrics = parse_kaspad_logs(logs);
        assert_eq!(metrics.status_text, Some("Synced".to_string()));
        assert_eq!(metrics.primary_metric, Some("5.00 TPS".to_string()));
        assert!(metrics.is_healthy);
    }

    #[test]
    fn test_execution_layer() {
        let logs = "Block added to canonical chain number=7705704 txs=15";

        let metrics = parse_execution_layer_logs(logs);
        assert_eq!(metrics.status_text, Some("Active".to_string()));
        assert_eq!(metrics.primary_metric, Some("#7705704".to_string()));
        assert_eq!(metrics.secondary_metric, Some("15 txs".to_string()));
    }

    #[test]
    fn test_format_large_number() {
        assert_eq!(format_large_number(1234567), "1,234,567");
        assert_eq!(format_large_number(283910951), "283,910,951");
    }
}
