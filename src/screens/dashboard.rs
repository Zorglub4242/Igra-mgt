/// Main dashboard screen

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, List, ListItem, Paragraph, Row, Table, Wrap},
    Frame,
};

use crate::app::{Screen, SystemResources};
use crate::core::docker::{ContainerInfo, ContainerStats};
use crate::core::wallet::WalletInfo;
use crate::core::ssl::CertificateInfo;
use crate::core::reth_metrics::RethMetrics;
use crate::core::l2_monitor::{Statistics, TransactionInfo, TransactionType};
use crate::screens::watch::TransactionFilter;
use std::collections::HashMap;

/// Parsed Docker Compose log line components
#[derive(Debug, Clone)]
struct ParsedLogLine {
    timestamp: String,      // Full timestamp: "2025-10-21T08:48:40Z"
    service: String,        // Service name from Docker: "viaduct"
    module_path: String,    // Rust module path: "viaduct::uni_storage"
    module_short: String,   // Last segment: "uni_storage"
    level: LogLevel,        // Detected log level
    message: String,        // Clean message without bracketed prefix
    raw_line: String,       // Original line for fallback
}

/// Log level enum for consistent handling
#[derive(Debug, Clone, PartialEq)]
enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
    Unknown,
}

impl LogLevel {
    fn from_str(s: &str) -> Self {
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

    fn to_string(&self) -> &str {
        match self {
            LogLevel::Error => "ERROR",
            LogLevel::Warn => "WARN ",
            LogLevel::Info => "INFO ",
            LogLevel::Debug => "DEBUG",
            LogLevel::Trace => "TRACE",
            LogLevel::Unknown => "     ",
        }
    }

    fn color(&self) -> Color {
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

/// Parse a Docker Compose log line into components
/// Handles Rust env_logger format: "service | [YYYY-MM-DDTHH:MM:SSZ LEVEL module::path] message"
/// Strip ANSI escape codes from a string
fn strip_ansi_codes(s: &str) -> String {
    // Match ANSI escape sequences: ESC [ ... m
    let ansi_regex = regex::Regex::new(r"\x1b\[[0-9;]*m").unwrap();
    ansi_regex.replace_all(s, "").to_string()
}

fn parse_docker_log_line(line: &str) -> ParsedLogLine {
    let raw_line = line.to_string();

    // Try to split on first pipe (service separator)
    if let Some(pipe_idx) = line.find('|') {
        let service = line[..pipe_idx].trim().to_string();
        let rest_with_ansi = line[pipe_idx + 1..].trim();

        // Strip ANSI color codes that docker adds
        let rest_cleaned = strip_ansi_codes(rest_with_ansi);
        let rest = rest_cleaned.as_str();

        // Try kaspad format: "YYYY-MM-DD HH:MM:SS.sss+TZ [LEVEL ] message"
        let kaspad_regex = regex::Regex::new(
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
        let bracketed_regex = regex::Regex::new(
            r"^\[(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z?)\s+(ERROR|WARN|INFO|DEBUG|TRACE)\s+([^\]]+)\]\s*(.*)$"
        ).unwrap();

        if let Some(caps) = bracketed_regex.captures(rest) {
            let timestamp = caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
            let level_str = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            let mut module_path = caps.get(3).map(|m| m.as_str().trim().to_string()).unwrap_or_default();
            let mut message = caps.get(4).map(|m| m.as_str().to_string()).unwrap_or_default();

            // Handle block-builder format: "module::path: src/file.rs:line"
            // The file path is in the module_path capture, but the actual message is already in the message capture
            // We just need to strip the file path from module_path
            if let Some(colon_pos) = module_path.find(": ") {
                // Check if what follows the colon looks like a file path
                let after_colon = &module_path[colon_pos + 2..];
                if after_colon.starts_with("src/") || after_colon.starts_with("/") {
                    // Strip everything from the colon onwards (the file path annotation)
                    module_path = module_path[..colon_pos].trim().to_string();
                }
            }

            // Extract short module name (last segment after ::)
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
        // This is what block-builder outputs
        let nonbracketed_regex = regex::Regex::new(
            r"^(\d{2}:\d{2}:\d{2}(?:\.\d+)?)\s+(ERROR|WARN|INFO|DEBUG|TRACE)\s+(.+)$"
        ).unwrap();

        if let Some(caps) = nonbracketed_regex.captures(rest) {
            let time_only = caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
            let level_str = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            let remainder = caps.get(3).map(|m| m.as_str().trim()).unwrap_or("");

            // Parse remainder: "module::path: src/file.rs:line: message"
            let (module_path, message) = if let Some(colon_pos) = remainder.find(": ") {
                let before_colon = &remainder[..colon_pos];
                let after_colon = &remainder[colon_pos + 2..];

                // Check if this looks like "module: src/file" pattern
                if after_colon.starts_with("src/") || after_colon.starts_with("/") {
                    // Find the next ": " which separates file path from message
                    if let Some(msg_pos) = after_colon.find(": ") {
                        (before_colon.to_string(), after_colon[msg_pos + 2..].to_string())
                    } else {
                        (before_colon.to_string(), String::new())
                    }
                } else {
                    // It's "module: message" format
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
        // Example: "2025-10-21T10:37:06.342076Z  INFO reth_node_events::node: Canonical chain committed"
        let iso_format_regex = regex::Regex::new(
            r"^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z?)\s+(ERROR|WARN|INFO|DEBUG|TRACE)\s+(.+)$"
        ).unwrap();

        if let Some(caps) = iso_format_regex.captures(rest) {
            let iso_timestamp = caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
            let level_str = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            let remainder = caps.get(3).map(|m| m.as_str().trim()).unwrap_or("");

            // Parse remainder: "module::path: message"
            // Note: We already captured the LEVEL, so remainder should be "module: message"
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
        let simple_timestamp_regex = regex::Regex::new(
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
        ParsedLogLine {
            timestamp: String::new(),
            service,
            module_path: String::new(),
            module_short: String::new(),
            level: LogLevel::from_str(rest),
            message: rest.to_string(),
            raw_line,
        }
    } else {
        // No pipe separator, raw log line
        ParsedLogLine {
            timestamp: String::new(),
            service: String::new(),
            module_path: String::new(),
            module_short: String::new(),
            level: LogLevel::from_str(line),
            message: line.to_string(),
            raw_line,
        }
    }
}

/// Format timestamp for compact display (HH:MM:SS)
fn format_timestamp_compact(timestamp: &str) -> String {
    // Handle ISO 8601 format: "2025-10-21T10:28:44.123Z" -> "10:28:44"
    if let Some(t_idx) = timestamp.find('T') {
        let time_part = &timestamp[t_idx + 1..];
        // Get HH:MM:SS, strip milliseconds and timezone
        if let Some(dot_idx) = time_part.find('.') {
            time_part[..dot_idx].to_string()
        } else if let Some(z_idx) = time_part.find('Z') {
            time_part[..z_idx].to_string()
        } else {
            // Take first 8 chars (HH:MM:SS)
            time_part.chars().take(8).collect()
        }
    } else if timestamp.contains(' ') {
        // Handle kaspad format: "2025-10-21 10:55:19.338+00:00" -> "10:55:19"
        let parts: Vec<&str> = timestamp.split_whitespace().collect();
        if parts.len() >= 2 {
            let time_part = parts[1];
            // Strip milliseconds if present
            if let Some(dot_idx) = time_part.find('.') {
                time_part[..dot_idx].to_string()
            } else {
                time_part.chars().take(8).collect()
            }
        } else {
            timestamp.to_string()
        }
    } else {
        timestamp.to_string()
    }
}

/// Log group for displaying consecutive logs with same level/module
#[derive(Debug, Clone)]
struct LogGroup {
    level: LogLevel,
    module: String,        // Short module name
    logs: Vec<ParsedLogLine>,
}

/// Group consecutive logs by level and module
fn group_logs_by_level_module(logs: Vec<ParsedLogLine>) -> Vec<LogGroup> {
    let mut groups: Vec<LogGroup> = Vec::new();

    for log in logs {
        // Check if we can add to current group (same level and module)
        let can_add_to_current = groups.last().map(|g| {
            g.level == log.level && g.module == log.module_short
        }).unwrap_or(false);

        if can_add_to_current {
            // Add to existing group
            if let Some(group) = groups.last_mut() {
                group.logs.push(log);
            }
        } else {
            // Start new group
            groups.push(LogGroup {
                level: log.level.clone(),
                module: log.module_short.clone(),
                logs: vec![log],
            });
        }
    }

    groups
}

pub struct Dashboard {
    pub title: String,
    // Services data
    containers: Vec<ContainerInfo>,
    container_stats: HashMap<String, ContainerStats>,
    image_versions: HashMap<String, crate::core::versions::ImageVersion>,
    profiles: Vec<String>,
    // Profiles data (active profiles)
    active_profiles: Vec<String>,
    // Wallets data
    wallets: Vec<WalletInfo>,
    // RPC data
    rpc_tokens: Vec<(usize, Option<String>)>,
    rpc_domain: String,
    // Config data
    config_data: Vec<(String, String)>,
    // SSL data
    ssl_cert_info: Option<CertificateInfo>,
    // Network name (testnet/mainnet) for proper currency labeling
    network: String,
}

impl Dashboard {
    pub fn new() -> Self {
        Self {
            title: "IGRA Orchestra Dashboard".to_string(),
            containers: Vec::new(),
            container_stats: HashMap::new(),
            image_versions: HashMap::new(),
            profiles: Vec::new(),
            active_profiles: Vec::new(),
            wallets: Vec::new(),
            rpc_tokens: Vec::new(),
            rpc_domain: String::new(),
            config_data: Vec::new(),
            ssl_cert_info: None,
            network: "testnet".to_string(),
        }
    }

    pub fn update_services(&mut self, containers: Vec<ContainerInfo>, profiles: Vec<String>, stats: HashMap<String, ContainerStats>, versions: HashMap<String, crate::core::versions::ImageVersion>) {
        self.containers = containers;
        self.profiles = profiles;
        self.container_stats = stats;
        self.image_versions = versions;
    }

    pub fn update_profiles(&mut self, active_profiles: Vec<String>) {
        self.active_profiles = active_profiles;
    }

    pub fn update_wallets(&mut self, wallets: Vec<WalletInfo>) {
        self.wallets = wallets;
    }

    pub fn update_rpc_tokens(&mut self, tokens: Vec<(usize, Option<String>)>, domain: String) {
        self.rpc_tokens = tokens;
        self.rpc_domain = domain;
    }

    pub fn update_config(&mut self, config: Vec<(String, String)>) {
        self.config_data = config;
    }

    pub fn update_ssl(&mut self, cert_info: Option<CertificateInfo>) {
        self.ssl_cert_info = cert_info;
    }

    pub fn update_network(&mut self, network: String) {
        self.network = network;
    }

    pub fn render(&self, frame: &mut Frame, current_screen: Screen, services_view: crate::app::ServicesView, config_section: crate::app::ConfigSection, selected_index: usize, status_message: Option<&str>, edit_mode: bool, edit_buffer: &str, detail_container: Option<&ContainerInfo>, detail_logs: &[String], system_resources: &SystemResources, show_help: bool, logs_selected_service: Option<&str>, logs_data: &[String], logs_follow_mode: bool, logs_compact_mode: bool, logs_live_mode: bool, logs_grouping_enabled: bool, logs_filter: Option<&str>, logs_scroll_offset: usize, containers: &[ContainerInfo], search_mode: bool, search_buffer: &str, filtered_indices: &[usize], show_send_dialog: bool, send_amount: &str, send_address: &str, send_input_field: usize, send_use_wallet_selector: bool, send_selected_wallet_index: usize, send_source_address: &str, wallets: &[crate::core::wallet::WalletInfo], reth_metrics: Option<&RethMetrics>, detail_wallet: Option<&WalletInfo>, detail_wallet_addresses: &[(String, f64, f64)], detail_wallet_utxos: &[crate::core::wallet::UtxoInfo], detail_wallet_scroll: usize, show_tx_detail: bool, selected_tx_index: Option<usize>, tx_search_mode: bool, tx_search_buffer: &str, filtered_tx_indices: &[usize], watch_stats: Option<&Statistics>, watch_transactions: &[TransactionInfo], watch_filter: &TransactionFilter, watch_scroll_offset: usize) {
        // If showing wallet detail view, render that instead
        if let Some(wallet) = detail_wallet {
            self.render_wallet_detail(frame, wallet, detail_wallet_addresses, detail_wallet_utxos, status_message, detail_wallet_scroll, tx_search_mode, tx_search_buffer, filtered_tx_indices, selected_tx_index);
            // Show transaction detail modal if requested
            if show_tx_detail {
                if let Some(tx_idx) = selected_tx_index {
                    if tx_idx < detail_wallet_utxos.len() {
                        self.render_transaction_detail_modal(frame, &detail_wallet_utxos[tx_idx]);
                    }
                }
            }
            if show_help {
                self.render_help(frame, current_screen);
            }
            return;
        }

        // If showing service detail view, render that instead
        if let Some(container) = detail_container {
            self.render_service_detail(frame, container, detail_logs, status_message, reth_metrics);
            // Still show help overlay if requested
            if show_help {
                self.render_help(frame, current_screen);
            }
            return;
        }
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5),  // Title (expanded for system info)
                Constraint::Length(3),  // Menu
                Constraint::Min(0),     // Content
                Constraint::Length(3),  // Footer
            ])
            .split(frame.size());

        // Title with system resources
        let memory_percent = if system_resources.memory_total_gb > 0.0 {
            (system_resources.memory_used_gb / system_resources.memory_total_gb * 100.0).round()
        } else {
            0.0
        };

        // Color-coded CPU
        let cpu_color = if system_resources.cpu_percent > 80.0 {
            Color::Red
        } else if system_resources.cpu_percent > 60.0 {
            Color::Yellow
        } else {
            Color::Gray
        };

        // Color-coded Memory
        let mem_color = if memory_percent > 80.0 {
            Color::Red
        } else if memory_percent > 60.0 {
            Color::Yellow
        } else {
            Color::Gray
        };

        // Color-coded Disk (warn if less than 10% free or < 10GB)
        let disk_percent_free = if system_resources.disk_total_gb > 0.0 {
            (system_resources.disk_free_gb / system_resources.disk_total_gb * 100.0).round()
        } else {
            100.0
        };
        let disk_color = if disk_percent_free < 10.0 || system_resources.disk_free_gb < 10.0 {
            Color::Red
        } else if disk_percent_free < 20.0 || system_resources.disk_free_gb < 20.0 {
            Color::Yellow
        } else {
            Color::Gray
        };

        // Line 1: Title + CPU/Mem/Disk usage
        let title_line = Line::from(vec![
            Span::styled(
                &self.title,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled("CPU: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{:.1}%", system_resources.cpu_percent),
                Style::default().fg(cpu_color).add_modifier(Modifier::BOLD)
            ),
            Span::raw(" | "),
            Span::styled("Mem: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{:.1}%", memory_percent),
                Style::default().fg(mem_color).add_modifier(Modifier::BOLD)
            ),
            Span::styled(
                format!(" ({:.1}/{:.1} GB)", system_resources.memory_used_gb, system_resources.memory_total_gb),
                Style::default().fg(Color::Gray)
            ),
            Span::raw(" | "),
            Span::styled("Disk: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{:.1}/{:.1} GB Free", system_resources.disk_free_gb, system_resources.disk_total_gb),
                Style::default().fg(disk_color).add_modifier(Modifier::BOLD)
            ),
        ]);

        // Line 2: OS and System info
        let os_line = Line::from(vec![
            Span::styled("OS: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{} {}", system_resources.os_name, system_resources.os_version),
                Style::default().fg(Color::White)
            ),
            Span::raw(" | "),
            Span::styled("CPU: ", Style::default().fg(Color::Gray)),
            Span::styled(
                if system_resources.cpu_frequency_ghz > 0.0 {
                    format!("{} Cores @{:.2} GHz", system_resources.cpu_cores, system_resources.cpu_frequency_ghz)
                } else {
                    format!("{} Cores", system_resources.cpu_cores)
                },
                Style::default().fg(Color::White)
            ),
            Span::raw("  "),
            Span::styled(
                &system_resources.cpu_model,
                Style::default().fg(Color::DarkGray)
            ),
        ]);

        // Line 3: Public IP
        let ip_line = if let Some(ref ip) = system_resources.public_ip {
            Line::from(vec![
                Span::styled("Public IP: ", Style::default().fg(Color::Gray)),
                Span::styled(ip, Style::default().fg(Color::Cyan)),
            ])
        } else {
            Line::from(vec![
                Span::styled("Public IP: ", Style::default().fg(Color::Gray)),
                Span::styled("Fetching...", Style::default().fg(Color::DarkGray)),
            ])
        };

        let title = Paragraph::new(vec![title_line, os_line, ip_line])
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));

        frame.render_widget(title, chunks[0]);

        // Menu bar
        let menu_items: Vec<Span> = Screen::all()
            .iter()
            .enumerate()
            .flat_map(|(i, screen)| {
                let is_current = *screen == current_screen;
                let style = if is_current {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                vec![
                    Span::styled(format!(" [{}] {} ", i + 1, screen.title()), style),
                    Span::raw("  "),
                ]
            })
            .collect();

        let menu = Paragraph::new(Line::from(menu_items))
            .block(Block::default().borders(Borders::ALL));

        frame.render_widget(menu, chunks[1]);

        // Content area - render based on current screen
        match current_screen {
            Screen::Services => self.render_services(frame, chunks[2], services_view, selected_index, filtered_indices),
            Screen::Wallets => self.render_wallets(frame, chunks[2], selected_index, filtered_indices),
            Screen::Watch => self.render_watch(frame, chunks[2], watch_stats, watch_transactions, watch_filter, watch_scroll_offset),
            Screen::Logs => self.render_logs(frame, chunks[2], logs_selected_service, logs_data, logs_follow_mode, logs_compact_mode, logs_live_mode, logs_grouping_enabled, logs_filter, logs_scroll_offset, selected_index, containers),
            Screen::Config => self.render_config(frame, chunks[2], config_section, selected_index, edit_mode, edit_buffer, filtered_indices),
        }

        // Footer with status message or help
        let footer_text = if let Some(status) = status_message {
            status.to_string()
        } else if search_mode {
            format!("Search: {} | [Enter] Apply | [Esc] Cancel", search_buffer)
        } else if edit_mode {
            "Editing config value - Type to edit | [Enter] Save | [Esc] Cancel".to_string()
        } else {
            match current_screen {
                Screen::Services => "[Tab] Switch view | [← →] Next screen | [↑↓] Select | [Enter] Details | [s]tart | [x]top | [R]estart | [q]uit".to_string(),
                Screen::Wallets => "[← →] Next screen | [↑↓] Select | [Enter] Info | [g]enerate | [t]ransfer | [/] Search | [r]efresh | [?] Help | [q]uit".to_string(),
                Screen::Watch => "[← →] Next screen | [↑↓] Scroll | [f] Filter | [r] Record | [?] Help | [q]uit".to_string(),
                Screen::Config => "[Tab] Switch tab | [← →] Next screen | [↑↓] Select | [e]dit | [g]enerate | [c]heck | [n]ew cert | [q]uit".to_string(),
                Screen::Logs => {
                    if logs_selected_service.is_some() {
                        "[← →] Next screen | [↑↓/PgUp/PgDn] Scroll | [l]ive | [f]ollow | [e]rror [w]arn [i]nfo [c]lear | [Esc] back".to_string()
                    } else {
                        "[← →] Next screen | [↑↓] Select | [Enter] View logs | [?] Help | [q]uit".to_string()
                    }
                }
            }
        };

        let footer = Paragraph::new(footer_text)
            .alignment(Alignment::Center)
            .style(if status_message.is_some() {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            })
            .block(Block::default().borders(Borders::ALL));

        frame.render_widget(footer, chunks[3]);

        // Show help overlay if requested
        if show_help {
            self.render_help(frame, current_screen);
        }

        // Show send transaction dialog if requested
        if show_send_dialog {
            self.render_send_dialog(frame, send_amount, send_address, send_input_field, send_use_wallet_selector, send_selected_wallet_index, send_source_address, wallets);
        }
    }

    /// Render a tab bar showing available sub-views with the active one highlighted
    fn render_tab_bar(&self, tabs: &[(&str, bool)]) -> Paragraph {
        let mut tab_spans = Vec::new();

        for (i, (tab_name, is_active)) in tabs.iter().enumerate() {
            if i > 0 {
                tab_spans.push(Span::raw(" "));
            }

            if *is_active {
                // Active tab: inverted colors
                tab_spans.push(Span::styled(
                    format!(" {} ", tab_name),
                    Style::default()
                        .bg(Color::Blue)
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ));
            } else {
                // Inactive tab: normal style with brackets
                tab_spans.push(Span::styled(
                    format!("[{}]", tab_name),
                    Style::default().fg(Color::Gray),
                ));
            }
        }

        tab_spans.push(Span::styled(
            "  Tab to switch",
            Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
        ));

        Paragraph::new(Line::from(tab_spans))
            .alignment(Alignment::Left)
    }

    fn render_services(&self, frame: &mut Frame, area: ratatui::layout::Rect, services_view: crate::app::ServicesView, selected_index: usize, filtered_indices: &[usize]) {
        use crate::app::ServicesView;

        // Split area to add tab bar
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(area);

        // Render tab bar
        let tabs = [
            ("Services", services_view == ServicesView::Services),
            ("Profiles", services_view == ServicesView::Profiles),
        ];
        let tab_bar = self.render_tab_bar(&tabs);
        frame.render_widget(tab_bar, chunks[0]);

        // Delegate to appropriate view based on services_view
        match services_view {
            ServicesView::Services => self.render_services_table(frame, chunks[1], selected_index, filtered_indices),
            ServicesView::Profiles => self.render_profiles(frame, chunks[1], selected_index),
        }
    }

    fn render_services_table(&self, frame: &mut Frame, area: ratatui::layout::Rect, selected_index: usize, filtered_indices: &[usize]) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(4), Constraint::Min(0)])
            .split(area);

        // Summary
        let profiles_text = if self.profiles.is_empty() {
            "No active profiles".to_string()
        } else {
            format!("Active: {}", self.profiles.join(", "))
        };

        let total_services = self.containers.len();
        let running_services = self.containers.iter().filter(|c| c.status.contains("Up")).count();

        let summary = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("Services: ", Style::default().fg(Color::White)),
                Span::styled(
                    format!("{}/{} running", running_services, total_services),
                    if running_services == total_services {
                        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                    }
                ),
                Span::raw("  |  "),
                Span::styled(profiles_text, Style::default().fg(Color::Cyan)),
            ]),
        ])
        .block(Block::default().borders(Borders::ALL).title("Status"));

        frame.render_widget(summary, chunks[0]);

        // Services table
        let header = Row::new(vec!["Service", "Status", "Metrics", "Ports", "CPU", "Memory", "Storage", "Image:Tag"])
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .bottom_margin(1);

        let rows: Vec<Row> = self.containers.iter().enumerate().map(|(idx, container)| {
            let is_selected = idx == selected_index;
            let is_filtered = !filtered_indices.is_empty() && filtered_indices.contains(&idx);

            let status_color = if container.status.contains("Up") {
                Color::Green
            } else {
                Color::Red
            };

            let _health_color = match container.health.as_deref() {
                Some("healthy") => Color::Green,
                Some("unhealthy") => Color::Red,
                Some("starting") => Color::Yellow,
                _ => Color::Gray,
            };

            let name = container.name.clone();
            let status = container.status.clone();
            let _health = container.health.as_deref().unwrap_or("N/A").to_string();

            // Get stats if available with color-coding
            let (cpu_cell, mem_cell, storage_cell, _net_rx_text, _net_tx_text) = if let Some(stats) = self.container_stats.get(&container.name) {
                // CPU with color coding
                let cpu_percent = stats.cpu_percent;
                let cpu_color = if cpu_percent > 80.0 {
                    Color::Red
                } else if cpu_percent > 60.0 {
                    Color::Yellow
                } else {
                    Color::White
                };
                let cpu_cell = Cell::from(Span::styled(
                    format!("{:.1}%", cpu_percent),
                    Style::default().fg(cpu_color)
                ));

                // Memory with color coding
                let mem_mb = stats.memory_usage / 1024 / 1024;
                let mem_percent = if stats.memory_limit > 0 {
                    (stats.memory_usage as f64 / stats.memory_limit as f64 * 100.0)
                } else {
                    0.0
                };
                let mem_color = if mem_percent > 80.0 {
                    Color::Red
                } else if mem_percent > 60.0 {
                    Color::Yellow
                } else {
                    Color::White
                };
                let mem_cell = Cell::from(Span::styled(
                    format!("{} MB", mem_mb),
                    Style::default().fg(mem_color)
                ));

                // Storage with color coding
                let container_mb = stats.container_size / 1024 / 1024;
                let volume_gb = stats.volume_size as f64 / 1024.0 / 1024.0 / 1024.0;

                let storage_color = if volume_gb > 50.0 {
                    Color::Red
                } else if volume_gb > 20.0 {
                    Color::Yellow
                } else {
                    Color::White
                };

                let storage_cell = Cell::from(Span::styled(
                    if volume_gb > 0.0 {
                        format!("{}M/{}G", container_mb, volume_gb as u64)
                    } else {
                        format!("{} MB", container_mb)
                    },
                    Style::default().fg(storage_color)
                ));

                // Format network I/O
                let rx = Self::format_bytes(stats.network_rx);
                let tx = Self::format_bytes(stats.network_tx);

                (cpu_cell, mem_cell, storage_cell, rx, tx)
            } else {
                (
                    Cell::from(Span::styled("N/A", Style::default().fg(Color::Gray))),
                    Cell::from(Span::styled("N/A", Style::default().fg(Color::Gray))),
                    Cell::from(Span::styled("N/A", Style::default().fg(Color::Gray))),
                    "N/A".to_string(),
                    "N/A".to_string()
                )
            };

            // Extract image name and tag
            let image_str = container.image
                .split('/')
                .last()
                .unwrap_or(&container.image);

            let (image_name, current_tag) = if let Some((name, tag)) = image_str.split_once(':') {
                (name, tag)
            } else {
                (image_str, "latest")
            };

            // Check for version info and update availability
            let (image_display, image_color) = if let Some(version_info) = self.image_versions.get(image_name) {
                if version_info.update_available {
                    if let Some(ref latest) = version_info.latest {
                        (
                            format!("{}:{} → {} 🔄", image_name, current_tag, latest),
                            Color::Yellow
                        )
                    } else {
                        (format!("{}:{}", image_name, current_tag), Color::White)
                    }
                } else {
                    (format!("{}:{} ✓", image_name, current_tag), Color::Green)
                }
            } else {
                (format!("{}:{}", image_name, current_tag), Color::White)
            };

            // Format ports
            let ports_text = if container.ports.is_empty() {
                "-".to_string()
            } else {
                container.ports.join(", ")
            };

            // Format metrics from log parsing
            let metrics_text = if let Some(ref status_text) = container.metrics.status_text {
                let mut parts = vec![status_text.clone()];
                if let Some(ref primary) = container.metrics.primary_metric {
                    parts.push(primary.clone());
                }
                parts.join(" ")
            } else {
                "-".to_string()
            };

            let metrics_color = if container.metrics.is_healthy {
                Color::Green
            } else {
                Color::Yellow
            };

            let row = Row::new(vec![
                Cell::from(name),
                Cell::from(Span::styled(status, Style::default().fg(status_color))),
                Cell::from(Span::styled(metrics_text, Style::default().fg(metrics_color))),
                Cell::from(ports_text),
                cpu_cell,
                mem_cell,
                storage_cell,
                Cell::from(Span::styled(image_display, Style::default().fg(image_color))),
            ]);

            if is_selected {
                row.style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
            } else if is_filtered {
                row.style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            } else {
                row
            }
        }).collect();

        let table = Table::new(
            rows,
            [
                Constraint::Length(22),  // Service
                Constraint::Length(12),  // Status
                Constraint::Length(22),  // Metrics
                Constraint::Length(16),  // Ports
                Constraint::Length(7),   // CPU
                Constraint::Length(10),  // Memory
                Constraint::Length(12),  // Storage
                Constraint::Min(15),     // Image:Tag
            ],
        )
        .header(header)
        .block(Block::default().borders(Borders::ALL).title("Services"));

        frame.render_widget(table, chunks[1]);
    }

    fn format_bytes(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = 1024 * KB;
        const GB: u64 = 1024 * MB;

        if bytes >= GB {
            format!("{:.1} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.1} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.1} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} B", bytes)
        }
    }

    fn render_profiles(&self, frame: &mut Frame, area: ratatui::layout::Rect, selected_index: usize) {
        let all_profiles = vec![
            ("kaspad", "Kaspad consensus node"),
            ("backend", "Execution layer + Block builder + Viaduct"),
            ("frontend-w1", "1 RPC provider + 1 Wallet worker"),
            ("frontend-w2", "2 RPC providers + 2 Wallet workers"),
            ("frontend-w3", "3 RPC providers + 3 Wallet workers"),
            ("frontend-w4", "4 RPC providers + 4 Wallet workers"),
            ("frontend-w5", "5 RPC providers + 5 Wallet workers"),
        ];

        let header = Row::new(vec!["Profile", "Description", "Status"])
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .bottom_margin(1);

        let rows: Vec<Row> = all_profiles.iter().enumerate().map(|(idx, (profile, description))| {
            let is_selected = idx == selected_index;
            let is_active = self.active_profiles.contains(&profile.to_string());

            let status = if is_active {
                ("Running", Color::Green)
            } else {
                ("Stopped", Color::Gray)
            };

            let row = Row::new(vec![
                Cell::from(*profile),
                Cell::from(*description),
                Cell::from(Span::styled(status.0, Style::default().fg(status.1))),
            ]);

            if is_selected {
                row.style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
            } else {
                row
            }
        }).collect();

        let table = Table::new(
            rows,
            [
                Constraint::Length(15),
                Constraint::Min(35),
                Constraint::Length(12),
            ],
        )
        .header(header)
        .block(Block::default().borders(Borders::ALL).title("Docker Compose Profiles"));

        frame.render_widget(table, area);
    }

    fn render_wallets(&self, frame: &mut Frame, area: ratatui::layout::Rect, selected_index: usize, filtered_indices: &[usize]) {
        // Determine currency label based on network
        let currency = if self.network == "mainnet" { "KAS" } else { "TKAS" };

        let header = Row::new(vec!["Worker", "Status", "Address", "Balance", "Fees Spent"])
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .bottom_margin(1);

        let rows: Vec<Row> = self.wallets.iter().enumerate().map(|(idx, wallet)| {
            let is_selected = idx == selected_index;
            let is_filtered = !filtered_indices.is_empty() && filtered_indices.contains(&idx);

            let status = if wallet.container_running {
                ("Running", Color::Green)
            } else {
                ("Stopped", Color::Red)
            };

            let address = wallet.address.as_deref().unwrap_or("Not generated");
            let balance = wallet
                .balance
                .map(|b| format!("{:.8} {}", b, currency))
                .unwrap_or_else(|| "N/A".to_string());

            // Format fees spent with color coding
            let (fees_text, fees_color) = if let Some(fees) = wallet.fees_spent {
                let color = if fees == 0.0 {
                    Color::Green
                } else if fees < 5.0 {
                    Color::Yellow
                } else {
                    Color::Red
                };
                (format!("{:.8} {}", fees, currency), color)
            } else {
                ("N/A".to_string(), Color::Gray)
            };

            let row = Row::new(vec![
                Cell::from(format!("Worker {}", wallet.worker_id)),
                Cell::from(Span::styled(status.0, Style::default().fg(status.1))),
                Cell::from(address),
                Cell::from(balance),
                Cell::from(Span::styled(fees_text, Style::default().fg(fees_color))),
            ]);

            if is_selected {
                row.style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
            } else if is_filtered {
                row.style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            } else {
                row
            }
        }).collect();

        let table = Table::new(
            rows,
            [
                Constraint::Length(10),
                Constraint::Length(10),
                Constraint::Min(30),
                Constraint::Length(20),
                Constraint::Length(20),
            ],
        )
        .header(header)
        .block(Block::default().borders(Borders::ALL).title("Wallets"));

        frame.render_widget(table, area);
    }

    fn render_watch(&self, frame: &mut Frame, area: ratatui::layout::Rect, stats: Option<&Statistics>, transactions: &[TransactionInfo], filter: &TransactionFilter, _scroll_offset: usize) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5),  // Stats header
                Constraint::Min(0),     // Transaction list
            ])
            .split(area);

        // Statistics header
        if let Some(stats) = stats {
            let stats_text = vec![
                Line::from(vec![
                    Span::styled("Block: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        format!("#{}", stats.current_block),
                        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("  │  "),
                    Span::styled("TPS: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        format!("{:.2}", stats.tps()),
                        Style::default().fg(Color::Yellow),
                    ),
                    Span::raw("  │  "),
                    Span::styled("Uptime: ", Style::default().fg(Color::Gray)),
                    Span::styled(stats.uptime(), Style::default().fg(Color::Blue)),
                ]),
                Line::from(vec![
                    Span::styled("Total: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        format!("{}", stats.total_transactions),
                        Style::default().fg(Color::White),
                    ),
                    Span::raw("  │  "),
                    Span::styled("Success: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        format!("{}", stats.successful_transactions),
                        Style::default().fg(Color::Green),
                    ),
                    Span::raw("  │  "),
                    Span::styled("Failed: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        format!("{}", stats.failed_transactions),
                        Style::default().fg(Color::Red),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("L2 Fees: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        format!("{:.4} iKAS", stats.total_gas_fees_ikas),
                        Style::default().fg(Color::Yellow),
                    ),
                    Span::raw("  │  "),
                    Span::styled("L1 Fees: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        format!("{:.6} KAS", stats.total_l1_fees_kas),
                        Style::default().fg(Color::Magenta),
                    ),
                    Span::raw(" (node wallet)"),
                ]),
            ];

            let stats_block = Paragraph::new(stats_text)
                .block(Block::default().borders(Borders::ALL).title("Statistics"))
                .wrap(Wrap { trim: false });
            frame.render_widget(stats_block, chunks[0]);
        } else {
            let connecting = Paragraph::new("Connecting to L2 node...")
                .block(Block::default().borders(Borders::ALL).title("Statistics"))
                .style(Style::default().fg(Color::Yellow));
            frame.render_widget(connecting, chunks[0]);
        }

        // Transaction list
        let filtered_txs: Vec<&TransactionInfo> = transactions
            .iter()
            .filter(|tx| filter.matches(&tx.tx_type))
            .collect();

        let items: Vec<ListItem> = filtered_txs
            .iter()
            .map(|tx| {
                let type_color = match tx.tx_type {
                    TransactionType::Transfer => Color::White,
                    TransactionType::Contract => Color::Cyan,
                    TransactionType::Entry => Color::Blue,
                    TransactionType::Unknown => Color::Gray,
                };

                let status_symbol = if tx.status { "✓" } else { "✗" };
                let status_color = if tx.status { Color::Green } else { Color::Red };

                let mut lines = vec![
                    Line::from(vec![
                        Span::styled(
                            format!("[{}] ", tx.timestamp.format("%H:%M:%S")),
                            Style::default().fg(Color::Gray),
                        ),
                        Span::styled(
                            format!("{}", tx.tx_type),
                            Style::default().fg(type_color).add_modifier(Modifier::BOLD),
                        ),
                        Span::raw("  "),
                        Span::styled(status_symbol, Style::default().fg(status_color)),
                    ]),
                    Line::from(vec![
                        Span::raw("  Hash: "),
                        Span::styled(tx.hash.chars().take(16).collect::<String>() + "...", Style::default().fg(Color::Gray)),
                    ]),
                ];

                lines.push(Line::from(vec![
                    Span::raw("  Value: "),
                    Span::styled(
                        format!("{:.4} iKAS", tx.value_ikas()),
                        Style::default().fg(Color::Green),
                    ),
                    Span::raw("  │  Gas: "),
                    Span::styled(
                        format!("{:.6} iKAS", tx.gas_fee_ikas()),
                        Style::default().fg(Color::Magenta),
                    ),
                ]));

                if let Some(l1_fee) = tx.l1_fee {
                    lines.push(Line::from(vec![
                        Span::raw("  L1 Fee: "),
                        Span::styled(
                            format!("{:.6} KAS", l1_fee),
                            Style::default().fg(Color::Yellow),
                        ),
                    ]));
                }

                ListItem::new(lines)
            })
            .collect();

        let filter_text = match filter {
            TransactionFilter::All => "All",
            TransactionFilter::Transfer => "Transfer",
            TransactionFilter::Contract => "Contract",
            TransactionFilter::Entry => "Entry",
        };

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(format!("Transactions - Filter: {} ({} shown)", filter_text, filtered_txs.len())));

        frame.render_widget(list, chunks[1]);
    }

    fn render_rpc_tokens(&self, frame: &mut Frame, area: ratatui::layout::Rect, selected_index: usize) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        // Header info
        let info = Paragraph::new(format!("Domain: {} | Total Tokens: {}",
            self.rpc_domain, self.rpc_tokens.len()))
            .block(Block::default().borders(Borders::ALL).title("RPC Configuration"));

        frame.render_widget(info, chunks[0]);

        // Tokens table
        let header = Row::new(vec!["Token #", "Value", "Status"])
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .bottom_margin(1);

        let rows: Vec<Row> = self.rpc_tokens.iter().enumerate().map(|(idx, (i, token))| {
            let is_selected = idx == selected_index;

            let (value, status, color) = if let Some(t) = token {
                (t.clone(), "✓ Set", Color::Green)
            } else {
                ("<not set>".to_string(), "✗ Missing", Color::Red)
            };

            let row = Row::new(vec![
                Cell::from(format!("TOKEN_{:02}", i)),
                Cell::from(value),
                Cell::from(Span::styled(status, Style::default().fg(color))),
            ]);

            if is_selected {
                row.style(Style::default().bg(Color::DarkGray).fg(Color::White))
            } else {
                row
            }
        }).collect();

        let table = Table::new(
            rows,
            [
                Constraint::Length(12),
                Constraint::Min(20),
                Constraint::Length(12),
            ],
        )
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(format!("RPC Tokens (Total: {})", self.rpc_tokens.len())));

        frame.render_widget(table, chunks[1]);
    }

    fn render_config(&self, frame: &mut Frame, area: ratatui::layout::Rect, config_section: crate::app::ConfigSection, selected_index: usize, edit_mode: bool, edit_buffer: &str, filtered_indices: &[usize]) {
        use crate::app::ConfigSection;

        // Split area to add tab bar
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(area);

        // Render tab bar
        let tabs = [
            ("Environment", config_section == ConfigSection::Environment),
            ("RPC Tokens", config_section == ConfigSection::RpcTokens),
            ("SSL Certificates", config_section == ConfigSection::SslCerts),
        ];
        let tab_bar = self.render_tab_bar(&tabs);
        frame.render_widget(tab_bar, chunks[0]);

        // Delegate to appropriate tab based on config_section
        match config_section {
            ConfigSection::Environment => self.render_config_environment(frame, chunks[1], selected_index, edit_mode, edit_buffer, filtered_indices),
            ConfigSection::RpcTokens => self.render_rpc_tokens(frame, chunks[1], selected_index),
            ConfigSection::SslCerts => self.render_ssl(frame, chunks[1]),
        }
    }

    fn render_config_environment(&self, frame: &mut Frame, area: ratatui::layout::Rect, selected_index: usize, edit_mode: bool, edit_buffer: &str, filtered_indices: &[usize]) {
        let header = Row::new(vec!["Key", "Value"])
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .bottom_margin(1);

        let rows: Vec<Row> = self.config_data.iter().take(30).enumerate().map(|(idx, (key, value))| {
            let is_selected = idx == selected_index;
            let is_filtered = !filtered_indices.is_empty() && filtered_indices.contains(&idx);

            // If this is the selected row and we're in edit mode, show the edit buffer
            let display_value = if is_selected && edit_mode {
                edit_buffer.to_string()
            } else {
                // Mask sensitive values
                if key.contains("PASSWORD")
                    || key.contains("SECRET")
                    || key.contains("KEY")
                    || key.contains("TOKEN")
                {
                    "****".to_string()
                } else {
                    if value.len() > 50 {
                        format!("{}...", &value[..47])
                    } else {
                        value.clone()
                    }
                }
            };

            let row = Row::new(vec![
                Cell::from(key.clone()),
                Cell::from(display_value),
            ]);

            if is_selected {
                if edit_mode {
                    row.style(Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD))
                } else {
                    row.style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
                }
            } else if is_filtered {
                row.style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            } else {
                row
            }
        }).collect();

        let table = Table::new(
            rows,
            [
                Constraint::Length(35),
                Constraint::Min(20),
            ],
        )
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(format!("Configuration (showing 30 of {} keys)", self.config_data.len())));

        frame.render_widget(table, area);
    }

    fn render_ssl(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
        let mut text = vec![
            Line::from(Span::styled(
                "SSL Certificate Management",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
        ];

        // Show certificate information if available
        if let Some(cert_info) = &self.ssl_cert_info {
            text.push(Line::from(vec![
                Span::styled("Domain: ", Style::default().fg(Color::White)),
                Span::styled(&cert_info.domain, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            ]));
            text.push(Line::from(""));

            // Status
            let (status_text, status_color) = if cert_info.is_valid {
                ("✓ Valid", Color::Green)
            } else {
                ("✗ Invalid/Expired", Color::Red)
            };
            text.push(Line::from(vec![
                Span::styled("Status: ", Style::default().fg(Color::White)),
                Span::styled(status_text, Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
            ]));

            // Valid from
            if let Some(from) = cert_info.valid_from {
                text.push(Line::from(vec![
                    Span::styled("Valid From: ", Style::default().fg(Color::White)),
                    Span::styled(from.format("%Y-%m-%d %H:%M:%S UTC").to_string(), Style::default().fg(Color::Gray)),
                ]));
            }

            // Valid until
            if let Some(until) = cert_info.valid_until {
                text.push(Line::from(vec![
                    Span::styled("Valid Until: ", Style::default().fg(Color::White)),
                    Span::styled(until.format("%Y-%m-%d %H:%M:%S UTC").to_string(), Style::default().fg(Color::Gray)),
                ]));
            }

            // Days remaining
            if let Some(days) = cert_info.days_remaining {
                let days_color = if days > 30 {
                    Color::Green
                } else if days > 7 {
                    Color::Yellow
                } else {
                    Color::Red
                };
                text.push(Line::from(vec![
                    Span::styled("Days Remaining: ", Style::default().fg(Color::White)),
                    Span::styled(
                        format!("{} days", days),
                        Style::default().fg(days_color).add_modifier(Modifier::BOLD)
                    ),
                ]));
            }
        } else {
            text.push(Line::from(Span::styled(
                "No certificate information available",
                Style::default().fg(Color::Yellow),
            )));
            text.push(Line::from(""));
            text.push(Line::from("Press [c] to check certificate"));
        }

        text.push(Line::from(""));
        text.push(Line::from(""));
        text.push(Line::from(Span::styled(
            "Available Actions:",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )));
        text.push(Line::from("  [c] Check certificate"));
        text.push(Line::from("  [n] Force renewal (restart Traefik)"));
        text.push(Line::from("  [r] Refresh"));

        let paragraph = Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title("SSL/TLS Certificates"))
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }

    fn render_logs(&self, frame: &mut Frame, area: ratatui::layout::Rect, selected_service: Option<&str>, logs_data: &[String], follow_mode: bool, compact_mode: bool, live_mode: bool, grouping_enabled: bool, filter: Option<&str>, scroll_offset: usize, selected_index: usize, containers: &[ContainerInfo]) {
        // If no service selected, show service list
        if selected_service.is_none() {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(4), Constraint::Min(0)])
                .split(area);

            // Info
            let info_text = vec![
                Line::from(Span::styled("Interactive Log Viewer", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
                Line::from("Select a service to view its logs"),
            ];
            let info = Paragraph::new(info_text)
                .block(Block::default().borders(Borders::ALL).title("Info"));
            frame.render_widget(info, chunks[0]);

            // Service list
            let header = Row::new(vec!["Service", "Status"])
                .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                .bottom_margin(1);

            let rows: Vec<Row> = containers.iter().enumerate().map(|(idx, container)| {
                let is_selected = idx == selected_index;
                let status_color = if container.status.contains("Up") {
                    Color::Green
                } else {
                    Color::Red
                };

                let row = Row::new(vec![
                    Cell::from(container.name.clone()),
                    Cell::from(Span::styled(&container.status, Style::default().fg(status_color))),
                ]);

                if is_selected {
                    row.style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
                } else {
                    row
                }
            }).collect();

            let table = Table::new(
                rows,
                [Constraint::Length(25), Constraint::Min(20)],
            )
            .header(header)
            .block(Block::default().borders(Borders::ALL).title("Select Service"));

            frame.render_widget(table, chunks[1]);
        } else {
            // Show logs for selected service
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(0)])
                .split(area);

            // Header with service name and controls
            let filter_text = if let Some(f) = filter {
                format!(" | Filter: {}", f)
            } else {
                String::new()
            };
            let follow_text = if follow_mode { " | FOLLOW" } else { "" };
            let live_text = if live_mode { " | 🔴 LIVE" } else { "" };
            let view_text = if compact_mode { " | Compact" } else { " | Detailed" };
            let group_text = if grouping_enabled { " | Grouped" } else { "" };

            let header_text = format!(
                "Service: {} | Lines: {}{}{}{}{}{}",
                selected_service.unwrap_or("N/A"),
                logs_data.len(),
                filter_text,
                follow_text,
                live_text,
                view_text,
                group_text
            );

            let header = Paragraph::new(header_text)
                .style(Style::default().fg(Color::Cyan))
                .block(Block::default().borders(Borders::ALL).title("Log Viewer"));
            frame.render_widget(header, chunks[0]);

            // Parse all logs first
            let parsed_logs: Vec<ParsedLogLine> = logs_data
                .iter()
                .map(|line| parse_docker_log_line(line))
                .collect();

            // Filter logs if filter is set
            let filtered_logs: Vec<ParsedLogLine> = if let Some(filter_level) = filter {
                parsed_logs.into_iter().filter(|log| {
                    log.raw_line.to_uppercase().contains(filter_level)
                }).collect()
            } else {
                parsed_logs
            };

            // Group or render chronologically based on setting
            let mut visible_lines: Vec<Line> = Vec::new();
            let max_lines = chunks[1].height as usize - 2; // Account for borders
            let total_logs = filtered_logs.len(); // Save count before move

            if grouping_enabled {
                // Group logs by level/module
                let groups = group_logs_by_level_module(filtered_logs);

                let mut line_count = 0;
                let mut skip_count = scroll_offset;

                for group in groups {
                    if line_count >= max_lines {
                        break;
                    }

                    let logs_count = group.logs.len();
                    let level_text = group.level.to_string();
                    let level_color = group.level.color();

                    // Group header: [LEVEL] module:
                    if skip_count == 0 {
                        let module_text = if !group.module.is_empty() {
                            format!(" {}:", group.module)
                        } else {
                            String::new()
                        };

                        visible_lines.push(Line::from(vec![
                            Span::styled(
                                format!("[{}]", level_text),
                                Style::default()
                                    .fg(Color::Black)
                                    .bg(level_color)
                                    .add_modifier(Modifier::BOLD)
                            ),
                            Span::styled(module_text, Style::default().fg(Color::Gray)),
                        ]));
                        line_count += 1;
                    } else {
                        skip_count -= 1;
                    }

                    // Group items
                    for (idx, log) in group.logs.into_iter().enumerate() {
                        if line_count >= max_lines {
                            break;
                        }

                        if skip_count > 0 {
                            skip_count -= 1;
                            continue;
                        }

                        let time = if !log.timestamp.is_empty() {
                            format_timestamp_compact(&log.timestamp)
                        } else {
                            "        ".to_string()
                        };

                        let prefix = if idx == logs_count - 1 {
                            "  └─ "
                        } else {
                            "  ├─ "
                        };

                        let level_color = log.level.color();

                        visible_lines.push(Line::from(vec![
                            Span::styled(prefix, Style::default().fg(Color::DarkGray)),
                            Span::styled(time, Style::default().fg(Color::DarkGray)),
                            Span::raw(" "),
                            Span::styled(log.message.clone(), Style::default().fg(level_color)),
                        ]));
                        line_count += 1;
                    }
                }
            } else {
                // Chronological view (no grouping)
                for log in filtered_logs.iter().skip(scroll_offset).take(max_lines) {
                    if compact_mode {
                        // Compact: "HH:MM:SS [LEVEL] module: message"
                        let time = if !log.timestamp.is_empty() {
                            format_timestamp_compact(&log.timestamp)
                        } else {
                            "        ".to_string()
                        };

                        let level_text = log.level.to_string();
                        let level_color = log.level.color();
                        let module_text = if !log.module_short.is_empty() {
                            format!(" {}:", log.module_short)
                        } else {
                            String::new()
                        };

                        visible_lines.push(Line::from(vec![
                            Span::styled(time, Style::default().fg(Color::DarkGray)),
                            Span::raw(" "),
                            Span::styled(
                                format!("[{}]", level_text),
                                Style::default()
                                    .fg(Color::Black)
                                    .bg(level_color)
                                    .add_modifier(Modifier::BOLD)
                            ),
                            Span::styled(module_text, Style::default().fg(Color::Gray)),
                            Span::raw(" "),
                            Span::styled(log.message.clone(), Style::default().fg(level_color)),
                        ]));
                    } else {
                        // Detailed: "YYYY-MM-DD HH:MM:SS [service] [module] [LEVEL] message"
                        let timestamp = if !log.timestamp.is_empty() {
                            log.timestamp.clone()
                        } else {
                            "                           ".to_string()
                        };

                        let level_text = log.level.to_string();
                        let level_color = log.level.color();

                        let mut spans = vec![
                            Span::styled(timestamp, Style::default().fg(Color::DarkGray)),
                            Span::raw(" "),
                        ];

                        if !log.service.is_empty() {
                            spans.push(Span::styled(format!("[{}]", log.service), Style::default().fg(Color::Blue)));
                            spans.push(Span::raw(" "));
                        }

                        if !log.module_path.is_empty() {
                            spans.push(Span::styled(format!("[{}]", log.module_path), Style::default().fg(Color::Cyan)));
                            spans.push(Span::raw(" "));
                        }

                        spans.push(Span::styled(
                            format!("[{}]", level_text),
                            Style::default()
                                .fg(Color::Black)
                                .bg(level_color)
                                .add_modifier(Modifier::BOLD)
                        ));
                        spans.push(Span::raw(" "));
                        spans.push(Span::styled(log.message.clone(), Style::default().fg(level_color)));

                        visible_lines.push(Line::from(spans));
                    }
                }
            }

            let logs_widget = Paragraph::new(visible_lines)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(format!("Logs (showing {}-{} of {}) | 't'=toggle view, 'l'=live, 'g'=group",
                            scroll_offset.min(total_logs),
                            (scroll_offset + max_lines).min(total_logs),
                            total_logs
                        ))
                )
                .wrap(Wrap { trim: false });

            frame.render_widget(logs_widget, chunks[1]);
        }
    }

    fn render_service_detail(&self, frame: &mut Frame, container: &ContainerInfo, logs: &[String], status_message: Option<&str>, reth_metrics: Option<&RethMetrics>) {
        // Determine if we should show metrics section
        let show_metrics = container.name == "execution-layer" && reth_metrics.is_some();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(if show_metrics {
                vec![
                    Constraint::Length(3),   // Title
                    Constraint::Length(6),   // Info section
                    Constraint::Length(12),  // Metrics section (execution-layer only)
                    Constraint::Min(0),      // Logs
                    Constraint::Length(3),   // Footer
                ]
            } else {
                vec![
                    Constraint::Length(3),   // Title
                    Constraint::Length(6),   // Info section
                    Constraint::Min(0),      // Logs
                    Constraint::Length(3),   // Footer
                ]
            })
            .split(frame.size());

        // Title
        let title = Paragraph::new(vec![Line::from(vec![Span::styled(
            format!("Service Details: {}", container.name),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )])])
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));

        frame.render_widget(title, chunks[0]);

        // Info section
        let status_color = if container.status.contains("Up") {
            Color::Green
        } else {
            Color::Red
        };

        let health_color = match container.health.as_deref() {
            Some("healthy") => Color::Green,
            Some("unhealthy") => Color::Red,
            Some("starting") => Color::Yellow,
            _ => Color::Gray,
        };

        let info_text = vec![
            Line::from(vec![
                Span::styled("Status: ", Style::default().fg(Color::White)),
                Span::styled(&container.status, Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
                Span::raw("  |  "),
                Span::styled("Health: ", Style::default().fg(Color::White)),
                Span::styled(
                    container.health.as_deref().unwrap_or("N/A"),
                    Style::default().fg(health_color).add_modifier(Modifier::BOLD)
                ),
            ]),
            Line::from(vec![
                Span::styled("Image: ", Style::default().fg(Color::White)),
                Span::styled(&container.image, Style::default().fg(Color::Cyan)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Actions: ", Style::default().fg(Color::Yellow)),
                Span::raw("[s]tart | [x]top | [R]estart | [r]efresh logs | [Esc/q] back"),
            ]),
        ];

        let info = Paragraph::new(info_text)
            .block(Block::default().borders(Borders::ALL).title("Information"));

        frame.render_widget(info, chunks[1]);

        // Metrics section (execution-layer only)
        let logs_chunk_idx = if show_metrics {
            if let Some(metrics) = reth_metrics {
                let mut metrics_lines = Vec::new();

                // Helper function to format numbers with commas
                let format_number = |n: u64| -> String {
                    n.to_string()
                        .as_bytes()
                        .rchunks(3)
                        .rev()
                        .map(std::str::from_utf8)
                        .collect::<Result<Vec<&str>, _>>()
                        .unwrap()
                        .join(",")
                };

                // Row 1: Blockchain metrics
                metrics_lines.push(Line::from(vec![
                    Span::styled("Block: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        metrics.blocks_processed.map(|v| format!("#{}", format_number(v))).unwrap_or("N/A".to_string()),
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                    ),
                    Span::raw("  "),
                    Span::styled("Canonical: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        metrics.canonical_chain_height.map(|v| format!("#{}", format_number(v))).unwrap_or("N/A".to_string()),
                        Style::default().fg(Color::Cyan)
                    ),
                    Span::raw("  "),
                    Span::styled("Headers: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        metrics.headers_synced.map(|v| format_number(v)).unwrap_or("N/A".to_string()),
                        Style::default().fg(Color::White)
                    ),
                ]));

                // Row 2: Sync status
                metrics_lines.push(Line::from(vec![
                    Span::styled("Sync: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        metrics.sync_stage.as_deref().unwrap_or("N/A"),
                        Style::default().fg(if metrics.sync_stage.as_deref() == Some("Synced") { Color::Green } else { Color::Yellow })
                    ),
                    Span::raw("  "),
                    Span::styled("Checkpoint: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        metrics.sync_checkpoint.map(|v| format_number(v)).unwrap_or("N/A".to_string()),
                        Style::default().fg(Color::White)
                    ),
                ]));

                // Row 3: Network metrics
                metrics_lines.push(Line::from(vec![
                    Span::styled("Peers: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        format!("{} connected / {} tracked",
                            metrics.peers_connected.unwrap_or(0),
                            metrics.peers_tracked.unwrap_or(0)
                        ),
                        Style::default().fg(if metrics.peers_connected.unwrap_or(0) > 0 { Color::Green } else { Color::Yellow })
                    ),
                ]));

                // Row 4: Transaction metrics
                metrics_lines.push(Line::from(vec![
                    Span::styled("Transactions: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        metrics.transactions_total.map(|v| format_number(v)).unwrap_or("N/A".to_string()),
                        Style::default().fg(Color::Cyan)
                    ),
                    Span::raw("  "),
                    Span::styled("Pending: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        metrics.transactions_pending.map(|v| v.to_string()).unwrap_or("N/A".to_string()),
                        Style::default().fg(Color::White)
                    ),
                    Span::raw("  "),
                    Span::styled("Blob: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        metrics.transactions_blob.map(|v| v.to_string()).unwrap_or("N/A".to_string()),
                        Style::default().fg(Color::White)
                    ),
                ]));

                // Row 5: TPS
                metrics_lines.push(Line::from(vec![
                    Span::styled("TPS: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        metrics.tps.map(|v| format!("{:.2} tx/s", v)).unwrap_or("Calculating...".to_string()),
                        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
                    ),
                    Span::raw("  "),
                    Span::styled("Inserted: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        metrics.transactions_inserted.map(|v| format_number(v)).unwrap_or("N/A".to_string()),
                        Style::default().fg(Color::White)
                    ),
                ]));

                // Row 6: Performance metrics
                let memory_mb = metrics.memory_bytes.map(|b| b as f64 / 1024.0 / 1024.0);
                let gas_billions = metrics.gas_processed.map(|g| g as f64 / 1_000_000_000.0);
                metrics_lines.push(Line::from(vec![
                    Span::styled("Memory: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        memory_mb.map(|m| format!("{:.1} MB", m)).unwrap_or("N/A".to_string()),
                        Style::default().fg(Color::White)
                    ),
                    Span::raw("  "),
                    Span::styled("Gas: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        gas_billions.map(|g| format!("{:.2}B", g)).unwrap_or("N/A".to_string()),
                        Style::default().fg(Color::White)
                    ),
                    Span::raw("  "),
                    Span::styled("Payloads: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        metrics.payloads_initiated.map(|v| v.to_string()).unwrap_or("N/A".to_string()),
                        Style::default().fg(Color::White)
                    ),
                ]));

                // Row 7: Blockchain tree metrics
                metrics_lines.push(Line::from(vec![
                    Span::styled("In-mem blocks: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        metrics.in_mem_blocks.map(|v| v.to_string()).unwrap_or("N/A".to_string()),
                        Style::default().fg(Color::White)
                    ),
                    Span::raw("  "),
                    Span::styled("Reorgs: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        metrics.reorgs_total.map(|v| v.to_string()).unwrap_or("N/A".to_string()),
                        Style::default().fg(if metrics.reorgs_total.unwrap_or(0) > 0 { Color::Yellow } else { Color::Green })
                    ),
                    Span::raw("  "),
                    Span::styled("Depth: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        metrics.reorg_depth.map(|v| v.to_string()).unwrap_or("N/A".to_string()),
                        Style::default().fg(Color::White)
                    ),
                ]));

                let metrics_widget = Paragraph::new(metrics_lines)
                    .block(Block::default().borders(Borders::ALL).title("Reth Metrics"));

                frame.render_widget(metrics_widget, chunks[2]);
            }
            3  // Logs are in chunk 3 when metrics are shown
        } else {
            2  // Logs are in chunk 2 when no metrics
        };

        // Logs section - Parse and group logs
        let parsed_logs: Vec<ParsedLogLine> = logs.iter()
            .rev()
            .take(100)
            .rev()
            .map(|log| parse_docker_log_line(log))
            .collect();

        let groups = group_logs_by_level_module(parsed_logs);
        let mut log_lines: Vec<Line> = Vec::new();

        for group in groups {
            // Group header: [LEVEL] module:
            let level_text = group.level.to_string();
            let level_color = group.level.color();
            let module_text = format!(" {}", group.module);

            log_lines.push(Line::from(vec![
                Span::styled(
                    format!("[{}]", level_text),
                    Style::default().fg(Color::Black).bg(level_color).add_modifier(Modifier::BOLD)
                ),
                Span::styled(module_text, Style::default().fg(Color::Gray)),
            ]));

            // Group items with tree characters
            let logs_count = group.logs.len();
            for (idx, log) in group.logs.into_iter().enumerate() {
                let prefix = if idx == logs_count - 1 { "  └─ " } else { "  ├─ " };
                let time = format_timestamp_compact(&log.timestamp);
                let level_color = log.level.color();

                log_lines.push(Line::from(vec![
                    Span::styled(prefix, Style::default().fg(Color::DarkGray)),
                    Span::styled(time, Style::default().fg(Color::DarkGray)),
                    Span::raw(" "),
                    Span::styled(log.message.clone(), Style::default().fg(level_color)),
                ]));
            }
        }

        let logs_widget = Paragraph::new(log_lines)
            .block(Block::default().borders(Borders::ALL).title(format!("Recent Logs (grouped, last {} lines)", logs.len())))
            .wrap(Wrap { trim: false });

        frame.render_widget(logs_widget, chunks[logs_chunk_idx]);

        // Footer
        let footer_chunk_idx = logs_chunk_idx + 1;
        let footer_text = if let Some(status) = status_message {
            status.to_string()
        } else {
            "[s]tart | [x]top | [R]estart | [r]efresh logs | [Esc/q] back to list".to_string()
        };

        let footer = Paragraph::new(footer_text)
            .alignment(Alignment::Center)
            .style(if status_message.is_some() {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            })
            .block(Block::default().borders(Borders::ALL));

        frame.render_widget(footer, chunks[footer_chunk_idx]);
    }

    fn render_help(&self, frame: &mut Frame, current_screen: Screen) {
        use ratatui::layout::Rect;

        // Create centered overlay
        let area = frame.size();
        let popup_width = area.width.min(80);
        let popup_height = area.height.min(30);
        let popup_x = (area.width.saturating_sub(popup_width)) / 2;
        let popup_y = (area.height.saturating_sub(popup_height)) / 2;

        let popup_area = Rect {
            x: popup_x,
            y: popup_y,
            width: popup_width,
            height: popup_height,
        };

        // Build help text based on current screen
        let mut help_text = vec![
            Line::from(Span::styled(
                "IGRA Orchestra - Keyboard Shortcuts",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled("Global Navigation:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
            Line::from("  [1-5]          Jump to screen (1=Services, 2=Wallets, 3=Watch, 4=Logs, 5=Config)"),
            Line::from("  [← →]          Next/Previous screen (1↔2↔3↔4↔5)"),
            Line::from("  [Tab]          Switch sub-views (Services/Config screens only)"),
            Line::from("  [↑ ↓]          Select items / scroll lists"),
            Line::from(""),
            Line::from(Span::styled("Global Commands:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
            Line::from("  [?] / [F1]     Toggle this help screen"),
            Line::from("  [q]            Quit application"),
            Line::from("  [r]            Refresh data"),
            Line::from("  [u]            Upgrade (pull latest Docker images)"),
            Line::from(""),
        ];

        // Screen-specific help
        match current_screen {
            Screen::Services => {
                help_text.push(Line::from(Span::styled("Services Screen:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))));
                help_text.push(Line::from("  [Tab]          Switch between Services and Profiles views"));
                help_text.push(Line::from(""));
                help_text.push(Line::from(Span::styled("Services View:", Style::default().fg(Color::Cyan))));
                help_text.push(Line::from("  [Enter]        View service details and logs"));
                help_text.push(Line::from("  [s]            Start selected service"));
                help_text.push(Line::from("  [x]            Stop selected service"));
                help_text.push(Line::from("  [R]            Restart selected service"));
                help_text.push(Line::from("  [/]            Search/filter services"));
                help_text.push(Line::from(""));
                help_text.push(Line::from(Span::styled("Profiles View:", Style::default().fg(Color::Cyan))));
                help_text.push(Line::from("  [Enter]        Start selected profile (service group)"));
            }
            Screen::Wallets => {
                help_text.push(Line::from(Span::styled("Wallets Screen:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))));
                help_text.push(Line::from("  [Enter]        Show wallet details"));
                help_text.push(Line::from("  [g]            Generate new wallet for selected worker"));
                help_text.push(Line::from("  [t]            Transfer/Send KAS transaction"));
                help_text.push(Line::from("  [/]            Search/filter wallets"));
                help_text.push(Line::from(""));
                help_text.push(Line::from(Span::styled("Wallet Detail View:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))));
                help_text.push(Line::from("  [Enter]        View transaction details (modal)"));
                help_text.push(Line::from("  [/]            Search transactions (by TxID, address, amount)"));
                help_text.push(Line::from("  [↑↓] / [j/k]   Scroll through transactions"));
                help_text.push(Line::from("  [Esc] / [q]    Return to wallet list"));
                help_text.push(Line::from("  [r]            Refresh wallet data"));
            }
            Screen::Watch => {
                help_text.push(Line::from(Span::styled("Watch Screen:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))));
                help_text.push(Line::from("  [↑↓] / [j/k]   Scroll through transactions"));
                help_text.push(Line::from("  [f]            Filter transactions (All/Transfer/Contract/Entry)"));
                help_text.push(Line::from("  [r]            Start/stop recording transactions"));
                help_text.push(Line::from("  [c]            Clear transaction history"));
            }
            Screen::Config => {
                help_text.push(Line::from(Span::styled("Configuration Screen:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))));
                help_text.push(Line::from("  [Tab]          Cycle tabs (Environment ↔ RPC Tokens ↔ SSL Certificates)"));
                help_text.push(Line::from(""));
                help_text.push(Line::from(Span::styled("Environment Tab:", Style::default().fg(Color::Cyan))));
                help_text.push(Line::from("  [e]            Edit selected config value"));
                help_text.push(Line::from("  [Enter]        Save changes (when editing)"));
                help_text.push(Line::from("  [Esc]          Cancel edit (when editing)"));
                help_text.push(Line::from("  [/]            Search/filter config keys"));
                help_text.push(Line::from(""));
                help_text.push(Line::from(Span::styled("RPC Tokens Tab:", Style::default().fg(Color::Cyan))));
                help_text.push(Line::from("  [Enter]        Test RPC endpoint"));
                help_text.push(Line::from("  [g]            Generate all RPC tokens"));
                help_text.push(Line::from(""));
                help_text.push(Line::from(Span::styled("SSL Certificates Tab:", Style::default().fg(Color::Cyan))));
                help_text.push(Line::from("  [c]            Check certificate status"));
                help_text.push(Line::from("  [n]            Force renewal (restart Traefik)"));
            }
            Screen::Logs => {
                help_text.push(Line::from(Span::styled("Logs Screen:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))));
                help_text.push(Line::from("  [Enter]        Select service and view logs"));
                help_text.push(Line::from("  [Esc]          Go back to service list"));
                help_text.push(Line::from("  [↑↓] / [PgUp/PgDn]  Scroll through logs"));
                help_text.push(Line::from("  [e]            Filter ERROR messages"));
                help_text.push(Line::from("  [w]            Filter WARN messages"));
                help_text.push(Line::from("  [i]            Filter INFO messages"));
                help_text.push(Line::from("  [c]            Clear filter"));
                help_text.push(Line::from("  [f]            Toggle follow mode (auto-refresh)"));
                help_text.push(Line::from("  [r]            Refresh logs"));
            }
        }

        help_text.push(Line::from(""));
        help_text.push(Line::from(Span::styled(
            "Press [?] or [Esc] to close this help",
            Style::default().fg(Color::Gray).add_modifier(Modifier::ITALIC),
        )));

        // Clear the popup area with a semi-transparent effect
        let clear_block = Block::default()
            .style(Style::default().bg(Color::Black));
        frame.render_widget(clear_block, popup_area);

        // Render help content
        let help_widget = Paragraph::new(help_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan))
                    .title(Span::styled(" Help ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)))
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(help_widget, popup_area);
    }

    fn render_send_dialog(&self, frame: &mut Frame, amount: &str, address: &str, active_field: usize, use_wallet_selector: bool, selected_wallet_index: usize, source_address: &str, wallets: &[crate::core::wallet::WalletInfo]) {
        use ratatui::layout::Rect;

        // Create centered dialog
        let area = frame.size();
        let dialog_width = area.width.min(80);
        let dialog_height = if use_wallet_selector { 20 } else { 15 };
        let dialog_x = (area.width.saturating_sub(dialog_width)) / 2;
        let dialog_y = (area.height.saturating_sub(dialog_height)) / 2;

        let dialog_area = Rect {
            x: dialog_x,
            y: dialog_y,
            width: dialog_width,
            height: dialog_height,
        };

        // Clear the dialog area
        let clear_block = Block::default()
            .style(Style::default().bg(Color::Black));
        frame.render_widget(clear_block, dialog_area);

        // Create dialog content
        let amount_field_style = if active_field == 0 {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let address_field_style = if active_field == 1 {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let mut dialog_text = vec![
            Line::from(Span::styled(
                "Send KAS Transaction",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("From: ", Style::default().fg(Color::Gray)),
                Span::styled(source_address, Style::default().fg(Color::Green)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Amount (KAS): ", amount_field_style),
                Span::styled(
                    if amount.is_empty() { "_" } else { amount },
                    if active_field == 0 {
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::UNDERLINED)
                    } else {
                        Style::default().fg(Color::Gray)
                    }
                ),
            ]),
            Line::from(""),
        ];

        // Add destination section
        if use_wallet_selector {
            dialog_text.push(Line::from(vec![
                Span::styled("Destination: ", address_field_style),
                Span::styled("[Wallet Selector - Use ↑↓ to select]", Style::default().fg(Color::Cyan).add_modifier(Modifier::ITALIC)),
            ]));
            dialog_text.push(Line::from(""));

            // Show wallet list (limit to 5 visible)
            for (idx, wallet) in wallets.iter().enumerate().take(8) {
                let is_selected = idx == selected_wallet_index && active_field == 1;
                let wallet_text = format!(
                    "  {} Worker {} - {}",
                    if is_selected { "►" } else { " " },
                    wallet.worker_id,
                    wallet.address.as_deref().unwrap_or("(no address)")
                );
                dialog_text.push(Line::from(Span::styled(
                    wallet_text,
                    if is_selected {
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Gray)
                    }
                )));
            }
        } else {
            dialog_text.push(Line::from(vec![
                Span::styled("Destination Address: ", address_field_style),
            ]));
            dialog_text.push(Line::from(vec![
                Span::styled(
                    if address.is_empty() { "_" } else { address },
                    if active_field == 1 {
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::UNDERLINED)
                    } else {
                        Style::default().fg(Color::Gray)
                    }
                ),
            ]));
        }

        dialog_text.push(Line::from(""));
        dialog_text.push(Line::from(""));
        dialog_text.push(Line::from(Span::styled(
            "Tab: Switch | s: Toggle wallet/manual | Enter: Send | Esc: Cancel",
            Style::default().fg(Color::Gray).add_modifier(Modifier::ITALIC),
        )));

        let dialog_widget = Paragraph::new(dialog_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan))
                    .title(Span::styled(" Send Transaction ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)))
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(dialog_widget, dialog_area);
    }

    fn render_wallet_detail(&self, frame: &mut Frame, wallet: &WalletInfo, addresses: &[(String, f64, f64)], utxos: &[crate::core::wallet::UtxoInfo], status_message: Option<&str>, scroll_offset: usize, tx_search_mode: bool, tx_search_buffer: &str, filtered_tx_indices: &[usize], selected_tx_index: Option<usize>) {
        let currency = if self.network == "mainnet" { "KAS" } else { "TKAS" };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),   // Title
                Constraint::Length(8),   // Wallet info section
                Constraint::Length((addresses.len().min(5) + 3) as u16),  // Address balances (limited height)
                Constraint::Min(0),      // Activity (UTXOs)
                Constraint::Length(3),   // Footer
            ])
            .split(frame.size());

        // Title
        let title = Paragraph::new(vec![Line::from(vec![Span::styled(
            format!("Wallet Details: Worker {}", wallet.worker_id),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )])])
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));

        frame.render_widget(title, chunks[0]);

        // Wallet info section
        let status_color = if wallet.container_running {
            Color::Green
        } else {
            Color::Red
        };

        let balance_text = wallet
            .balance
            .map(|b| format!("{:.8} {}", b, currency))
            .unwrap_or_else(|| "N/A".to_string());

        let (fees_text, fees_color) = if let Some(fees) = wallet.fees_spent {
            let color = if fees == 0.0 {
                Color::Green
            } else if fees < 5.0 {
                Color::Yellow
            } else {
                Color::Red
            };
            (format!("{:.8} {}", fees, currency), color)
        } else {
            ("N/A".to_string(), Color::Gray)
        };

        let initial_balance_text = wallet
            .initial_balance
            .map(|b| format!("{:.8} {}", b, currency))
            .unwrap_or_else(|| "N/A".to_string());

        let info_text = vec![
            Line::from(vec![
                Span::styled("Status: ", Style::default().fg(Color::White)),
                Span::styled(
                    if wallet.container_running { "Running" } else { "Stopped" },
                    Style::default().fg(status_color).add_modifier(Modifier::BOLD)
                ),
            ]),
            Line::from(vec![
                Span::styled("Address: ", Style::default().fg(Color::White)),
                Span::styled(
                    wallet.address.as_deref().unwrap_or("Not generated"),
                    Style::default().fg(Color::Cyan)
                ),
            ]),
            Line::from(vec![
                Span::styled("Total Balance: ", Style::default().fg(Color::White)),
                Span::styled(balance_text, Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("Initial Balance: ", Style::default().fg(Color::White)),
                Span::styled(initial_balance_text, Style::default().fg(Color::Gray)),
            ]),
            Line::from(vec![
                Span::styled("Fees Spent: ", Style::default().fg(Color::White)),
                Span::styled(fees_text, Style::default().fg(fees_color).add_modifier(Modifier::BOLD)),
            ]),
        ];

        let info = Paragraph::new(info_text)
            .block(Block::default().borders(Borders::ALL).title("Wallet Information"));

        frame.render_widget(info, chunks[1]);

        // Address balances section
        if addresses.is_empty() {
            let empty_text = vec![
                Line::from(Span::styled(
                    "No address balances available",
                    Style::default().fg(Color::Yellow),
                )),
            ];
            let empty_widget = Paragraph::new(empty_text)
                .block(Block::default().borders(Borders::ALL).title("Address Balances"))
                .alignment(Alignment::Center);
            frame.render_widget(empty_widget, chunks[2]);
        } else {
            let header = Row::new(vec!["Address", "Available", "Pending"])
                .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                .bottom_margin(1);

            // Show all addresses (no limit)
            let rows: Vec<Row> = addresses.iter().map(|(address, available, pending)| {
                Row::new(vec![
                    Cell::from(address.clone()),
                    Cell::from(Span::styled(
                        format!("{:.8} {}", available, currency),
                        Style::default().fg(Color::Green)
                    )),
                    Cell::from(Span::styled(
                        format!("{:.8} {}", pending, currency),
                        if *pending > 0.0 { Style::default().fg(Color::Yellow) } else { Style::default().fg(Color::Gray) }
                    )),
                ])
            }).collect();

            let table = Table::new(
                rows,
                [
                    Constraint::Min(30),
                    Constraint::Length(20),
                    Constraint::Length(20),
                ],
            )
            .header(header)
            .block(Block::default().borders(Borders::ALL).title(format!("Address Balances ({} addresses)", addresses.len())));

            frame.render_widget(table, chunks[2]);
        }

        // Activity section (UTXOs) - Show detailed transaction information
        if utxos.is_empty() {
            let empty_text = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "No transaction history found",
                    Style::default().fg(Color::Gray),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "This wallet has no incoming transactions (UTXOs)",
                    Style::default().fg(Color::Gray),
                )),
            ];
            let empty_widget = Paragraph::new(empty_text)
                .block(Block::default().borders(Borders::ALL).title("Activity / Transaction History"))
                .alignment(Alignment::Center);
            frame.render_widget(empty_widget, chunks[3]);
        } else {
            // Determine which transactions to show
            let (display_utxos, total_count): (Vec<&crate::core::wallet::UtxoInfo>, usize) = if !filtered_tx_indices.is_empty() {
                // Show filtered transactions
                let filtered: Vec<&crate::core::wallet::UtxoInfo> = filtered_tx_indices
                    .iter()
                    .filter_map(|&idx| utxos.get(idx))
                    .collect();
                let count = filtered.len();
                (filtered, count)
            } else {
                // Show all transactions
                (utxos.iter().collect(), utxos.len())
            };

            // Build header with search indicator
            let title = if !filtered_tx_indices.is_empty() {
                format!("Transaction History ({} of {} UTXOs - filtered by '{}')", display_utxos.len(), utxos.len(), tx_search_buffer)
            } else {
                format!("Transaction History ({} UTXOs)", total_count)
            };

            let mut tx_lines = vec![
                Line::from(Span::styled(
                    title,
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                )),
                Line::from(""),
            ];

            for (display_idx, utxo) in display_utxos.iter().enumerate() {
                let is_selected = selected_tx_index == Some(display_idx);
                let utxo_type = if utxo.is_coinbase { "Coinbase" } else { "Transfer" };
                let type_color = if utxo.is_coinbase { Color::Yellow } else { Color::Cyan };

                // Format timestamp
                let timestamp_str = if utxo.timestamp_ms > 0 {
                    let secs = utxo.timestamp_ms / 1000;
                    let dt = chrono::DateTime::from_timestamp(secs as i64, 0);
                    if let Some(datetime) = dt {
                        datetime.format("%Y-%m-%d %H:%M:%S UTC").to_string()
                    } else {
                        "Unknown".to_string()
                    }
                } else {
                    "Unknown".to_string()
                };

                // Selection indicator prefix (arrow only, no background)
                let prefix = if is_selected { "► " } else { "  " };

                // Transaction header
                tx_lines.push(Line::from(vec![
                    Span::styled(format!("{}[{}] ", prefix, display_idx + 1), Style::default().fg(Color::Gray)),
                    Span::styled(utxo_type, Style::default().fg(type_color).add_modifier(Modifier::BOLD)),
                    Span::styled(
                        format!(" - {:.8} {}", utxo.amount_kas, currency),
                        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
                    ),
                ]));

                // Date/Time
                tx_lines.push(Line::from(vec![
                    Span::styled(format!("{}  Date: ", prefix), Style::default().fg(Color::Gray)),
                    Span::styled(timestamp_str, Style::default().fg(Color::Yellow)),
                ]));

                // Transaction ID (full)
                tx_lines.push(Line::from(vec![
                    Span::styled(format!("{}  TxID: ", prefix), Style::default().fg(Color::Gray)),
                    Span::styled(&utxo.tx_id, Style::default().fg(Color::Cyan)),
                ]));

                // Block DAA Score
                tx_lines.push(Line::from(vec![
                    Span::styled(format!("{}  Block DAA: ", prefix), Style::default().fg(Color::Gray)),
                    Span::styled(format!("{}", utxo.block_daa_score), Style::default().fg(Color::White)),
                ]));

                // Address
                tx_lines.push(Line::from(vec![
                    Span::styled(format!("{}  Address: ", prefix), Style::default().fg(Color::Gray)),
                    Span::styled(&utxo.address, Style::default().fg(Color::Magenta)),
                ]));

                tx_lines.push(Line::from("")); // Blank line between transactions
            }

            // Add scroll indicator to title
            let scroll_indicator = if utxos.len() > 1 {
                format!(" (↑/↓ to scroll, showing from line {})", scroll_offset + 1)
            } else {
                String::new()
            };

            let tx_paragraph = Paragraph::new(tx_lines)
                .block(Block::default().borders(Borders::ALL).title(format!("Activity / Transaction History{}", scroll_indicator)))
                .wrap(ratatui::widgets::Wrap { trim: false })
                .scroll((scroll_offset as u16, 0));

            frame.render_widget(tx_paragraph, chunks[3]);
        }

        // Footer
        let footer_text = if let Some(status) = status_message {
            status.to_string()
        } else {
            "[Esc/q] back | [Enter] details | [/] search | [↑/↓] scroll | [r]efresh".to_string()
        };

        let footer = Paragraph::new(footer_text)
            .alignment(Alignment::Center)
            .style(if status_message.is_some() {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            })
            .block(Block::default().borders(Borders::ALL));

        frame.render_widget(footer, chunks[4]);
    }

    fn render_transaction_detail_modal(&self, frame: &mut Frame, utxo: &crate::core::wallet::UtxoInfo) {
        let currency = if self.network == "mainnet" { "KAS" } else { "TKAS" };

        // Create centered modal area (70% width, 80% height)
        let area = frame.size();
        let modal_width = (area.width * 70) / 100;
        let modal_height = (area.height * 80) / 100;
        let modal_x = (area.width - modal_width) / 2;
        let modal_y = (area.height - modal_height) / 2;

        let modal_area = ratatui::layout::Rect {
            x: modal_x,
            y: modal_y,
            width: modal_width,
            height: modal_height,
        };

        // Render opaque dark overlay over entire screen using Clear
        use ratatui::widgets::Clear;
        frame.render_widget(Clear, frame.size());

        // Add dark background
        let overlay = Block::default()
            .style(Style::default().bg(Color::Black));
        frame.render_widget(overlay, frame.size());

        // Format timestamp and relative time
        let (timestamp_str, relative_time) = if utxo.timestamp_ms > 0 {
            let secs = utxo.timestamp_ms / 1000;
            let dt = chrono::DateTime::from_timestamp(secs as i64, 0);
            let timestamp = if let Some(datetime) = dt {
                datetime.format("%Y-%m-%d %H:%M:%S UTC").to_string()
            } else {
                "Unknown".to_string()
            };

            // Calculate relative time
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let age_secs = now.saturating_sub(secs as u64);
            let relative = if age_secs < 60 {
                format!("{} seconds ago", age_secs)
            } else if age_secs < 3600 {
                format!("{} minutes ago", age_secs / 60)
            } else if age_secs < 86400 {
                format!("{} hours ago", age_secs / 3600)
            } else {
                format!("{} days ago", age_secs / 86400)
            };
            (timestamp, relative)
        } else {
            ("Unknown".to_string(), "Unknown".to_string())
        };

        let utxo_type = if utxo.is_coinbase { "Coinbase Reward" } else { "Transfer" };
        let amount_sompi = (utxo.amount_kas * 100_000_000.0) as u64;

        // Build modal content with more details
        let mut lines = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("━━━ TRANSACTION INFORMATION ━━━", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Type: ", Style::default().fg(Color::Gray).add_modifier(Modifier::BOLD)),
                Span::styled(utxo_type, Style::default().fg(if utxo.is_coinbase { Color::Yellow } else { Color::Cyan }).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Amount: ", Style::default().fg(Color::Gray).add_modifier(Modifier::BOLD)),
                Span::styled(
                    format!("{:.8} {}", utxo.amount_kas, currency),
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
                ),
            ]),
            Line::from(vec![
                Span::styled("        ", Style::default()),
                Span::styled(format!("({} sompi)", amount_sompi), Style::default().fg(Color::DarkGray)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("━━━ TIMING ━━━", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Date/Time: ", Style::default().fg(Color::Gray).add_modifier(Modifier::BOLD)),
                Span::styled(&timestamp_str, Style::default().fg(Color::Yellow)),
            ]),
            Line::from(vec![
                Span::styled("Age:       ", Style::default().fg(Color::Gray).add_modifier(Modifier::BOLD)),
                Span::styled(relative_time, Style::default().fg(Color::Yellow)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("━━━ BLOCKCHAIN DATA ━━━", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(Span::styled("Transaction ID:", Style::default().fg(Color::Gray).add_modifier(Modifier::BOLD))),
            Line::from(Span::styled(&utxo.tx_id, Style::default().fg(Color::Cyan))),
            Line::from(""),
            Line::from(vec![
                Span::styled("Block DAA Score: ", Style::default().fg(Color::Gray).add_modifier(Modifier::BOLD)),
                Span::styled(format!("{}", utxo.block_daa_score), Style::default().fg(Color::White)),
            ]),
            Line::from(""),
            Line::from(Span::styled("Destination Address (Your Wallet):", Style::default().fg(Color::Gray).add_modifier(Modifier::BOLD))),
            Line::from(Span::styled(&utxo.address, Style::default().fg(Color::Magenta))),
            Line::from(""),
        ];

        // Add source addresses for non-coinbase transactions
        if !utxo.is_coinbase {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("━━━ TRANSACTION SOURCE ━━━", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            ]));
            lines.push(Line::from(""));

            if !utxo.source_addresses.is_empty() {
                lines.push(Line::from(vec![
                    Span::styled("Source Address(es):", Style::default().fg(Color::Gray).add_modifier(Modifier::BOLD)),
                ]));
                for (idx, source_addr) in utxo.source_addresses.iter().enumerate() {
                    let prefix = if utxo.source_addresses.len() > 1 {
                        format!("  [{}] ", idx + 1)
                    } else {
                        "  ".to_string()
                    };
                    lines.push(Line::from(vec![
                        Span::styled(prefix, Style::default().fg(Color::DarkGray)),
                        Span::styled(source_addr.clone(), Style::default().fg(Color::Magenta)),
                    ]));
                }
            } else {
                lines.push(Line::from(vec![
                    Span::styled("Source Address: ", Style::default().fg(Color::Gray).add_modifier(Modifier::BOLD)),
                    Span::styled("Not available", Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("  (Transaction confirmed in block, source data archived)", Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)),
                ]));
            }
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
                "Press [Enter] or [Esc] to close",
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)
            )));

        let modal_widget = Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                    .border_type(ratatui::widgets::BorderType::Double)
                    .title(Span::styled(
                        " 📋 Transaction Details ",
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                    ))
                    .style(Style::default().bg(Color::Black))
            )
            .alignment(Alignment::Left)
            .wrap(ratatui::widgets::Wrap { trim: false });

        // Render modal
        frame.render_widget(modal_widget, modal_area);
    }
}
