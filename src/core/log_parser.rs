/// Log parsing for extracting meaningful status information from service logs
///
/// This module provides intelligent parsing of container logs to extract:
/// - Sync status (synced, syncing, stalled)
/// - Performance metrics (TPS, latency, block rate)
/// - Block heights and numbers
/// - Transaction throughput
/// - Health indicators
/// - Individual log line parsing (timestamp, level, module, message)

use regex::Regex;
use std::sync::OnceLock;

/// Parsed Docker Compose log line components
#[derive(Debug, Clone)]
pub struct ParsedLogLine {
    pub timestamp: String,      // Full timestamp: "2025-10-21T08:48:40Z"
    pub service: String,        // Service name from Docker: "viaduct"
    pub module_path: String,    // Rust module path: "viaduct::uni_storage"
    pub module_short: String,   // Last segment: "uni_storage"
    pub level: LogLevel,        // Detected log level
    pub message: String,        // Clean message without bracketed prefix
    pub raw_line: String,       // Original line for fallback
}

/// Log level enum for consistent handling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
    Unknown,
}

impl LogLevel {
    pub fn from_str(s: &str) -> Self {
        let upper = s.to_uppercase();
        if upper.contains("ERROR") {
            LogLevel::Error
        } else if upper.contains("WARN") {
            LogLevel::Warn
        } else if upper.contains("INFO") {
            LogLevel::Info
        } else if upper.contains("DEBUG") {
            LogLevel::Debug
        } else if upper.contains("TRACE") {
            LogLevel::Trace
        } else {
            LogLevel::Unknown
        }
    }

    pub fn to_string(&self) -> &str {
        match self {
            LogLevel::Error => "ERROR",
            LogLevel::Warn => "WARN ",
            LogLevel::Info => "INFO ",
            LogLevel::Debug => "DEBUG",
            LogLevel::Trace => "TRACE",
            LogLevel::Unknown => "     ",
        }
    }

    pub fn color(&self) -> ratatui::style::Color {
        use ratatui::style::Color;
        match self {
            LogLevel::Error => Color::Red,
            LogLevel::Warn => Color::Yellow,
            LogLevel::Info => Color::Cyan,
            LogLevel::Debug => Color::Gray,
            LogLevel::Trace => Color::DarkGray,
            LogLevel::Unknown => Color::White,
        }
    }
}

