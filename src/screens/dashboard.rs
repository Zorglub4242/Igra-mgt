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

    pub fn render(&self, frame: &mut Frame, current_screen: Screen, selected_index: usize, status_message: Option<&str>, edit_mode: bool, edit_buffer: &str, detail_container: Option<&ContainerInfo>, detail_logs: &[String], system_resources: &SystemResources, show_help: bool, logs_selected_service: Option<&str>, logs_data: &[String], logs_follow_mode: bool, logs_filter: Option<&str>, logs_scroll_offset: usize, containers: &[ContainerInfo], search_mode: bool, search_buffer: &str, filtered_indices: &[usize], show_send_dialog: bool, send_amount: &str, send_address: &str, send_input_field: usize) {
        // If showing detail view, render that instead
        if let Some(container) = detail_container {
            self.render_service_detail(frame, container, detail_logs, status_message);
            // Still show help overlay if requested
            if show_help {
                self.render_help(frame, current_screen);
            }
            return;
        }
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Title
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

        let title = Paragraph::new(vec![title_line])
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
        let header = Row::new(vec!["Worker", "Status", "Address", "Balance"])
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
                .map(|b| format!("{:.8} KAS", b))
                .unwrap_or_else(|| "N/A".to_string());

            let row = Row::new(vec![
                Cell::from(format!("Worker {}", wallet.worker_id)),
                Cell::from(Span::styled(status.0, Style::default().fg(status.1))),
                Cell::from(address),
                Cell::from(balance),
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
                Constraint::Min(45),
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

    fn render_service_detail(&self, frame: &mut Frame, container: &ContainerInfo, logs: &[String], status_message: Option<&str>) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Title
                Constraint::Length(6),  // Info section
                Constraint::Min(0),     // Logs
                Constraint::Length(3),  // Footer
            ])
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

        frame.render_widget(logs_widget, chunks[2]);

        // Footer
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

        frame.render_widget(footer, chunks[3]);
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
}
