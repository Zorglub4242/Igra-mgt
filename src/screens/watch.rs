/// Watch Screen - Real-time L2 transaction monitoring TUI

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::interval;

use crate::core::l2_monitor::{TransactionInfo, TransactionMonitor, TransactionType};

/// Transaction filter
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionFilter {
    All,
    Transfer,
    Contract,
    Entry,
}

impl TransactionFilter {
    pub fn matches(&self, tx_type: &TransactionType) -> bool {
        match self {
            TransactionFilter::All => true,
            TransactionFilter::Transfer => tx_type == &TransactionType::Transfer,
            TransactionFilter::Contract => tx_type == &TransactionType::Contract,
            TransactionFilter::Entry => tx_type == &TransactionType::Entry,
        }
    }
}

/// Watch screen state
struct WatchState {
    transactions: Vec<TransactionInfo>,
    list_state: ListState,
    filter: TransactionFilter,
    file_recorder: Option<std::fs::File>,
    format: String,
}

impl WatchState {
    fn new(record_path: Option<String>, format: String) -> Result<Self> {
        let file_recorder = if let Some(path) = record_path {
            Some(std::fs::File::create(path)?)
        } else {
            None
        };

        Ok(Self {
            transactions: Vec::new(),
            list_state: ListState::default(),
            filter: TransactionFilter::All,
            file_recorder,
            format,
        })
    }

    fn add_transactions(&mut self, new_txs: Vec<TransactionInfo>) {
        for tx in new_txs {
            // Record to file if enabled
            if let Some(ref mut file) = self.file_recorder {
                let _ = Self::write_transaction_to_file_static(file, &tx, &self.format);
            }

            // Add to display list (keep last 100 transactions)
            self.transactions.insert(0, tx);
            if self.transactions.len() > 100 {
                self.transactions.truncate(100);
            }
        }
    }

    fn write_transaction_to_file_static(file: &mut std::fs::File, tx: &TransactionInfo, format: &str) -> Result<()> {
        use std::io::Write;

        match format {
            "json" => {
                let json = serde_json::to_string(tx)?;
                writeln!(file, "{}", json)?;
            }
            "csv" => {
                // CSV header is written once at file creation
                writeln!(
                    file,
                    "{},{},{},{},{},{},{},{},{},{}",
                    tx.timestamp.format("%Y-%m-%d %H:%M:%S"),
                    tx.tx_type,
                    tx.hash,
                    tx.from,
                    tx.to.as_deref().unwrap_or(""),
                    tx.value_ikas(),
                    tx.gas_fee_ikas(),
                    tx.l1_fee.unwrap_or(0.0),
                    tx.status,
                    tx.block_number
                )?;
            }
            _ => {
                // Text format
                writeln!(file, "[{}] {}", tx.timestamp.format("%H:%M:%S"), tx.tx_type)?;
                writeln!(file, "  Hash: {}", tx.hash)?;
                writeln!(file, "  From: {}", tx.from)?;
                if let Some(ref to) = tx.to {
                    writeln!(file, "  To:   {}", to)?;
                }
                writeln!(file, "  Value: {} iKAS", tx.value_ikas())?;
                writeln!(file, "  Gas: {} iKAS", tx.gas_fee_ikas())?;
                if let Some(l1_fee) = tx.l1_fee {
                    writeln!(file, "  L1 Fee: {} KAS", l1_fee)?;
                }
                writeln!(file, "  Status: {}", if tx.status { "Success" } else { "Failed" })?;
                writeln!(file)?;
            }
        }

        Ok(())
    }

    fn write_transaction_to_file(&self, file: &mut std::fs::File, tx: &TransactionInfo) -> Result<()> {
        use std::io::Write;

        match self.format.as_str() {
            "json" => {
                let json = serde_json::to_string(tx)?;
                writeln!(file, "{}", json)?;
            }
            "csv" => {
                // CSV header is written once at file creation
                writeln!(
                    file,
                    "{},{},{},{},{},{},{},{},{},{}",
                    tx.timestamp.format("%Y-%m-%d %H:%M:%S"),
                    tx.tx_type,
                    tx.hash,
                    tx.from,
                    tx.to.as_deref().unwrap_or(""),
                    tx.value_ikas(),
                    tx.gas_fee_ikas(),
                    tx.l1_fee.unwrap_or(0.0),
                    tx.status,
                    tx.block_number
                )?;
            }
            _ => {
                // Text format
                writeln!(file, "[{}] {}", tx.timestamp.format("%H:%M:%S"), tx.tx_type)?;
                writeln!(file, "  Hash: {}", tx.hash)?;
                writeln!(file, "  From: {}", tx.from)?;
                if let Some(ref to) = tx.to {
                    writeln!(file, "  To:   {}", to)?;
                }
                writeln!(file, "  Value: {} iKAS", tx.value_ikas())?;
                writeln!(file, "  Gas: {} iKAS", tx.gas_fee_ikas())?;
                if let Some(l1_fee) = tx.l1_fee {
                    writeln!(file, "  L1 Fee: {} KAS", l1_fee)?;
                }
                writeln!(file, "  Status: {}", if tx.status { "Success" } else { "Failed" })?;
                writeln!(file)?;
            }
        }

        Ok(())
    }