/// Strip ANSI color codes from log strings
fn strip_ansi_codes(text: &str) -> String {
    static ANSI_RE: OnceLock<Regex> = OnceLock::new();
    let re = ANSI_RE.get_or_init(|| {
        Regex::new(r"\x1b\[[0-9;]*[a-zA-Z]").unwrap()
    });
    re.replace_all(text, "").to_string()
}

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
    // Strip ANSI codes once for all parsers
    let clean_logs = strip_ansi_codes(logs);

    match service_name {
        s if s.contains("kaspad") => parse_kaspad_logs(&clean_logs),
        s if s.contains("execution-layer") => parse_execution_layer_logs(&clean_logs),
        s if s.contains("viaduct") => parse_viaduct_logs(&clean_logs),
        s if s.contains("block-builder") => parse_block_builder_logs(&clean_logs),
        s if s.contains("rpc-provider") => parse_rpc_provider_logs(&clean_logs),
        s if s.contains("kaswallet") => parse_kaswallet_logs(&clean_logs),
        s if s.contains("node-health-check") => parse_health_check_logs(&clean_logs),
        s if s.contains("traefik") => parse_traefik_logs(&clean_logs),
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

/// Parse a Docker Compose log line into components
/// Handles multiple log formats from different services
pub fn parse_docker_log_line(line: &str) -> ParsedLogLine {
    let raw_line = line.to_string();

    // Try to split on first pipe (service separator)
    if let Some(pipe_idx) = line.find('|') {
        let service = line[..pipe_idx].trim().to_string();
        let rest_with_ansi = line[pipe_idx + 1..].trim();

        // Strip ANSI color codes that docker adds
        let rest_cleaned = strip_ansi_codes(rest_with_ansi);
        let rest = rest_cleaned.as_str();

        // Try kaspad format: "YYYY-MM-DD HH:MM:SS.sss+TZ [LEVEL ] message"
        let kaspad_regex = Regex::new(
            r"^(\d{4}-\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:[+-]\d{2}:\d{2})?)\s+\[(ERROR|WARN|INFO|DEBUG|TRACE)\s*\]\s+(.*)$"
        ).unwrap();

        if let Some(caps) = kaspad_regex.captures(rest) {
            let timestamp = caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
            let level_str = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            let message = caps.get(3).map(|m| m.as_str().to_string()).unwrap_or_default();

            let level = match level_str {
                "ERROR" => LogLevel::Error,
                "WARN" => LogLevel::Warn,
                "INFO" => LogLevel::Info,
                "DEBUG" => LogLevel::Debug,
                "TRACE" => LogLevel::Trace,
                _ => LogLevel::Unknown,
            };

            return ParsedLogLine {
                timestamp,
                service,
                module_path: String::new(),
                module_short: String::new(),
                level,
                message,
                raw_line,
            };
        }

        // Try to match bracketed Rust log format: [timestamp LEVEL module::path] message
        let bracketed_regex = Regex::new(
            r"^\[(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z?)\s+(ERROR|WARN|INFO|DEBUG|TRACE)\s+([^\]]+)\]\s*(.*)$"
        ).unwrap();

        if let Some(caps) = bracketed_regex.captures(rest) {
            let timestamp = caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
            let level_str = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            let mut module_path = caps.get(3).map(|m| m.as_str().trim().to_string()).unwrap_or_default();
            let message = caps.get(4).map(|m| m.as_str().to_string()).unwrap_or_default();

            // Handle block-builder format: "module::path: src/file.rs:line"
            if let Some(colon_pos) = module_path.find(": ") {
                let after_colon = &module_path[colon_pos + 2..];
                if after_colon.starts_with("src/") || after_colon.starts_with("/") {
                    module_path = module_path[..colon_pos].trim().to_string();
                }
            }

            let module_short = module_path.split("::").last().unwrap_or(&module_path).to_string();

            let level = match level_str {
                "ERROR" => LogLevel::Error,
                "WARN" => LogLevel::Warn,
                "INFO" => LogLevel::Info,
                "DEBUG" => LogLevel::Debug,
                "TRACE" => LogLevel::Trace,
                _ => LogLevel::Unknown,
            };

            return ParsedLogLine {
                timestamp,
                service,
                module_path,
                module_short,
                level,
                message,
                raw_line,
            };
        }

        // Try non-bracketed format: "HH:MM:SS LEVEL module::path: src/file.rs:line: message"
        let nonbracketed_regex = Regex::new(
            r"^(\d{2}:\d{2}:\d{2}(?:\.\d+)?)\s+(ERROR|WARN|INFO|DEBUG|TRACE)\s+(.+)$"
        ).unwrap();

        if let Some(caps) = nonbracketed_regex.captures(rest) {
            let time_only = caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
            let level_str = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            let remainder = caps.get(3).map(|m| m.as_str().trim()).unwrap_or("");

            let (module_path, message) = if let Some(colon_pos) = remainder.find(": ") {
                let before_colon = &remainder[..colon_pos];
                let after_colon = &remainder[colon_pos + 2..];

                if after_colon.starts_with("src/") || after_colon.starts_with("/") {
                    if let Some(msg_pos) = after_colon.find(": ") {
                        (before_colon.to_string(), after_colon[msg_pos + 2..].to_string())
                    } else {
                        (before_colon.to_string(), String::new())
                    }
                } else {
                    (before_colon.to_string(), after_colon.to_string())
                }
            } else {
                (String::new(), remainder.to_string())
            };

            let module_short = module_path.split("::").last().unwrap_or(&module_path).to_string();

            let level = match level_str {
                "ERROR" => LogLevel::Error,
                "WARN" => LogLevel::Warn,
                "INFO" => LogLevel::Info,
                "DEBUG" => LogLevel::Debug,
                "TRACE" => LogLevel::Trace,
                _ => LogLevel::Unknown,
            };

            return ParsedLogLine {
                timestamp: time_only,
                service,
                module_path,
                module_short,
                level,
                message,
                raw_line,
            };
        }

        // Fallback: Try ISO timestamp + LEVEL + target: message format (execution-layer/reth logs)
        let iso_format_regex = Regex::new(
            r"^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z?)\s+(ERROR|WARN|INFO|DEBUG|TRACE)\s+(.+)$"
        ).unwrap();

        if let Some(caps) = iso_format_regex.captures(rest) {
            let iso_timestamp = caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
            let level_str = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            let remainder = caps.get(3).map(|m| m.as_str().trim()).unwrap_or("");

            let (module_path, message) = if let Some(colon_pos) = remainder.find(": ") {
                let before_colon = &remainder[..colon_pos];
                let after_colon = &remainder[colon_pos + 2..];
                (before_colon.to_string(), after_colon.to_string())
            } else {
                (String::new(), remainder.to_string())
            };

            let module_short = module_path.split("::").last().unwrap_or(&module_path).to_string();

            let level = match level_str {
                "ERROR" => LogLevel::Error,
                "WARN" => LogLevel::Warn,
                "INFO" => LogLevel::Info,
                "DEBUG" => LogLevel::Debug,
                "TRACE" => LogLevel::Trace,
                _ => LogLevel::Unknown,
            };

            return ParsedLogLine {
                timestamp: iso_timestamp,
                service,
                module_path,
                module_short,
                level,
                message,
                raw_line,
            };
        }

        // Final fallback: Just extract timestamp if present
        let simple_timestamp_regex = Regex::new(
            r"^(\d{4}-\d{2}-\d{2}[T\s]\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})?)"
        ).unwrap();

        if let Some(ts_match) = simple_timestamp_regex.find(rest) {
            let timestamp = ts_match.as_str().to_string();
            let after_ts = rest[ts_match.end()..].trim();
            let level = LogLevel::from_str(after_ts);

            return ParsedLogLine {
                timestamp,
                service,
                module_path: String::new(),
                module_short: String::new(),
                level,
                message: after_ts.to_string(),
                raw_line,
            };
        }

        // No timestamp found, treat everything as message
        return ParsedLogLine {
            timestamp: String::new(),
            service,
            module_path: String::new(),
            module_short: String::new(),
            level: LogLevel::from_str(rest),
            message: rest.to_string(),
            raw_line,
        };
    }

    // No pipe separator found, treat whole line as unparsed
    ParsedLogLine {
        timestamp: String::new(),
        service: String::new(),
        module_path: String::new(),
        module_short: String::new(),
        level: LogLevel::Unknown,
        message: line.to_string(),
        raw_line,
    }
}
