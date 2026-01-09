/// Service Details TUI Screen
///
/// Displays comprehensive service information in a tabbed interface
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs, Wrap},
    Frame,
};

use crate::core::docker::ServiceDetails;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceDetailsTab {
    Overview,
    Metrics,
    Configuration,
    Storage,
    Network,
    Logs,
}

impl ServiceDetailsTab {
    pub fn all() -> &'static [ServiceDetailsTab] {
        &[
            ServiceDetailsTab::Overview,
            ServiceDetailsTab::Metrics,
            ServiceDetailsTab::Configuration,
            ServiceDetailsTab::Storage,
            ServiceDetailsTab::Network,
            ServiceDetailsTab::Logs,
        ]
    }

    pub fn title(&self) -> &'static str {
        match self {
            ServiceDetailsTab::Overview => "Overview",
            ServiceDetailsTab::Metrics => "Metrics",
            ServiceDetailsTab::Configuration => "Configuration",
            ServiceDetailsTab::Storage => "Storage",
            ServiceDetailsTab::Network => "Network",
            ServiceDetailsTab::Logs => "Logs",
        }
    }
}

pub struct ServiceDetailsScreen {
    pub selected_tab: usize,
    pub scroll_offset: usize,
}

impl ServiceDetailsScreen {
    pub fn new() -> Self {
        Self {
            selected_tab: 0,
            scroll_offset: 0,
        }
    }

    pub fn next_tab(&mut self) {
        let tabs = ServiceDetailsTab::all();
        self.selected_tab = (self.selected_tab + 1) % tabs.len();
        self.scroll_offset = 0;
    }

    pub fn prev_tab(&mut self) {
        let tabs = ServiceDetailsTab::all();
        self.selected_tab = if self.selected_tab == 0 {
            tabs.len() - 1
        } else {
            self.selected_tab - 1
        };
        self.scroll_offset = 0;
    }