    fn scroll_up(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i > 0 {
                    i - 1
                } else {
                    0
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn scroll_down(&mut self, max: usize) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i < max.saturating_sub(1) {
                    i + 1
                } else {
                    i
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn toggle_filter(&mut self) {
        self.filter = match self.filter {
            TransactionFilter::All => TransactionFilter::Transfer,
            TransactionFilter::Transfer => TransactionFilter::Contract,
            TransactionFilter::Contract => TransactionFilter::Entry,
            TransactionFilter::Entry => TransactionFilter::All,
        };
    }

    fn filtered_transactions(&self) -> Vec<&TransactionInfo> {
        self.transactions
            .iter()
            .filter(|tx| self.filter.matches(&tx.tx_type))
            .collect()
    }
}

/// Run the watch TUI
pub async fn run_watch_tui(
    filter: String,
    record: Option<String>,
    format: String,
) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Initialize monitor and state
    let monitor = Arc::new(TransactionMonitor::new().await?);
    let state = Arc::new(RwLock::new(WatchState::new(record, format)?));

    // Set initial filter
    {
        let mut s = state.write().await;
        s.filter = match filter.as_str() {
            "transfer" => TransactionFilter::Transfer,
            "contract" => TransactionFilter::Contract,
            "entry" => TransactionFilter::Entry,
            _ => TransactionFilter::All,
        };
    }

    // Spawn background tasks
    let monitor_clone = Arc::clone(&monitor);
    let state_clone = Arc::clone(&state);
    tokio::spawn(async move {
        let mut poll_interval = interval(Duration::from_secs(1));
        let mut l1_interval = interval(Duration::from_secs(10));

        loop {
            tokio::select! {
                _ = poll_interval.tick() => {
                    if let Ok(new_txs) = monitor_clone.poll_new_transactions().await {
                        if !new_txs.is_empty() {
                            let mut s = state_clone.write().await;
                            s.add_transactions(new_txs);
                        }
                    }
                }
                _ = l1_interval.tick() => {
                    let _ = monitor_clone.update_l1_data().await;
                }
            }
        }
    });

    // Run UI loop
    let res = run_ui_loop(&mut terminal, &monitor, &state).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    res
}

async fn run_ui_loop<B: Backend>(
    terminal: &mut Terminal<B>,
    monitor: &Arc<TransactionMonitor>,
    state: &Arc<RwLock<WatchState>>,
) -> Result<()> {
    loop {
        // Draw UI
        let stats = monitor.get_statistics().await;
        let state_guard = state.read().await;

        terminal.draw(|f| {
            ui(f, &stats, &*state_guard);
        })?;
        drop(state_guard);

        // Handle input (with timeout)
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        return Ok(());
                    }
                    KeyCode::Up => {
                        let mut s = state.write().await;
                        s.scroll_up();
                    }
                    KeyCode::Down => {
                        let mut s = state.write().await;
                        let max = s.filtered_transactions().len();
                        s.scroll_down(max);
                    }
                    KeyCode::Char('f') => {
                        let mut s = state.write().await;
                        s.toggle_filter();
                    }
                    _ => {}
                }
            }
        }
    }
}

fn ui(
    f: &mut Frame,
    stats: &crate::core::l2_monitor::Statistics,
    state: &WatchState,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // Title
            Constraint::Length(4),  // Stats header
            Constraint::Min(0),     // Transaction list
            Constraint::Length(1),  // Footer
        ])
        .split(f.size());

    // Title
    let title = Paragraph::new("L2 Transaction Monitor - IGRA Testnet")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center);
    f.render_widget(title, chunks[0]);

    // Statistics header
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
    f.render_widget(stats_block, chunks[1]);

    // Transaction list
    let filtered_txs = state.filtered_transactions();

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
                    Span::raw("  From: "),
                    Span::styled(tx.from.clone(), Style::default().fg(Color::Yellow)),
                ]),
            ];

            if let Some(ref to) = tx.to {
                lines.push(Line::from(vec![
                    Span::raw("  To:   "),
                    Span::styled(to.clone(), Style::default().fg(Color::Yellow)),
                ]));
            }

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
                        Style::default().fg(Color::Red),
                    ),
                    Span::styled(" (node wallet)", Style::default().fg(Color::Gray)),
                ]));
            }

            ListItem::new(Text::from(lines)).style(Style::default())
        })
        .collect();

    let filter_str = match state.filter {
        TransactionFilter::All => "All",
        TransactionFilter::Transfer => "Transfers",
        TransactionFilter::Contract => "Contracts",
        TransactionFilter::Entry => "Entry TXs",
    };

    let mut list_state = state.list_state.clone();
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Transactions [Filter: {}]", filter_str)),
        )
        .highlight_style(Style::default().bg(Color::DarkGray));

    f.render_stateful_widget(list, chunks[2], &mut list_state);

    // Footer
    let footer = Paragraph::new("[q] Quit  [↑↓] Scroll  [f] Toggle Filter")
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center);
    f.render_widget(footer, chunks[3]);
}
