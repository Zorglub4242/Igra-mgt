/// Main dashboard screen

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Wrap},
    Frame,
};

use crate::app::{Screen, SystemResources};
use crate::core::docker::{ContainerInfo, ContainerStats};
use crate::core::wallet::WalletInfo;
use crate::core::ssl::CertificateInfo;
use crate::core::reth_metrics::RethMetrics;
use std::collections::HashMap;

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

    pub fn render(&self, frame: &mut Frame, current_screen: Screen, selected_index: usize, status_message: Option<&str>, edit_mode: bool, edit_buffer: &str, detail_container: Option<&ContainerInfo>, detail_logs: &[String], system_resources: &SystemResources, show_help: bool, logs_selected_service: Option<&str>, logs_data: &[String], logs_follow_mode: bool, logs_filter: Option<&str>, logs_scroll_offset: usize, containers: &[ContainerInfo], search_mode: bool, search_buffer: &str, filtered_indices: &[usize], show_send_dialog: bool, send_amount: &str, send_address: &str, send_input_field: usize, reth_metrics: Option<&RethMetrics>, detail_wallet: Option<&WalletInfo>, detail_wallet_addresses: &[(String, f64, f64)], detail_wallet_utxos: &[crate::core::wallet::UtxoInfo], detail_wallet_scroll: usize, show_tx_detail: bool, selected_tx_index: Option<usize>, tx_search_mode: bool, tx_search_buffer: &str, filtered_tx_indices: &[usize]) {
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
            Screen::Services => self.render_services(frame, chunks[2], selected_index, filtered_indices),
            Screen::Profiles => self.render_profiles(frame, chunks[2], selected_index),
            Screen::Wallets => self.render_wallets(frame, chunks[2], selected_index, filtered_indices),
            Screen::RpcTokens => self.render_rpc_tokens(frame, chunks[2], selected_index),
            Screen::Config => self.render_config(frame, chunks[2], selected_index, edit_mode, edit_buffer, filtered_indices),
            Screen::Ssl => self.render_ssl(frame, chunks[2]),
            Screen::Logs => self.render_logs(frame, chunks[2], logs_selected_service, logs_data, logs_follow_mode, logs_filter, logs_scroll_offset, selected_index, containers),
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
                Screen::Services => "[↑↓] Select | [Enter] Details | [s]tart | [x]top | [R]estart | [/] Search | [r]efresh | [?] Help | [q]uit".to_string(),
                Screen::Profiles => "[↑↓] Select | [Enter/Space] Toggle | [s]tart | [x]top | [u]pgrade | [r]efresh | [?] Help | [q]uit".to_string(),
                Screen::Wallets => "[↑↓] Select | [Enter] Info | [g]enerate | [t]ransfer | [/] Search | [r]efresh | [?] Help | [q]uit".to_string(),
                Screen::RpcTokens => "[↑↓] Select | [Enter] Test | [g]enerate All | [u]pgrade | [r]efresh | [?] Help | [q]uit".to_string(),
                Screen::Config => "[↑↓] Select | [e]dit Value | [/] Search | [r]efresh | [?] Help | [q]uit".to_string(),
                Screen::Ssl => "[c] Check | [n] Force Renewal | [u]pgrade | [r]efresh | [?] Help | [q]uit".to_string(),
                Screen::Logs => {
                    if logs_selected_service.is_some() {
                        "[↑↓/PgUp/PgDn] Scroll | [e]rror [w]arn [i]nfo [c]lear filter | [f]ollow | [r]efresh | [Esc] back | [?] Help".to_string()
                    } else {
                        "[↑↓] Select | [Enter] View logs | [?] Help | [q]uit".to_string()
                    }
                }
                _ => "[1-7] Screen | [Tab]/[←→] Navigate | [u]pgrade | [r]efresh | [?] Help | [q]uit".to_string(),
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
            self.render_send_dialog(frame, send_amount, send_address, send_input_field);
        }
    }

    fn render_services(&self, frame: &mut Frame, area: ratatui::layout::Rect, selected_index: usize, filtered_indices: &[usize]) {
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
        let header = Row::new(vec!["Service", "Status", "Metrics", "Ports", "CPU", "Memory", "Image:Tag"])
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

            let health_color = match container.health.as_deref() {
                Some("healthy") => Color::Green,
                Some("unhealthy") => Color::Red,
                Some("starting") => Color::Yellow,
                _ => Color::Gray,
            };

            let name = container.name.clone();
            let status = container.status.clone();
            let health = container.health.as_deref().unwrap_or("N/A").to_string();

            // Get stats if available with color-coding
            let (cpu_cell, mem_cell, net_rx_text, net_tx_text) = if let Some(stats) = self.container_stats.get(&container.name) {
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

                // Format network I/O
                let rx = Self::format_bytes(stats.network_rx);
                let tx = Self::format_bytes(stats.network_tx);

                (cpu_cell, mem_cell, rx, tx)
            } else {
                (
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

    fn render_config(&self, frame: &mut Frame, area: ratatui::layout::Rect, selected_index: usize, edit_mode: bool, edit_buffer: &str, filtered_indices: &[usize]) {
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

    fn render_logs(&self, frame: &mut Frame, area: ratatui::layout::Rect, selected_service: Option<&str>, logs_data: &[String], follow_mode: bool, filter: Option<&str>, scroll_offset: usize, selected_index: usize, containers: &[ContainerInfo]) {
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
            let follow_text = if follow_mode { " | FOLLOW MODE" } else { "" };

            let header_text = format!(
                "Service: {} | Lines: {}{}{}",
                selected_service.unwrap_or("N/A"),
                logs_data.len(),
                filter_text,
                follow_text
            );

            let header = Paragraph::new(header_text)
                .style(Style::default().fg(Color::Cyan))
                .block(Block::default().borders(Borders::ALL).title("Log Viewer"));
            frame.render_widget(header, chunks[0]);

            // Filter and render logs
            let filtered_logs: Vec<&String> = if let Some(filter_level) = filter {
                logs_data.iter().filter(|line| {
                    line.to_uppercase().contains(filter_level)
                }).collect()
            } else {
                logs_data.iter().collect()
            };

            // Apply scroll offset and get visible logs
            let visible_logs: Vec<Line> = filtered_logs
                .iter()
                .skip(scroll_offset)
                .take(chunks[1].height as usize - 2) // Account for borders
                .map(|log| {
                    let style = if log.contains("ERROR") || log.contains("error") {
                        Style::default().fg(Color::Red)
                    } else if log.contains("WARN") || log.contains("warn") {
                        Style::default().fg(Color::Yellow)
                    } else if log.contains("INFO") || log.contains("info") {
                        Style::default().fg(Color::Cyan)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    Line::from(Span::styled(log.as_str(), style))
                })
                .collect();

            let logs_widget = Paragraph::new(visible_logs)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(format!("Logs (showing {}-{} of {})",
                            scroll_offset.min(filtered_logs.len()),
                            (scroll_offset + chunks[1].height as usize).min(filtered_logs.len()),
                            filtered_logs.len()
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

        // Logs section
        let log_lines: Vec<Line> = logs.iter().rev().take(100).rev().map(|log| {
            let style = if log.contains("ERROR") || log.contains("error") {
                Style::default().fg(Color::Red)
            } else if log.contains("WARN") || log.contains("warn") {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::White)
            };
            Line::from(Span::styled(log.as_str(), style))
        }).collect();

        let logs_widget = Paragraph::new(log_lines)
            .block(Block::default().borders(Borders::ALL).title(format!("Recent Logs (last {} lines)", logs.len())))
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
            Line::from(Span::styled("Global Commands:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
            Line::from("  [?] / [F1]     Toggle this help screen"),
            Line::from("  [q] / [Esc]    Quit application"),
            Line::from("  [r]            Refresh data"),
            Line::from("  [u]            Upgrade (pull latest Docker images)"),
            Line::from("  [1-7]          Jump to screen (1=Services, 2=Profiles, 3=Wallets, etc.)"),
            Line::from("  [Tab] / [→]    Next screen"),
            Line::from("  [Shift+Tab] / [←]  Previous screen"),
            Line::from("  [↑] / [k]      Move selection up"),
            Line::from("  [↓] / [j]      Move selection down"),
            Line::from(""),
        ];

        // Screen-specific help
        match current_screen {
            Screen::Services => {
                help_text.push(Line::from(Span::styled("Services Screen:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))));
                help_text.push(Line::from("  [Enter]        View service details and logs"));
                help_text.push(Line::from("  [s]            Start selected service"));
                help_text.push(Line::from("  [x]            Stop selected service"));
                help_text.push(Line::from("  [R]            Restart selected service"));
                help_text.push(Line::from("  [/]            Search/filter services"));
            }
            Screen::Profiles => {
                help_text.push(Line::from(Span::styled("Profiles Screen:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))));
                help_text.push(Line::from("  [Enter]/[Space] Toggle profile on/off"));
                help_text.push(Line::from("  [s]            Start selected profile"));
                help_text.push(Line::from("  [x]            Stop selected profile"));
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
            Screen::RpcTokens => {
                help_text.push(Line::from(Span::styled("RPC Tokens Screen:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))));
                help_text.push(Line::from("  [Enter]        Test RPC endpoint"));
                help_text.push(Line::from("  [g]            Generate all RPC tokens"));
            }
            Screen::Config => {
                help_text.push(Line::from(Span::styled("Configuration Screen:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))));
                help_text.push(Line::from("  [e]            Edit selected config value"));
                help_text.push(Line::from("  [Enter]        Save changes (when editing)"));
                help_text.push(Line::from("  [Esc]          Cancel edit (when editing)"));
                help_text.push(Line::from("  [/]            Search/filter config keys"));
            }
            Screen::Ssl => {
                help_text.push(Line::from(Span::styled("SSL Certificates Screen:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))));
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

    fn render_send_dialog(&self, frame: &mut Frame, amount: &str, address: &str, active_field: usize) {
        use ratatui::layout::Rect;

        // Create centered dialog
        let area = frame.size();
        let dialog_width = area.width.min(70);
        let dialog_height = 12;
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

        let dialog_text = vec![
            Line::from(Span::styled(
                "Send KAS Transaction",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )),
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
            Line::from(vec![
                Span::styled("Destination Address: ", address_field_style),
            ]),
            Line::from(vec![
                Span::styled(
                    if address.is_empty() { "_" } else { address },
                    if active_field == 1 {
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::UNDERLINED)
                    } else {
                        Style::default().fg(Color::Gray)
                    }
                ),
            ]),
            Line::from(""),
            Line::from(""),
            Line::from(Span::styled(
                "Tab: Switch field | Enter: Send | Esc: Cancel",
                Style::default().fg(Color::Gray).add_modifier(Modifier::ITALIC),
            )),
        ];

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