    pub fn scroll_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(1);
    }

    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    pub fn render(&self, f: &mut Frame, area: Rect, details: &ServiceDetails) {
        // Main layout: title + tabs + content
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Length(3), // Tabs
                Constraint::Min(0),    // Content
                Constraint::Length(3), // Help
            ])
            .split(area);

        // Title
        let title = Paragraph::new(format!("Service Details: {}", details.name))
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        // Tabs
        let tabs = ServiceDetailsTab::all();
        let tab_titles: Vec<String> = tabs.iter().map(|t| t.title().to_string()).collect();
        let tabs_widget = Tabs::new(tab_titles)
            .block(Block::default().borders(Borders::ALL).title("Tabs"))
            .select(self.selected_tab)
            .style(Style::default().fg(Color::White))
            .highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            );
        f.render_widget(tabs_widget, chunks[1]);

        // Content based on selected tab
        match tabs[self.selected_tab] {
            ServiceDetailsTab::Overview => self.render_overview(f, chunks[2], details),
            ServiceDetailsTab::Metrics => self.render_metrics(f, chunks[2], details),
            ServiceDetailsTab::Configuration => self.render_configuration(f, chunks[2], details),
            ServiceDetailsTab::Storage => self.render_storage(f, chunks[2], details),
            ServiceDetailsTab::Network => self.render_network(f, chunks[2], details),
            ServiceDetailsTab::Logs => self.render_logs(f, chunks[2], details),
        }

        // Help text
        let help_text = "Tab/Shift+Tab: Switch tabs | ↑/↓: Scroll | Esc/q: Back | r: Refresh";
        let help = Paragraph::new(help_text)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(help, chunks[3]);
    }

    fn render_overview(&self, f: &mut Frame, area: Rect, details: &ServiceDetails) {
        let mut lines = vec![];

        // Basic info
        lines.push(Line::from(vec![
            Span::styled("Status: ", Style::default().fg(Color::Gray)),
            Span::styled(&details.status, Style::default().fg(Color::Green)),
        ]));

        lines.push(Line::from(vec![
            Span::styled("State: ", Style::default().fg(Color::Gray)),
            Span::styled(&details.state, Style::default().fg(Color::Cyan)),
        ]));

        lines.push(Line::from(vec![
            Span::styled("Image: ", Style::default().fg(Color::Gray)),
            Span::styled(&details.image, Style::default().fg(Color::White)),
        ]));

        lines.push(Line::from(vec![
            Span::styled("Created: ", Style::default().fg(Color::Gray)),
            Span::styled(&details.created, Style::default().fg(Color::White)),
        ]));

        lines.push(Line::from(vec![
            Span::styled("Started: ", Style::default().fg(Color::Gray)),
            Span::styled(&details.started, Style::default().fg(Color::White)),
        ]));

        lines.push(Line::from(""));

        // Note
        lines.push(Line::from(Span::styled(
            "Description:",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(details.note.as_str()));

        lines.push(Line::from(""));

        // Resource usage
        lines.push(Line::from(Span::styled(
            "Resource Usage:",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )));

        lines.push(Line::from(vec![
            Span::styled("CPU: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{:.2}%", details.cpu_stats.cpu_percent),
                Style::default().fg(Color::Cyan),
            ),
        ]));

        lines.push(Line::from(vec![
            Span::styled("Memory: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{:.2}%", details.memory_stats.percent),
                Style::default().fg(Color::Cyan),
            ),
            Span::raw(" ("),
            Span::raw(format!(
                "{} / {}",
                format_bytes(details.memory_stats.usage),
                format_bytes(details.memory_stats.limit)
            )),
            Span::raw(")"),
        ]));

        let paragraph = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title("Overview"))
            .wrap(Wrap { trim: false })
            .scroll((self.scroll_offset as u16, 0));

        f.render_widget(paragraph, area);
    }

    fn render_metrics(&self, f: &mut Frame, area: Rect, details: &ServiceDetails) {
        let items: Vec<ListItem> = details
            .metrics
            .iter()
            .map(|metric| {
                let category = metric.category.as_deref().unwrap_or("general");
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("[{}] ", category),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled(&metric.name, Style::default().fg(Color::Yellow)),
                    Span::raw(": "),
                    Span::styled(&metric.formatted, Style::default().fg(Color::Cyan)),
                ]))
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Metrics"))
            .style(Style::default().fg(Color::White));

        f.render_widget(list, area);
    }

    fn render_configuration(&self, f: &mut Frame, area: Rect, details: &ServiceDetails) {
        let mut lines = vec![];

        // Environment variables
        lines.push(Line::from(Span::styled(
            "Environment Variables:",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )));

        for (key, value) in &details.env_vars {
            lines.push(Line::from(vec![
                Span::styled(key, Style::default().fg(Color::Cyan)),
                Span::raw(" = "),
                Span::styled(value, Style::default().fg(Color::White)),
            ]));
        }

        lines.push(Line::from(""));

        // Command and entrypoint
        if let Some(cmd) = &details.command {
            lines.push(Line::from(Span::styled(
                "Command:",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(cmd.as_str()));
            lines.push(Line::from(""));
        }

        if let Some(entrypoint) = &details.entrypoint {
            lines.push(Line::from(Span::styled(
                "Entrypoint:",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(entrypoint.as_str()));
            lines.push(Line::from(""));
        }

        let paragraph = Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Configuration"),
            )
            .wrap(Wrap { trim: false })
            .scroll((self.scroll_offset as u16, 0));

        f.render_widget(paragraph, area);
    }

    fn render_storage(&self, f: &mut Frame, area: Rect, details: &ServiceDetails) {
        let items: Vec<ListItem> = details
            .volumes
            .iter()
            .map(|vol| {
                ListItem::new(Line::from(vec![
                    Span::styled(&vol.source, Style::default().fg(Color::Cyan)),
                    Span::raw(" → "),
                    Span::styled(&vol.destination, Style::default().fg(Color::Yellow)),
                    Span::raw(" ("),
                    Span::styled(&vol.mode, Style::default().fg(Color::Gray)),
                    Span::raw(")"),
                ]))
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Volumes"))
            .style(Style::default().fg(Color::White));

        f.render_widget(list, area);
    }

    fn render_network(&self, f: &mut Frame, area: Rect, details: &ServiceDetails) {
        let mut lines = vec![];

        // Networks
        lines.push(Line::from(Span::styled(
            "Networks:",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )));

        for net in &details.networks {
            lines.push(Line::from(vec![
                Span::styled(&net.name, Style::default().fg(Color::Cyan)),
                Span::raw(" - IP: "),
                Span::styled(&net.ip_address, Style::default().fg(Color::Green)),
                Span::raw(" - Gateway: "),
                Span::styled(&net.gateway, Style::default().fg(Color::White)),
            ]));
        }

        lines.push(Line::from(""));

        // Ports
        lines.push(Line::from(Span::styled(
            "Port Mappings:",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )));

        for port in &details.ports {
            let host_port_str = port
                .host_port
                .map(|p| p.to_string())
                .unwrap_or_else(|| "N/A".to_string());
            let container_port_str = port.container_port.to_string();
            lines.push(Line::from(format!(
                "{} → {}/{}",
                host_port_str, container_port_str, port.protocol
            )));
        }

        let paragraph = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title("Network"))
            .wrap(Wrap { trim: false })
            .scroll((self.scroll_offset as u16, 0));

        f.render_widget(paragraph, area);
    }

    fn render_logs(&self, f: &mut Frame, area: Rect, _details: &ServiceDetails) {
        let placeholder = Paragraph::new(
            "Log viewing not implemented yet.\nUse 'docker logs' command or the Logs screen.",
        )
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("Logs"));

        f.render_widget(placeholder, area);
    }
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
