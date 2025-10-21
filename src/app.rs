/// Main TUI application

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::io;
use std::time::{Duration, Instant};

use crate::core::{ConfigManager, DockerManager};
use crate::core::wallet::WalletManager;
use crate::core::ssl::SslManager;
use crate::screens::Dashboard;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Services,
    Wallets,
    Watch,
    Logs,
    Config,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServicesView {
    Services,
    Profiles,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigSection {
    Environment,
    RpcTokens,
    SslCerts,
}

impl Screen {
    pub fn title(&self) -> &'static str {
        match self {
            Screen::Services => "Services",
            Screen::Wallets => "Wallets",
            Screen::Watch => "Watch",
            Screen::Logs => "Logs",
            Screen::Config => "Configuration",
        }
    }

    pub fn all() -> &'static [Screen] {
        &[
            Screen::Services,
            Screen::Wallets,
            Screen::Watch,
            Screen::Logs,
            Screen::Config,
        ]
    }
}

#[derive(Debug, Clone)]
pub struct SystemResources {
    pub cpu_percent: f32,
    pub memory_used_gb: f32,
    pub memory_total_gb: f32,
    pub disk_free_gb: f32,
    pub disk_total_gb: f32,
    pub os_name: String,
    pub os_version: String,
    pub cpu_cores: usize,
    pub cpu_frequency_ghz: f32,
    pub cpu_model: String,
    pub public_ip: Option<String>,
}

pub struct App {
    dashboard: Dashboard,
    docker: DockerManager,
    config: ConfigManager,
    wallet_manager: WalletManager,
    ssl_manager: SslManager,
    current_screen: Screen,
    selected_index: usize,
    should_quit: bool,
    last_refresh: Instant,
    refresh_interval: Duration,
    status_message: Option<String>,
    show_help: bool,
    // Background data refresh channels
    container_data_rx: tokio::sync::mpsc::UnboundedReceiver<Vec<crate::core::docker::ContainerInfo>>,
    container_stats_rx: tokio::sync::mpsc::UnboundedReceiver<std::collections::HashMap<String, crate::core::docker::ContainerStats>>,
    image_versions_rx: tokio::sync::mpsc::UnboundedReceiver<std::collections::HashMap<String, crate::core::versions::ImageVersion>>,
    // Watch screen channels
    watch_transactions_tx: tokio::sync::mpsc::UnboundedSender<Vec<crate::core::l2_monitor::TransactionInfo>>,
    watch_transactions_rx: tokio::sync::mpsc::UnboundedReceiver<Vec<crate::core::l2_monitor::TransactionInfo>>,
    watch_stats_tx: tokio::sync::mpsc::UnboundedSender<crate::core::l2_monitor::Statistics>,
    watch_stats_rx: tokio::sync::mpsc::UnboundedReceiver<crate::core::l2_monitor::Statistics>,
    // Logs live mode channels
    logs_live_tx: tokio::sync::mpsc::UnboundedSender<Vec<String>>,
    logs_live_rx: tokio::sync::mpsc::UnboundedReceiver<Vec<String>>,
    logs_live_task_handle: Option<tokio::task::JoinHandle<()>>,
    // Cached data for actions
    containers: Vec<crate::core::docker::ContainerInfo>,
    container_stats: std::collections::HashMap<String, crate::core::docker::ContainerStats>,
    image_versions: std::collections::HashMap<String, crate::core::versions::ImageVersion>,
    reth_metrics: Option<crate::core::reth_metrics::RethMetrics>,
    reth_metrics_timestamp: Option<Instant>,
    wallets: Vec<crate::core::wallet::WalletInfo>,
    config_data: Vec<(String, String)>,
    active_profiles: Vec<String>,
    ssl_cert_info: Option<crate::core::ssl::CertificateInfo>,
    ssl_domain: String,
    system_resources: SystemResources,
    // Config editing state
    edit_mode: bool,
    edit_buffer: String,
    edit_key: Option<String>,
    // Service detail view state
    detail_view_service: Option<String>,
    detail_logs: Vec<String>,
    // Wallet detail view state
    detail_view_wallet: Option<usize>, // worker_id
    detail_wallet_addresses: Vec<(String, f64, f64)>, // (address, available, pending)
    detail_wallet_utxos: Vec<crate::core::wallet::UtxoInfo>, // UTXOs for activity view
    // Logs viewer state
    logs_selected_service: Option<String>,
    logs_data: Vec<String>,
    logs_follow_mode: bool,
    logs_filter: Option<String>, // None, "ERROR", "WARN", "INFO"
    logs_scroll_offset: usize,
    // Search/filter state
    search_mode: bool,
    search_buffer: String,
    filtered_indices: Vec<usize>, // Indices of items that match search
    // Wallet transaction state
    show_send_dialog: bool,
    send_amount: String,
    send_address: String,
    send_input_field: usize, // 0 = amount, 1 = address
    send_use_wallet_selector: bool, // Toggle between manual address entry and wallet selection
    send_selected_wallet_index: usize, // Index of selected wallet for destination
    send_source_address: String, // Source wallet address to display
    // New feature states
    detail_wallet_scroll: usize, // Scroll offset for transaction list
    detail_addresses_scroll: usize, // Scroll offset for addresses
    show_tx_detail: bool, // Transaction detail modal
    selected_tx_index: Option<usize>, // Selected transaction for detail view
    tx_search_mode: bool, // Transaction search/filter mode
    tx_search_buffer: String, // Search query for transactions
    filtered_tx_indices: Vec<usize>, // Filtered transaction indices
    auto_refresh_enabled: bool, // Auto-refresh toggle
    color_theme: String, // Color theme name
    // New v0.5.0 dashboard reorganization states
    services_view: ServicesView, // Services/Profiles tab view
    config_section: ConfigSection, // Config multi-tab section
    logs_live_mode: bool, // Live/realtime log streaming
    logs_compact_mode: bool, // Compact vs detailed log view (default: true)
    logs_grouping_enabled: bool, // Group logs by level/module (default: true)
    // Watch screen state
    watch_monitor: Option<std::sync::Arc<crate::core::l2_monitor::TransactionMonitor>>,
    watch_transactions: Vec<crate::core::l2_monitor::TransactionInfo>,
    watch_statistics: Option<crate::core::l2_monitor::Statistics>,
    watch_filter: crate::screens::watch::TransactionFilter,
    watch_scroll_offset: usize,
    watch_recording_file: Option<std::fs::File>,
    watch_recording_format: String,
}

impl App {
    pub fn new() -> Result<Self> {
        let docker = DockerManager::new_sync()?;
        let config = ConfigManager::load_from_project()?;
        let wallet_manager = WalletManager::new()?;
        let ssl_manager = SslManager::new()?;

        // Get domain from config
        let ssl_domain = config.get("IGRA_ORCHESTRA_DOMAIN")
            .unwrap_or("N/A")
            .to_string();

        // Create channels for background updates
        let (container_data_tx, container_data_rx) = tokio::sync::mpsc::unbounded_channel();
        let (container_stats_tx, container_stats_rx) = tokio::sync::mpsc::unbounded_channel();
        let (image_versions_tx, image_versions_rx) = tokio::sync::mpsc::unbounded_channel();
        let (watch_transactions_tx, watch_transactions_rx) = tokio::sync::mpsc::unbounded_channel();
        let (watch_stats_tx, watch_stats_rx) = tokio::sync::mpsc::unbounded_channel();
        let (logs_live_tx, logs_live_rx) = tokio::sync::mpsc::unbounded_channel();

        // Spawn background task to fetch container data
        let docker_clone = docker.clone();
        tokio::spawn(async move {
            loop {
                // Fetch container data with metrics (includes parallel log parsing)
                if let Ok(containers) = docker_clone.list_containers().await {
                    // Send to main thread (non-blocking send)
                    let _ = container_data_tx.send(containers);
                }

                // Wait 2 seconds before next update
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        });

        // Spawn background task to fetch container stats
        let docker_clone2 = docker.clone();
        tokio::spawn(async move {
            loop {
                // Small delay before first stats collection
                tokio::time::sleep(Duration::from_millis(500)).await;

                // Fetch current running containers from Docker
                if let Ok(containers) = docker_clone2.list_containers().await {
                    use std::collections::HashMap;
                    use futures::future::join_all;

                    let running_containers: Vec<String> = containers
                        .iter()
                        .filter(|c| c.state.is_running())
                        .map(|c| c.name.clone())
                        .collect();

                    // Fetch stats in parallel
                    let stats_futures = running_containers.iter().map(|name| {
                        let docker = docker_clone2.clone();
                        let name = name.clone();
                        async move {
                            docker.get_container_stats(&name).await.ok().flatten().map(|stats| (name, stats))
                        }
                    });

                    let stats_results = join_all(stats_futures).await;

                    // Build stats map
                    let mut stats_map = HashMap::new();
                    for result in stats_results.into_iter().flatten() {
                        stats_map.insert(result.0, result.1);
                    }

                    // Send to main thread
                    let _ = container_stats_tx.send(stats_map);
                }

                // Wait 2 seconds before next update
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        });

        // Spawn background task to check image versions (runs every 5 minutes)
        let docker_clone3 = docker.clone();
        tokio::spawn(async move {
            // Initial immediate check
            if let Ok(containers) = docker_clone3.list_containers().await {
                use std::collections::HashMap;

                let mut current_images = HashMap::new();
                for container in &containers {
                    // Extract image name and current tag
                    let image_str = container.image
                        .split('/')
                        .last()
                        .unwrap_or(&container.image);

                    let (name, tag) = if let Some((n, t)) = image_str.split_once(':') {
                        (n.to_string(), t.to_string())
                    } else {
                        (image_str.to_string(), "latest".to_string())
                    };

                    current_images.insert(name, tag);
                }

                // Check versions (async HTTP calls)
                let versions = crate::core::versions::check_versions(current_images).await;
                // Send to main thread
                let _ = image_versions_tx.send(versions);
            }

            loop {
                // Wait 5 minutes before next check
                tokio::time::sleep(Duration::from_secs(300)).await;

                // Fetch current containers and extract images
                if let Ok(containers) = docker_clone3.list_containers().await {
                    use std::collections::HashMap;

                    let mut current_images = HashMap::new();
                    for container in &containers {
                        // Extract image name and current tag
                        let image_str = container.image
                            .split('/')
                            .last()
                            .unwrap_or(&container.image);

                        let (name, tag) = if let Some((n, t)) = image_str.split_once(':') {
                            (n.to_string(), t.to_string())
                        } else {
                            (image_str.to_string(), "latest".to_string())
                        };

                        current_images.insert(name, tag);
                    }

                    // Check versions (async HTTP calls)
                    let versions = crate::core::versions::check_versions(current_images).await;
                    // Send to main thread
                    let _ = image_versions_tx.send(versions);
                }
            }
        });

        // Create dashboard and initialize with network info
        let mut dashboard = Dashboard::new();
        dashboard.update_network(docker.network().to_string());

        Ok(Self {
            dashboard,
            docker,
            config,
            wallet_manager,
            ssl_manager,
            current_screen: Screen::Services,
            selected_index: 0,
            should_quit: false,
            last_refresh: Instant::now(),
            refresh_interval: Duration::from_secs(2),
            status_message: None,
            show_help: false,
            container_data_rx,
            container_stats_rx,
            image_versions_rx,
            watch_transactions_tx,
            watch_transactions_rx,
            watch_stats_tx,
            watch_stats_rx,
            logs_live_tx,
            logs_live_rx,
            logs_live_task_handle: None,
            containers: Vec::new(),
            container_stats: std::collections::HashMap::new(),
            image_versions: std::collections::HashMap::new(),
            reth_metrics: None,
            reth_metrics_timestamp: None,
            wallets: Vec::new(),
            config_data: Vec::new(),
            active_profiles: Vec::new(),
            ssl_cert_info: None,
            ssl_domain,
            system_resources: SystemResources {
                cpu_percent: 0.0,
                memory_used_gb: 0.0,
                memory_total_gb: 0.0,
                disk_free_gb: 0.0,
                disk_total_gb: 0.0,
                os_name: String::new(),
                os_version: String::new(),
                cpu_cores: 0,
                cpu_frequency_ghz: 0.0,
                cpu_model: String::new(),
                public_ip: None,
            },
            edit_mode: false,
            edit_buffer: String::new(),
            edit_key: None,
            detail_view_service: None,
            detail_logs: Vec::new(),
            detail_view_wallet: None,
            detail_wallet_addresses: Vec::new(),
            detail_wallet_utxos: Vec::new(),
            logs_selected_service: None,
            logs_data: Vec::new(),
            logs_follow_mode: false,
            logs_filter: None,
            logs_scroll_offset: 0,
            search_mode: false,
            search_buffer: String::new(),
            filtered_indices: Vec::new(),
            show_send_dialog: false,
            send_amount: String::new(),
            send_address: String::new(),
            send_input_field: 0,
            send_use_wallet_selector: false,
            send_selected_wallet_index: 0,
            send_source_address: String::new(),
            // New feature initializations
            detail_wallet_scroll: 0,
            detail_addresses_scroll: 0,
            show_tx_detail: false,
            selected_tx_index: None,
            tx_search_mode: false,
            tx_search_buffer: String::new(),
            filtered_tx_indices: Vec::new(),
            auto_refresh_enabled: true,
            color_theme: "dark".to_string(),
            // New v0.5.0 dashboard reorganization initializations
            services_view: ServicesView::Services,
            config_section: ConfigSection::Environment,
            logs_live_mode: false,
            logs_compact_mode: true, // Default to compact view
            logs_grouping_enabled: true, // Default to grouped view
            watch_monitor: None,
            watch_transactions: Vec::new(),
            watch_statistics: None,
            watch_filter: crate::screens::watch::TransactionFilter::All,
            watch_scroll_offset: 0,
            watch_recording_file: None,
            watch_recording_format: "text".to_string(),
        })
    }

    fn collect_system_resources() -> SystemResources {
        use std::process::Command;

        // Get CPU usage
        let cpu_percent = Command::new("sh")
            .arg("-c")
            .arg("top -bn1 | grep 'Cpu(s)' | sed 's/.*, *\\([0-9.]*\\)%* id.*/\\1/' | awk '{print 100 - $1}'")
            .output()
            .ok()
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .and_then(|s| s.trim().parse::<f32>().ok())
            .unwrap_or(0.0);

        // Get memory usage
        let mem_output = Command::new("sh")
            .arg("-c")
            .arg("free -g | grep Mem")
            .output()
            .ok()
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .unwrap_or_default();

        let mem_parts: Vec<&str> = mem_output.split_whitespace().collect();
        let memory_total_gb = mem_parts.get(1).and_then(|s| s.parse::<f32>().ok()).unwrap_or(0.0);
        let memory_used_gb = mem_parts.get(2).and_then(|s| s.parse::<f32>().ok()).unwrap_or(0.0);

        // Get disk usage for root
        let disk_output = Command::new("sh")
            .arg("-c")
            .arg("df -BG / | tail -1")
            .output()
            .ok()
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .unwrap_or_default();

        let disk_parts: Vec<&str> = disk_output.split_whitespace().collect();
        let disk_total_gb = disk_parts.get(1)
            .and_then(|s| s.trim_end_matches('G').parse::<f32>().ok())
            .unwrap_or(0.0);
        let disk_free_gb = disk_parts.get(3)
            .and_then(|s| s.trim_end_matches('G').parse::<f32>().ok())
            .unwrap_or(0.0);

        // Get OS info from /etc/os-release
        let os_info = Command::new("sh")
            .arg("-c")
            .arg("grep -E '^(NAME|VERSION)=' /etc/os-release | head -2")
            .output()
            .ok()
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .unwrap_or_default();

        let mut os_name = String::from("Linux");
        let mut os_version = String::new();
        for line in os_info.lines() {
            if line.starts_with("NAME=") {
                os_name = line.strip_prefix("NAME=").unwrap_or("Linux")
                    .trim_matches('"').to_string();
            } else if line.starts_with("VERSION=") {
                os_version = line.strip_prefix("VERSION=").unwrap_or("")
                    .trim_matches('"').to_string();
            }
        }

        // Get CPU info from lscpu
        let cpu_info = Command::new("lscpu")
            .output()
            .ok()
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .unwrap_or_default();

        let mut cpu_cores = 0usize;
        let mut cpu_frequency_ghz = 0.0f32;
        let mut cpu_model = String::from("Unknown CPU");

        for line in cpu_info.lines() {
            if line.starts_with("CPU(s):") {
                cpu_cores = line.split_whitespace()
                    .nth(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
            } else if line.starts_with("CPU max MHz:") || line.starts_with("CPU MHz:") {
                let mhz = line.split_whitespace()
                    .nth(2)
                    .and_then(|s| s.parse::<f32>().ok())
                    .unwrap_or(0.0);
                cpu_frequency_ghz = mhz / 1000.0;
            } else if line.starts_with("Model name:") {
                cpu_model = line.split(':')
                    .nth(1)
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|| "Unknown CPU".to_string());
            }
        }

        // Get public IP (non-blocking, use cached value on failure)
        let public_ip = Command::new("curl")
            .args(&["-s", "--max-time", "2", "https://api.ipify.org"])
            .output()
            .ok()
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .filter(|ip| !ip.is_empty() && ip.len() < 50); // Sanity check

        SystemResources {
            cpu_percent,
            memory_used_gb,
            memory_total_gb,
            disk_free_gb,
            disk_total_gb,
            os_name,
            os_version,
            cpu_cores,
            cpu_frequency_ghz,
            cpu_model,
            public_ip,
        }
    }

    /// Update dashboard with existing cached data (non-blocking, no async calls)
    fn update_dashboard_for_current_screen(&mut self) {
        match self.current_screen {
            Screen::Services => {
                // Update both services and profiles (merged screen)
                self.dashboard.update_services(
                    self.containers.clone(),
                    self.active_profiles.clone(),
                    self.container_stats.clone(),
                    self.image_versions.clone()
                );
                self.dashboard.update_profiles(self.active_profiles.clone());
            }
            Screen::Wallets => {
                // Use cached wallet data - will be refreshed by periodic timer
                self.dashboard.update_wallets(self.wallets.clone());
            }
            Screen::Watch => {
                // Watch screen handles its own updates via monitor
            }
            Screen::Logs => {
                // Logs are handled separately
            }
            Screen::Config => {
                // Update all config sections (environment, RPC tokens, SSL)
                self.config_data = self.config.keys()
                    .into_iter()
                    .map(|k| {
                        let val = self.config.get(&k).unwrap_or("");
                        (k.clone(), val.to_string())
                    })
                    .collect();
                self.dashboard.update_config(self.config_data.clone());

                let tokens = self.config.get_rpc_tokens();
                let domain = self.config.get("IGRA_ORCHESTRA_DOMAIN")
                    .unwrap_or("N/A")
                    .to_string();
                self.dashboard.update_rpc_tokens(tokens, domain);

                self.dashboard.update_ssl(self.ssl_cert_info.clone());
            }
        }
    }

    fn set_status(&mut self, message: String) {
        self.status_message = Some(message);
    }

    fn clear_status(&mut self) {
        self.status_message = None;
    }

    pub async fn run(&mut self) -> Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Initial data load
        self.refresh_data().await?;

        let result = self.run_loop(&mut terminal).await;

        // Restore terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen
        )?;
        terminal.show_cursor()?;

        result
    }

    async fn refresh_data(&mut self) -> Result<()> {
        // NOTE: Container list and stats are now updated in background tasks
        // Only refresh system resources and screen-specific data here
        self.system_resources = Self::collect_system_resources();

        // Update Reth metrics if viewing execution-layer detail
        if let Some(ref service) = self.detail_view_service {
            if service == "execution-layer" {
                let _ = self.update_reth_metrics().await;
            }
        }

        // Update dashboard based on current screen
        match self.current_screen {
            Screen::Services => {
                // Container data already updated by background task
                // Update both services and profiles views
                self.dashboard.update_services(
                    self.containers.clone(),
                    self.active_profiles.clone(),
                    self.container_stats.clone(),
                    self.image_versions.clone()
                );
                self.dashboard.update_profiles(self.active_profiles.clone());
            }
            Screen::Wallets => {
                self.wallets = self.wallet_manager.list_wallets().await?;
                self.dashboard.update_wallets(self.wallets.clone());
            }
            Screen::Watch => {
                // Watch screen updates handled by monitor
            }
            Screen::Logs => {
                // Refresh logs if in follow mode or live mode
                if self.logs_follow_mode || self.logs_live_mode {
                    if let Some(service_name) = &self.logs_selected_service {
                        match self.docker.get_logs(service_name, Some(500)).await {
                            Ok(logs) => {
                                self.logs_data = logs.lines().map(|s| s.to_string()).collect();
                                // Auto-scroll to bottom in follow/live mode
                                self.logs_scroll_offset = self.logs_data.len().saturating_sub(50);
                            }
                            Err(_) => {
                                // Silently fail - don't interrupt user experience
                            }
                        }
                    }
                }
            }
            Screen::Config => {
                // Update all config sections
                self.config_data = self.config.keys()
                    .into_iter()
                    .map(|k| {
                        let val = self.config.get(&k).unwrap_or("");
                        (k.clone(), val.to_string())
                    })
                    .collect();
                self.dashboard.update_config(self.config_data.clone());

                let tokens = self.config.get_rpc_tokens();
                let domain = self.config.get("IGRA_ORCHESTRA_DOMAIN")
                    .unwrap_or("N/A")
                    .to_string();
                self.dashboard.update_rpc_tokens(tokens, domain);

                // Load SSL certificate info
                if self.ssl_domain != "N/A" {
                    match self.ssl_manager.get_certificate_info(&self.ssl_domain).await {
                        Ok(cert_info) => {
                            self.dashboard.update_ssl(Some(cert_info.clone()));
                            self.ssl_cert_info = Some(cert_info);
                        }
                        Err(_) => {
                            self.dashboard.update_ssl(None);
                            self.ssl_cert_info = None;
                        }
                    }
                } else {
                    self.dashboard.update_ssl(None);
                }
            }
        }

        self.last_refresh = Instant::now();
        Ok(())
    }

    /// Fetch and update Reth metrics with TPS calculation
    async fn update_reth_metrics(&mut self) -> Result<()> {
        // Fetch current metrics
        let mut current_metrics = crate::core::reth_metrics::fetch_reth_metrics().await?;

        // Calculate TPS if we have previous metrics
        if let (Some(ref prev_metrics), Some(prev_timestamp)) =
            (&self.reth_metrics, self.reth_metrics_timestamp) {
            let elapsed = prev_timestamp.elapsed().as_secs_f64();
            if let Some(tps) = crate::core::reth_metrics::calculate_tps(
                &current_metrics,
                prev_metrics,
                elapsed
            ) {
                current_metrics.tps = Some(tps);
            }
        }

        // Store current metrics and timestamp for next calculation
        self.reth_metrics_timestamp = Some(Instant::now());
        self.reth_metrics = Some(current_metrics);

        Ok(())
    }

    async fn run_loop<B: ratatui::backend::Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
    ) -> Result<()> {
        loop {
            // Check for new container data from background task (non-blocking)
            while let Ok(containers) = self.container_data_rx.try_recv() {
                self.containers = containers;
                // Derive profiles synchronously from container list (no blocking!)
                self.active_profiles = DockerManager::get_active_profiles_from_list(&self.containers);

                // Update dashboard with new container data
                if self.current_screen == Screen::Services {
                    self.dashboard.update_services(
                        self.containers.clone(),
                        self.active_profiles.clone(),
                        self.container_stats.clone(),
                        self.image_versions.clone()
                    );
                }
            }

            // Check for new container stats from background task (non-blocking)
            while let Ok(stats) = self.container_stats_rx.try_recv() {
                self.container_stats = stats;

                // Update dashboard with new stats
                if self.current_screen == Screen::Services {
                    self.dashboard.update_services(
                        self.containers.clone(),
                        self.active_profiles.clone(),
                        self.container_stats.clone(),
                        self.image_versions.clone()
                    );
                }
            }

            // Check for new image versions from background task (non-blocking)
            while let Ok(versions) = self.image_versions_rx.try_recv() {
                self.image_versions = versions;

                // Update dashboard with new version info
                if self.current_screen == Screen::Services {
                    self.dashboard.update_services(
                        self.containers.clone(),
                        self.active_profiles.clone(),
                        self.container_stats.clone(),
                        self.image_versions.clone()
                    );
                }
            }

            // Check for new watch transactions from background task (non-blocking)
            while let Ok(new_txs) = self.watch_transactions_rx.try_recv() {
                let tx_count = new_txs.len(); // Save count before moving

                // Add new transactions to the beginning (newest first)
                for tx in new_txs.into_iter().rev() {
                    self.watch_transactions.insert(0, tx);
                }
                // Keep only last 100 transactions
                if self.watch_transactions.len() > 100 {
                    self.watch_transactions.truncate(100);
                }

                // Record transactions to file if enabled
                if let Some(ref mut file) = self.watch_recording_file {
                    for tx in &self.watch_transactions[0..self.watch_transactions.len().min(tx_count)] {
                        let _ = Self::write_transaction_to_file(file, tx, &self.watch_recording_format);
                    }
                }
            }

            // Check for new watch statistics from background task (non-blocking)
            while let Ok(stats) = self.watch_stats_rx.try_recv() {
                self.watch_statistics = Some(stats);
            }

            // Check for new logs from live mode background task (non-blocking)
            while let Ok(new_logs) = self.logs_live_rx.try_recv() {
                self.logs_data = new_logs;
                // Auto-scroll to bottom in live mode
                self.logs_scroll_offset = self.logs_data.len().saturating_sub(50);
            }

            // Refresh non-container data periodically
            if self.last_refresh.elapsed() >= self.refresh_interval {
                if let Err(e) = self.refresh_data().await {
                    // Show error but don't crash
                    eprintln!("Failed to refresh data: {}", e);
                }
            }

            terminal.draw(|f| self.render(f))?;

            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key_event) = event::read()? {
                    self.handle_key(key_event).await?;
                }
            }

            if self.should_quit {
                break;
            }
        }

        Ok(())
    }

    async fn handle_key(&mut self, key_event: event::KeyEvent) -> Result<()> {
        let key = key_event.code;
        let modifiers = key_event.modifiers;

        // Handle edit mode separately
        if self.edit_mode {
            return self.handle_edit_key(key).await;
        }

        // Handle transaction search mode separately
        if self.tx_search_mode {
            return self.handle_tx_search_key(key).await;
        }

        // Handle search mode separately
        if self.search_mode {
            return self.handle_search_key(key).await;
        }

        // Handle send dialog separately
        if self.show_send_dialog {
            return self.handle_send_dialog_key(key).await;
        }

        // Handle detail view separately
        if self.detail_view_service.is_some() || self.detail_view_wallet.is_some() {
            return self.handle_detail_view_key(key).await;
        }

        // Clear status message on any key (when not in edit mode, search mode, send dialog, or detail view)
        self.clear_status();

        match key {
            KeyCode::Char('q') => {
                self.should_quit = true;
            }
            KeyCode::Esc => {
                // Close help if it's showing, go back from logs viewer, or quit
                if self.show_help {
                    self.show_help = false;
                } else if self.current_screen == Screen::Logs && self.logs_selected_service.is_some() {
                    // Go back to service list
                    self.logs_selected_service = None;
                    self.logs_data.clear();
                    self.logs_filter = None;
                    self.logs_follow_mode = false;
                    self.logs_live_mode = false;
                    self.stop_logs_live_mode(); // Stop live mode task
                    self.logs_scroll_offset = 0;
                } else {
                    self.should_quit = true;
                }
            }
            KeyCode::Char('?') | KeyCode::F(1) => {
                self.show_help = !self.show_help;
            }
            KeyCode::Char('r') => {
                // Refresh logs if in logs viewer, otherwise refresh all data
                if self.current_screen == Screen::Logs && self.logs_selected_service.is_some() {
                    let service_name = self.logs_selected_service.as_ref().unwrap().clone();
                    self.set_status(format!("Refreshing logs for {}...", service_name));

                    match self.docker.get_logs(&service_name, Some(500)).await {
                        Ok(logs) => {
                            self.logs_data = logs.lines().map(|s| s.to_string()).collect();
                            self.clear_status();
                        }
                        Err(e) => {
                            self.set_status(format!("✗ Failed to refresh logs: {}", e));
                        }
                    }
                } else {
                    self.set_status("Refreshing...".to_string());
                    self.refresh_data().await?;
                }
            }
            KeyCode::Right => {
                // Right arrow: Navigate to next main screen
                self.next_screen();
                self.selected_index = 0;
                self.update_dashboard_for_current_screen();
            }
            KeyCode::Left => {
                // Left arrow: Navigate to previous main screen
                self.prev_screen();
                self.selected_index = 0;
                self.update_dashboard_for_current_screen();
            }
            KeyCode::Tab => {
                // Tab: Navigate to next sub-view within Services or Config screens only
                match self.current_screen {
                    Screen::Services => {
                        self.services_view = match self.services_view {
                            ServicesView::Services => ServicesView::Profiles,
                            ServicesView::Profiles => ServicesView::Services,
                        };
                        self.selected_index = 0;
                    }
                    Screen::Config => {
                        self.config_section = match self.config_section {
                            ConfigSection::Environment => ConfigSection::RpcTokens,
                            ConfigSection::RpcTokens => ConfigSection::SslCerts,
                            ConfigSection::SslCerts => ConfigSection::Environment,
                        };
                        self.selected_index = 0;
                    }
                    _ => {
                        // On other screens, Tab does nothing
                    }
                }
            }
            KeyCode::BackTab => {
                // Shift+Tab: Navigate to previous sub-view within Services or Config screens only
                match self.current_screen {
                    Screen::Services => {
                        self.services_view = match self.services_view {
                            ServicesView::Services => ServicesView::Profiles,
                            ServicesView::Profiles => ServicesView::Services,
                        };
                        self.selected_index = 0;
                    }
                    Screen::Config => {
                        self.config_section = match self.config_section {
                            ConfigSection::Environment => ConfigSection::SslCerts,
                            ConfigSection::RpcTokens => ConfigSection::Environment,
                            ConfigSection::SslCerts => ConfigSection::RpcTokens,
                        };
                        self.selected_index = 0;
                    }
                    _ => {
                        // On other screens, Shift+Tab does nothing
                    }
                }
            }
            KeyCode::Char('1') => {
                self.current_screen = Screen::Services;
                self.selected_index = 0;
                self.update_dashboard_for_current_screen();
            }
            KeyCode::Char('2') => {
                self.current_screen = Screen::Wallets;
                self.selected_index = 0;
                self.update_dashboard_for_current_screen();
            }
            KeyCode::Char('3') => {
                self.current_screen = Screen::Watch;
                self.selected_index = 0;
                self.update_dashboard_for_current_screen();
                // Initialize watch monitor and start background polling if not already initialized
                if self.watch_monitor.is_none() {
                    if let Ok(monitor) = crate::core::l2_monitor::TransactionMonitor::new_sync() {
                        let monitor_arc = std::sync::Arc::new(monitor);
                        self.watch_monitor = Some(monitor_arc.clone());

                        // Spawn background polling task for Watch screen
                        let watch_tx_tx = self.watch_transactions_tx.clone();
                        let watch_stats_tx = self.watch_stats_tx.clone();
                        tokio::spawn(async move {
                            let mut poll_interval = tokio::time::interval(Duration::from_secs(1));
                            let mut l1_interval = tokio::time::interval(Duration::from_secs(10));

                            loop {
                                tokio::select! {
                                    _ = poll_interval.tick() => {
                                        // Poll for new transactions
                                        if let Ok(new_txs) = monitor_arc.poll_new_transactions().await {
                                            if !new_txs.is_empty() {
                                                let _ = watch_tx_tx.send(new_txs);
                                            }
                                        }

                                        // Get current statistics
                                        let stats = monitor_arc.get_statistics().await;
                                        let _ = watch_stats_tx.send(stats);
                                    }
                                    _ = l1_interval.tick() => {
                                        // Update L1 data periodically
                                        let _ = monitor_arc.update_l1_data().await;
                                    }
                                }
                            }
                        });
                    }
                }
            }
            KeyCode::Char('4') => {
                self.current_screen = Screen::Logs;
                self.selected_index = 0;
                self.update_dashboard_for_current_screen();
            }
            KeyCode::Char('5') => {
                self.current_screen = Screen::Config;
                self.selected_index = 0;
                self.update_dashboard_for_current_screen();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                use event::KeyModifiers;

                // Determine scroll amount based on modifiers
                let is_ctrl = modifiers.contains(KeyModifiers::CONTROL);
                let is_shift = modifiers.contains(KeyModifiers::SHIFT);

                // Select transaction in wallet detail view
                if self.detail_view_wallet.is_some() && !self.detail_wallet_utxos.is_empty() {
                    let current_selection = self.selected_tx_index.unwrap_or(0);
                    if is_ctrl && is_shift {
                        // Jump to beginning
                        self.selected_tx_index = Some(0);
                    } else if is_ctrl {
                        // Fast scroll up (10 items)
                        self.selected_tx_index = Some(current_selection.saturating_sub(10));
                    } else if current_selection > 0 {
                        // Normal scroll
                        self.selected_tx_index = Some(current_selection - 1);
                    }
                }
                // Scroll logs if in logs viewer
                else if self.current_screen == Screen::Logs && self.logs_selected_service.is_some() {
                    if is_ctrl && is_shift {
                        // Jump to top
                        self.logs_scroll_offset = 0;
                    } else if is_ctrl {
                        // Fast scroll up (10 lines)
                        self.logs_scroll_offset = self.logs_scroll_offset.saturating_sub(10);
                    } else {
                        // Normal scroll
                        self.logs_scroll_offset = self.logs_scroll_offset.saturating_sub(1);
                    }
                }
                // Move selection
                else if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                use event::KeyModifiers;

                // Determine scroll amount based on modifiers
                let is_ctrl = modifiers.contains(KeyModifiers::CONTROL);
                let is_shift = modifiers.contains(KeyModifiers::SHIFT);

                // Select transaction in wallet detail view
                if self.detail_view_wallet.is_some() && !self.detail_wallet_utxos.is_empty() {
                    let tx_count = if self.tx_search_mode && !self.filtered_tx_indices.is_empty() {
                        self.filtered_tx_indices.len()
                    } else {
                        self.detail_wallet_utxos.len()
                    };
                    let current_selection = self.selected_tx_index.unwrap_or(0);
                    let max_idx = tx_count.saturating_sub(1);

                    if is_ctrl && is_shift {
                        // Jump to end
                        self.selected_tx_index = Some(max_idx);
                    } else if is_ctrl {
                        // Fast scroll down (10 items)
                        self.selected_tx_index = Some((current_selection + 10).min(max_idx));
                    } else if current_selection < max_idx {
                        // Normal scroll
                        self.selected_tx_index = Some(current_selection + 1);
                    }
                }
                // Scroll logs if in logs viewer
                else if self.current_screen == Screen::Logs && self.logs_selected_service.is_some() {
                    let max_scroll = self.logs_data.len().saturating_sub(10);

                    if is_ctrl && is_shift {
                        // Jump to bottom
                        self.logs_scroll_offset = max_scroll;
                    } else if is_ctrl {
                        // Fast scroll down (10 lines)
                        self.logs_scroll_offset = (self.logs_scroll_offset + 10).min(max_scroll);
                    } else {
                        // Normal scroll
                        self.logs_scroll_offset = (self.logs_scroll_offset + 1).min(max_scroll);
                    }
                }
                // Move selection
                else {
                    let max = self.get_max_selection();
                    if self.selected_index < max {
                        self.selected_index += 1;
                    }
                }
            }
            KeyCode::Enter => {
                // In wallet detail view, Enter toggles transaction detail modal
                if self.detail_view_wallet.is_some() && !self.detail_wallet_utxos.is_empty() {
                    if self.show_tx_detail {
                        // Close transaction detail modal
                        self.show_tx_detail = false;
                    } else {
                        // Open transaction detail modal for selected transaction
                        // Initialize selection to 0 if not set
                        if self.selected_tx_index.is_none() {
                            self.selected_tx_index = Some(0);
                        }
                        self.show_tx_detail = true;
                    }
                } else {
                    self.handle_action().await?;
                }
            }
            KeyCode::Char(' ') => {
                // Space for toggle on Services/Profiles view
                if self.current_screen == Screen::Services && self.services_view == ServicesView::Profiles {
                    self.handle_action().await?;
                }
            }
            KeyCode::Char('s') => {
                // Quick action: Start
                match self.current_screen {
                    Screen::Services => {
                        if self.services_view == ServicesView::Services {
                            self.handle_service_start().await?;
                        } else {
                            self.handle_profile_start().await?;
                        }
                    }
                    _ => {}
                }
            }
            KeyCode::Char('x') => {
                // Quick action: Stop
                match self.current_screen {
                    Screen::Services => {
                        if self.services_view == ServicesView::Services {
                            self.handle_service_stop().await?;
                        } else {
                            self.handle_profile_stop().await?;
                        }
                    }
                    _ => {}
                }
            }
            KeyCode::Char('R') => {
                // Quick action: Restart (capital R)
                if self.current_screen == Screen::Services {
                    self.handle_service_restart().await?;
                }
            }
            KeyCode::Char('g') => {
                // Generate tokens / wallets
                match self.current_screen {
                    Screen::Config => {
                        if self.config_section == ConfigSection::RpcTokens {
                            self.handle_generate_tokens().await?;
                        }
                    }
                    Screen::Wallets => self.handle_generate_wallet().await?,
                    _ => {}
                }
            }
            KeyCode::Char('t') => {
                // Send transaction (Transfer)
                if self.current_screen == Screen::Wallets {
                    self.open_send_dialog();
                }
            }
            KeyCode::Char('e') => {
                // Edit config value or Filter ERROR in logs
                if self.current_screen == Screen::Config {
                    self.enter_edit_mode();
                } else if self.current_screen == Screen::Logs && self.logs_selected_service.is_some() {
                    self.logs_filter = Some("ERROR".to_string());
                    self.logs_scroll_offset = 0;
                    self.set_status("Filtering: ERROR".to_string());
                }
            }
            KeyCode::Char('c') => {
                // Check certificate (SSL) or Clear filter (Logs) or Clear transactions (Watch)
                if self.current_screen == Screen::Config && self.config_section == ConfigSection::SslCerts {
                    self.handle_ssl_check().await?;
                } else if self.current_screen == Screen::Logs && self.logs_selected_service.is_some() {
                    self.logs_filter = None;
                    self.set_status("Filter cleared".to_string());
                } else if self.current_screen == Screen::Watch {
                    self.watch_transactions.clear();
                    self.set_status("Transaction history cleared".to_string());
                }
            }
            KeyCode::Char('n') => {
                // Force renewal (reNew)
                if self.current_screen == Screen::Config && self.config_section == ConfigSection::SslCerts {
                    self.handle_ssl_renew().await?;
                }
            }
            KeyCode::Char('f') => {
                // Toggle follow mode in logs viewer (or filter toggle in Watch screen)
                if self.current_screen == Screen::Logs && self.logs_selected_service.is_some() {
                    self.logs_follow_mode = !self.logs_follow_mode;
                    let msg = if self.logs_follow_mode {
                        "Follow mode enabled - logs will auto-refresh"
                    } else {
                        "Follow mode disabled"
                    };
                    self.set_status(msg.to_string());
                } else if self.current_screen == Screen::Watch {
                    // Toggle transaction filter in Watch screen
                    use crate::screens::watch::TransactionFilter;
                    self.watch_filter = match self.watch_filter {
                        TransactionFilter::All => TransactionFilter::Transfer,
                        TransactionFilter::Transfer => TransactionFilter::Contract,
                        TransactionFilter::Contract => TransactionFilter::Entry,
                        TransactionFilter::Entry => TransactionFilter::All,
                    };
                }
            }
            KeyCode::Char('l') => {
                // Toggle live mode in logs viewer
                if self.current_screen == Screen::Logs && self.logs_selected_service.is_some() {
                    self.logs_live_mode = !self.logs_live_mode;

                    if self.logs_live_mode {
                        // Start background polling task for logs
                        self.start_logs_live_mode();
                        self.set_status("🔴 LIVE mode enabled - logs updating every 1s".to_string());
                    } else {
                        // Stop background polling task
                        self.stop_logs_live_mode();
                        self.set_status("Live mode disabled".to_string());
                    }
                }
            }
            KeyCode::Char('w') => {
                // Filter WARN in logs
                if self.current_screen == Screen::Logs && self.logs_selected_service.is_some() {
                    self.logs_filter = Some("WARN".to_string());
                    self.logs_scroll_offset = 0;
                    self.set_status("Filtering: WARN".to_string());
                }
            }
            KeyCode::Char('i') => {
                // Filter INFO in logs
                if self.current_screen == Screen::Logs && self.logs_selected_service.is_some() {
                    self.logs_filter = Some("INFO".to_string());
                    self.logs_scroll_offset = 0;
                    self.set_status("Filtering: INFO".to_string());
                }
            }
            KeyCode::Char('t') => {
                // Toggle compact/detailed log view
                if self.current_screen == Screen::Logs && self.logs_selected_service.is_some() {
                    self.logs_compact_mode = !self.logs_compact_mode;
                    let msg = if self.logs_compact_mode {
                        "Compact view enabled (time + level + message)"
                    } else {
                        "Detailed view enabled (full timestamp + service + level + message)"
                    };
                    self.set_status(msg.to_string());
                }
            }
            KeyCode::Char('g') => {
                // Toggle log grouping by level/module
                if self.current_screen == Screen::Logs && self.logs_selected_service.is_some() {
                    self.logs_grouping_enabled = !self.logs_grouping_enabled;
                    let msg = if self.logs_grouping_enabled {
                        "Log grouping enabled (group by level/module)"
                    } else {
                        "Log grouping disabled (chronological view)"
                    };
                    self.set_status(msg.to_string());
                }
            }
            KeyCode::PageUp => {
                // Page up in logs
                if self.current_screen == Screen::Logs && self.logs_selected_service.is_some() {
                    self.logs_scroll_offset = self.logs_scroll_offset.saturating_sub(20);
                }
            }
            KeyCode::PageDown => {
                // Page down in logs
                if self.current_screen == Screen::Logs && self.logs_selected_service.is_some() {
                    let max_scroll = self.logs_data.len().saturating_sub(10);
                    self.logs_scroll_offset = (self.logs_scroll_offset + 20).min(max_scroll);
                }
            }
            KeyCode::Char('u') => {
                // Upgrade (pull images)
                self.handle_upgrade().await?;
            }
            KeyCode::Char('/') => {
                // Enter search mode (on searchable screens or wallet detail view)
                if self.detail_view_wallet.is_some() {
                    // Transaction search mode in wallet detail
                    self.tx_search_mode = true;
                    self.tx_search_buffer.clear();
                    self.filtered_tx_indices.clear();
                    self.set_status("Search transactions: (type TxID, address, or amount)".to_string());
                } else if matches!(self.current_screen, Screen::Services | Screen::Config | Screen::Wallets) {
                    self.search_mode = true;
                    self.search_buffer.clear();
                    self.filtered_indices.clear();
                    self.set_status("Search: (type to filter, Enter to apply, Esc to cancel)".to_string());
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn get_max_selection(&self) -> usize {
        match self.current_screen {
            Screen::Services => {
                match self.services_view {
                    ServicesView::Services => self.containers.len().saturating_sub(1),
                    ServicesView::Profiles => 6, // kaspad, backend, frontend-w1 through w5 = 7 profiles (0-6)
                }
            }
            Screen::Wallets => self.wallets.len().saturating_sub(1),
            Screen::Watch => self.watch_transactions.len().saturating_sub(1),
            Screen::Config => {
                match self.config_section {
                    ConfigSection::Environment => self.config_data.len().saturating_sub(1),
                    ConfigSection::RpcTokens => 45, // 46 tokens, 0-45
                    ConfigSection::SslCerts => 0, // No selection in SSL section
                }
            }
            Screen::Logs => {
                if self.logs_selected_service.is_none() {
                    self.containers.len().saturating_sub(1)
                } else {
                    0 // No selection when viewing logs
                }
            }
        }
    }

    async fn handle_action(&mut self) -> Result<()> {
        match self.current_screen {
            Screen::Services => {
                match self.services_view {
                    ServicesView::Services => self.show_service_details().await,
                    ServicesView::Profiles => self.handle_profile_toggle().await,
                }
            }
            Screen::Wallets => self.show_wallet_details().await,
            Screen::Config => {
                match self.config_section {
                    ConfigSection::RpcTokens => self.handle_rpc_action().await,
                    _ => Ok(()),
                }
            }
            Screen::Logs => self.handle_logs_action().await,
            _ => Ok(()),
        }
    }

    async fn handle_logs_action(&mut self) -> Result<()> {
        if self.logs_selected_service.is_none() {
            // Select service to view logs
            if self.selected_index >= self.containers.len() {
                return Ok(());
            }

            let service_name = self.containers[self.selected_index].name.clone();
            self.set_status(format!("Loading logs for {}...", service_name));

            // Stop live mode if switching services
            if self.logs_live_mode {
                self.logs_live_mode = false;
                self.stop_logs_live_mode();
            }

            // Load initial logs (last 500 lines)
            match self.docker.get_logs(&service_name, Some(500)).await {
                Ok(logs) => {
                    self.logs_data = logs.lines().map(|s| s.to_string()).collect();
                    self.logs_selected_service = Some(service_name);
                    self.logs_scroll_offset = self.logs_data.len().saturating_sub(50); // Start near bottom
                    self.clear_status();
                }
                Err(e) => {
                    self.set_status(format!("✗ Failed to load logs: {}", e));
                }
            }
        }
        Ok(())
    }

    async fn show_service_details(&mut self) -> Result<()> {
        if self.selected_index >= self.containers.len() {
            return Ok(());
        }

        let service = self.containers[self.selected_index].name.clone();
        self.set_status(format!("Loading details for {}...", service));

        // Load logs (last 50 lines)
        match self.docker.get_logs(&service, Some(50)).await {
            Ok(logs) => {
                self.detail_logs = logs.lines().map(|s| s.to_string()).collect();
            }
            Err(_) => {
                self.detail_logs = vec!["Failed to load logs".to_string()];
            }
        }

        self.detail_view_service = Some(service);
        self.clear_status();

        Ok(())
    }

    async fn show_wallet_details(&mut self) -> Result<()> {
        if self.selected_index >= self.wallets.len() {
            return Ok(());
        }

        let wallet = &self.wallets[self.selected_index];
        let worker_id = wallet.worker_id;

        if !wallet.container_running {
            self.set_status(format!("Wallet {} container not running", worker_id));
            return Ok(());
        }

        self.set_status(format!("Loading wallet details for worker {}...", worker_id));

        // Fetch detailed balance info with per-address breakdown
        let address_balances = match self.wallet_manager.get_balance_detailed(worker_id).await {
            Ok(balances) => balances,
            Err(e) => {
                self.set_status(format!("✗ Failed to load wallet details: {}", e));
                self.detail_wallet_addresses = Vec::new();
                self.detail_wallet_utxos = Vec::new();
                return Ok(());
            }
        };

        // Fetch UTXOs for activity history
        let utxos = match self.wallet_manager.get_utxos(worker_id).await {
            Ok(utxos) => utxos,
            Err(e) => {
                // UTXOs are optional, continue without them
                eprintln!("Warning: Failed to load UTXOs: {}", e);
                Vec::new()
            }
        };

        self.detail_wallet_addresses = address_balances;
        self.detail_wallet_utxos = utxos;
        self.detail_view_wallet = Some(worker_id);
        self.detail_wallet_scroll = 0; // Reset scroll when opening wallet detail
        self.detail_addresses_scroll = 0;
        // Initialize transaction selection to first transaction
        self.selected_tx_index = if !self.detail_wallet_utxos.is_empty() {
            Some(0)
        } else {
            None
        };
        self.clear_status();

        Ok(())
    }

    async fn handle_generate_wallet(&mut self) -> Result<()> {
        if self.selected_index >= self.wallets.len() {
            return Ok(());
        }

        let wallet = &self.wallets[self.selected_index];
        let worker_id = wallet.worker_id;

        if !wallet.container_running {
            self.set_status(format!("✗ Wallet {} container not running. Start frontend profile first.", worker_id));
            return Ok(());
        }

        if wallet.address.is_some() {
            self.set_status(format!("✗ Wallet {} already exists", worker_id));
            return Ok(());
        }

        self.set_status(format!("Generating wallet for worker {}...", worker_id));

        // Use default password from config or "password"
        let password = self.config.get(&format!("W{}_KASWALLET_PASSWORD", worker_id))
            .unwrap_or("password");

        match self.wallet_manager.generate_wallet(worker_id, password).await {
            Ok(address) => {
                self.set_status(format!("✓ Generated wallet {}: {}", worker_id, address));
                // Refresh wallet data
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                self.refresh_data().await?;
            }
            Err(e) => {
                self.set_status(format!("✗ Failed to generate wallet {}: {}", worker_id, e));
            }
        }

        Ok(())
    }

    fn open_send_dialog(&mut self) {
        if self.selected_index >= self.wallets.len() {
            return;
        }

        let wallet = &self.wallets[self.selected_index];

        if !wallet.container_running {
            self.set_status(format!("✗ Wallet {} container not running", wallet.worker_id));
            return;
        }

        if wallet.address.is_none() {
            self.set_status(format!("✗ Wallet {} not generated yet", wallet.worker_id));
            return;
        }

        // Capture source address
        self.send_source_address = wallet.address.clone().unwrap_or_default();

        // Open the send dialog
        self.show_send_dialog = true;
        self.send_amount.clear();
        self.send_address.clear();
        self.send_input_field = 0;
        self.send_use_wallet_selector = false;
        self.send_selected_wallet_index = 0;
        self.set_status("Enter transaction details | Tab: switch | s: select wallet | Enter: send | Esc: cancel".to_string());
    }

    async fn handle_send_dialog_key(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Char('s') | KeyCode::Char('S') => {
                // Toggle wallet selector mode
                if self.send_input_field == 1 {
                    self.send_use_wallet_selector = !self.send_use_wallet_selector;
                    if self.send_use_wallet_selector {
                        // Clear manual address when switching to wallet selector
                        self.send_address.clear();
                        self.set_status("Use ↑↓ to select wallet | s: toggle manual entry | Tab: switch | Enter: send".to_string());
                    } else {
                        self.set_status("Enter address manually | s: toggle wallet selector | Tab: switch | Enter: send".to_string());
                    }
                }
            }
            KeyCode::Char(c) => {
                if self.send_input_field == 0 {
                    // Amount field - only allow numbers and decimal point
                    if c.is_ascii_digit() || c == '.' {
                        self.send_amount.push(c);
                    }
                } else if !self.send_use_wallet_selector {
                    // Address field (manual entry only)
                    self.send_address.push(c);
                }
            }
            KeyCode::Backspace => {
                if self.send_input_field == 0 {
                    self.send_amount.pop();
                } else if !self.send_use_wallet_selector {
                    self.send_address.pop();
                }
            }
            KeyCode::Up => {
                // Navigate wallet selector
                if self.send_input_field == 1 && self.send_use_wallet_selector {
                    if self.send_selected_wallet_index > 0 {
                        self.send_selected_wallet_index -= 1;
                    }
                }
            }
            KeyCode::Down => {
                // Navigate wallet selector
                if self.send_input_field == 1 && self.send_use_wallet_selector {
                    if self.send_selected_wallet_index + 1 < self.wallets.len() {
                        self.send_selected_wallet_index += 1;
                    }
                }
            }
            KeyCode::Tab => {
                // Switch between fields
                self.send_input_field = if self.send_input_field == 0 { 1 } else { 0 };
            }
            KeyCode::Enter => {
                // Send transaction
                self.execute_send_transaction().await?;
            }
            KeyCode::Esc => {
                // Cancel
                self.show_send_dialog = false;
                self.send_amount.clear();
                self.send_address.clear();
                self.set_status("Transaction cancelled".to_string());
            }
            _ => {}
        }
        Ok(())
    }

    async fn execute_send_transaction(&mut self) -> Result<()> {
        // Validate inputs
        if self.send_amount.is_empty() {
            self.set_status("✗ Amount is required".to_string());
            return Ok(());
        }

        // Get destination address from either manual entry or wallet selector
        let destination_address = if self.send_use_wallet_selector {
            if self.send_selected_wallet_index >= self.wallets.len() {
                self.set_status("✗ Invalid wallet selection".to_string());
                return Ok(());
            }
            let selected_wallet = &self.wallets[self.send_selected_wallet_index];
            match &selected_wallet.address {
                Some(addr) => addr.clone(),
                None => {
                    self.set_status("✗ Selected wallet has no address".to_string());
                    return Ok(());
                }
            }
        } else {
            if self.send_address.is_empty() {
                self.set_status("✗ Destination address is required".to_string());
                return Ok(());
            }
            self.send_address.clone()
        };

        let amount: f64 = match self.send_amount.parse() {
            Ok(a) => a,
            Err(_) => {
                self.set_status("✗ Invalid amount".to_string());
                return Ok(());
            }
        };

        if amount <= 0.0 {
            self.set_status("✗ Amount must be greater than 0".to_string());
            return Ok(());
        }

        if self.selected_index >= self.wallets.len() {
            return Ok(());
        }

        let wallet = &self.wallets[self.selected_index];
        let worker_id = wallet.worker_id;

        // Check if wallet has sufficient balance
        if let Some(balance) = wallet.balance {
            if amount > balance {
                self.set_status(format!("✗ Insufficient balance. Available: {:.8} KAS", balance));
                return Ok(());
            }
        }

        self.set_status(format!("Sending {:.8} KAS to {}...", amount, destination_address));

        // Get password from config
        let password = self.config.get(&format!("W{}_KASWALLET_PASSWORD", worker_id))
            .unwrap_or("password");

        // Send transaction
        match self.wallet_manager.send_transaction(worker_id, &destination_address, amount, password).await {
            Ok(tx_id) => {
                self.set_status(format!("✓ Transaction sent! ID: {}", tx_id));
                self.show_send_dialog = false;
                self.send_amount.clear();
                self.send_address.clear();

                // Refresh wallet data after a delay
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                self.refresh_data().await?;
            }
            Err(e) => {
                self.set_status(format!("✗ Failed to send transaction: {}", e));
            }
        }

        Ok(())
    }

    async fn handle_service_start(&mut self) -> Result<()> {
        if self.selected_index >= self.containers.len() {
            return Ok(());
        }

        let service = self.containers[self.selected_index].name.clone();
        self.set_status(format!("Starting {}...", service));

        match self.docker.start_service(&service).await {
            Ok(_) => {
                self.set_status(format!("✓ Started {}", service));
                self.refresh_data().await?;
            }
            Err(e) => {
                self.set_status(format!("✗ Failed to start {}: {}", service, e));
            }
        }

        Ok(())
    }

    async fn handle_service_stop(&mut self) -> Result<()> {
        if self.selected_index >= self.containers.len() {
            return Ok(());
        }

        let service = self.containers[self.selected_index].name.clone();
        self.set_status(format!("Stopping {}...", service));

        match self.docker.stop_service(&service).await {
            Ok(_) => {
                self.set_status(format!("✓ Stopped {}", service));
                self.refresh_data().await?;
            }
            Err(e) => {
                self.set_status(format!("✗ Failed to stop {}: {}", service, e));
            }
        }

        Ok(())
    }

    async fn handle_service_restart(&mut self) -> Result<()> {
        if self.selected_index >= self.containers.len() {
            return Ok(());
        }

        let service = self.containers[self.selected_index].name.clone();
        self.set_status(format!("Restarting {}...", service));

        match self.docker.restart_service(&service).await {
            Ok(_) => {
                self.set_status(format!("✓ Restarted {}", service));
                self.refresh_data().await?;
            }
            Err(e) => {
                self.set_status(format!("✗ Failed to restart {}: {}", service, e));
            }
        }

        Ok(())
    }

    fn get_profile_name(&self, index: usize) -> Option<String> {
        match index {
            0 => Some("kaspad".to_string()),
            1 => Some("backend".to_string()),
            2 => Some("frontend-w1".to_string()),
            3 => Some("frontend-w2".to_string()),
            4 => Some("frontend-w3".to_string()),
            5 => Some("frontend-w4".to_string()),
            6 => Some("frontend-w5".to_string()),
            _ => None,
        }
    }

    async fn handle_profile_toggle(&mut self) -> Result<()> {
        let profile = match self.get_profile_name(self.selected_index) {
            Some(p) => p,
            None => return Ok(()),
        };

        // Check if profile is active
        let is_active = self.active_profiles.contains(&profile);

        if is_active {
            self.handle_profile_stop().await
        } else {
            self.handle_profile_start().await
        }
    }

    async fn handle_profile_start(&mut self) -> Result<()> {
        let profile = match self.get_profile_name(self.selected_index) {
            Some(p) => p,
            None => return Ok(()),
        };

        self.set_status(format!("Starting profile {}...", profile));

        match self.docker.start_profile(&profile).await {
            Ok(_) => {
                self.set_status(format!("✓ Started profile {}", profile));
                self.refresh_data().await?;
            }
            Err(e) => {
                self.set_status(format!("✗ Failed to start profile {}: {}", profile, e));
            }
        }

        Ok(())
    }

    async fn handle_profile_stop(&mut self) -> Result<()> {
        let profile = match self.get_profile_name(self.selected_index) {
            Some(p) => p,
            None => return Ok(()),
        };

        self.set_status(format!("Stopping profile {}...", profile));

        match self.docker.stop_profile(&profile).await {
            Ok(_) => {
                self.set_status(format!("✓ Stopped profile {}", profile));
                self.refresh_data().await?;
            }
            Err(e) => {
                self.set_status(format!("✗ Failed to stop profile {}: {}", profile, e));
            }
        }

        Ok(())
    }

    async fn handle_rpc_action(&mut self) -> Result<()> {
        use crate::core::rpc::RpcTester;

        let tokens = self.config.get_rpc_tokens();
        if self.selected_index >= tokens.len() {
            return Ok(());
        }

        let (token_num, token_opt) = &tokens[self.selected_index];
        if let Some(token) = token_opt {
            self.set_status(format!("Testing RPC token {}...", token_num));

            if let Some(domain) = self.config.get("IGRA_ORCHESTRA_DOMAIN") {
                let tester = RpcTester::new();
                match tester.test_both_endpoints(domain, token).await {
                    Ok((http, https)) => {
                        let msg = if http.success && https.success {
                            format!("✓ Token {} OK - HTTP: {}ms, HTTPS: {}ms",
                                token_num, http.response_time_ms, https.response_time_ms)
                        } else {
                            format!("✗ Token {} failed", token_num)
                        };
                        self.set_status(msg);
                    }
                    Err(e) => {
                        self.set_status(format!("✗ Test failed: {}", e));
                    }
                }
            } else {
                self.set_status("✗ IGRA_ORCHESTRA_DOMAIN not configured".to_string());
            }
        } else {
            self.set_status(format!("Token {} not set", token_num));
        }

        Ok(())
    }

    async fn handle_generate_tokens(&mut self) -> Result<()> {
        self.set_status("Generating all RPC tokens...".to_string());

        match self.config.generate_all_rpc_tokens() {
            Ok(tokens) => {
                match self.config.save() {
                    Ok(_) => {
                        self.set_status(format!("✓ Generated {} tokens and saved to .env", tokens.len()));
                        // Reload config
                        self.config = ConfigManager::load_from_project()?;
                        self.refresh_data().await?;
                    }
                    Err(e) => {
                        self.set_status(format!("✗ Failed to save: {}", e));
                    }
                }
            }
            Err(e) => {
                self.set_status(format!("✗ Failed to generate tokens: {}", e));
            }
        }

        Ok(())
    }

    async fn handle_upgrade(&mut self) -> Result<()> {
        self.set_status("Pulling latest Docker images...".to_string());

        match self.docker.pull_images().await {
            Ok(_) => {
                self.set_status("✓ Images updated. Restart services to apply changes.".to_string());
            }
            Err(e) => {
                self.set_status(format!("✗ Failed to pull images: {}", e));
            }
        }

        Ok(())
    }

    async fn handle_ssl_check(&mut self) -> Result<()> {
        if self.ssl_domain == "N/A" {
            self.set_status("✗ No domain configured. Set IGRA_ORCHESTRA_DOMAIN in config.".to_string());
            return Ok(());
        }

        self.set_status(format!("Checking certificate for {}...", self.ssl_domain));

        match self.ssl_manager.get_certificate_info(&self.ssl_domain).await {
            Ok(cert_info) => {
                let status = if cert_info.is_valid {
                    if let Some(days) = cert_info.days_remaining {
                        format!("✓ Certificate valid - {} days remaining", days)
                    } else {
                        "✓ Certificate valid".to_string()
                    }
                } else {
                    "✗ Certificate invalid or expired".to_string()
                };
                self.set_status(status);
                self.ssl_cert_info = Some(cert_info.clone());
                self.dashboard.update_ssl(Some(cert_info));
            }
            Err(e) => {
                self.set_status(format!("✗ Failed to check certificate: {}", e));
                self.ssl_cert_info = None;
                self.dashboard.update_ssl(None);
            }
        }

        Ok(())
    }

    async fn handle_ssl_renew(&mut self) -> Result<()> {
        self.set_status("Forcing certificate renewal (restarting Traefik)...".to_string());

        match self.ssl_manager.force_renewal().await {
            Ok(_) => {
                self.set_status("✓ Traefik restarted. Certificate will renew if needed.".to_string());
                // Wait a moment then refresh
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                self.refresh_data().await?;
            }
            Err(e) => {
                self.set_status(format!("✗ Failed to restart Traefik: {}", e));
            }
        }

        Ok(())
    }

    async fn handle_detail_view_key(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Enter => {
                // In wallet detail view, Enter toggles transaction detail modal
                if self.detail_view_wallet.is_some() && !self.detail_wallet_utxos.is_empty() {
                    if self.show_tx_detail {
                        // Close transaction detail modal
                        self.show_tx_detail = false;
                    } else {
                        // Open transaction detail modal for selected transaction
                        // Initialize selection to 0 if not set
                        if self.selected_tx_index.is_none() {
                            self.selected_tx_index = Some(0);
                        }
                        self.show_tx_detail = true;
                    }
                }
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                // Close modal if showing, otherwise exit detail view
                if self.show_tx_detail {
                    self.show_tx_detail = false;
                } else {
                    // Exit detail view
                    self.detail_view_service = None;
                    self.detail_logs.clear();
                    self.detail_view_wallet = None;
                    self.detail_wallet_addresses.clear();
                    self.detail_wallet_utxos.clear();
                    self.detail_wallet_scroll = 0;
                    self.detail_addresses_scroll = 0;
                    self.selected_tx_index = None;
                }
            }
            KeyCode::Char('s') => {
                // Start service from detail view
                if let Some(service) = &self.detail_view_service {
                    let service = service.clone();
                    self.set_status(format!("Starting {}...", service));
                    match self.docker.start_service(&service).await {
                        Ok(_) => {
                            self.set_status(format!("✓ Started {}", service));
                            self.refresh_data().await?;
                        }
                        Err(e) => {
                            self.set_status(format!("✗ Failed to start {}: {}", service, e));
                        }
                    }
                }
            }
            KeyCode::Char('x') => {
                // Stop service from detail view
                if let Some(service) = &self.detail_view_service {
                    let service = service.clone();
                    self.set_status(format!("Stopping {}...", service));
                    match self.docker.stop_service(&service).await {
                        Ok(_) => {
                            self.set_status(format!("✓ Stopped {}", service));
                            self.refresh_data().await?;
                        }
                        Err(e) => {
                            self.set_status(format!("✗ Failed to stop {}: {}", service, e));
                        }
                    }
                }
            }
            KeyCode::Char('R') => {
                // Restart service from detail view
                if let Some(service) = &self.detail_view_service {
                    let service = service.clone();
                    self.set_status(format!("Restarting {}...", service));
                    match self.docker.restart_service(&service).await {
                        Ok(_) => {
                            self.set_status(format!("✓ Restarted {}", service));
                            self.refresh_data().await?;
                        }
                        Err(e) => {
                            self.set_status(format!("✗ Failed to restart {}: {}", service, e));
                        }
                    }
                }
            }
            KeyCode::Char('r') => {
                // Refresh logs
                if let Some(service) = &self.detail_view_service {
                    let service = service.clone();
                    match self.docker.get_logs(&service, Some(50)).await {
                        Ok(logs) => {
                            self.detail_logs = logs.lines().map(|s| s.to_string()).collect();
                            self.set_status("✓ Refreshed logs".to_string());
                        }
                        Err(e) => {
                            self.set_status(format!("✗ Failed to refresh logs: {}", e));
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn enter_edit_mode(&mut self) {
        if self.selected_index >= self.config_data.len() {
            return;
        }

        let (key, value) = &self.config_data[self.selected_index];

        // Don't allow editing of sensitive fields
        if key.contains("PASSWORD") || key.contains("SECRET") || key.contains("KEY") || key.contains("TOKEN") {
            self.set_status("Cannot edit sensitive fields directly. Edit .env file manually.".to_string());
            return;
        }

        self.edit_mode = true;
        self.edit_key = Some(key.clone());
        self.edit_buffer = value.clone();
        self.set_status(format!("Editing {} - [Enter] Save | [Esc] Cancel", key));
    }

    async fn handle_edit_key(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Char(c) => {
                self.edit_buffer.push(c);
            }
            KeyCode::Backspace => {
                self.edit_buffer.pop();
            }
            KeyCode::Enter => {
                // Save the edit
                self.save_config_edit().await?;
            }
            KeyCode::Esc => {
                // Cancel edit
                self.edit_mode = false;
                self.edit_buffer.clear();
                self.edit_key = None;
                self.set_status("Edit cancelled".to_string());
            }
            _ => {}
        }
        Ok(())
    }

    async fn save_config_edit(&mut self) -> Result<()> {
        let key = match &self.edit_key {
            Some(k) => k.clone(),
            None => {
                self.edit_mode = false;
                return Ok(());
            }
        };

        let value = self.edit_buffer.clone();

        // Validate based on key type
        let validation_error = self.validate_config_value(&key, &value);
        if let Some(error) = validation_error {
            self.set_status(format!("✗ Validation failed: {}", error));
            return Ok(());
        }

        // Set the value in config manager
        self.config.set(&key, &value);

        // Save to file
        match self.config.save() {
            Ok(_) => {
                self.set_status(format!("✓ Saved {} = {}", key, value));
                self.edit_mode = false;
                self.edit_buffer.clear();
                self.edit_key = None;

                // Reload config
                self.config = ConfigManager::load_from_project()?;
                self.refresh_data().await?;
            }
            Err(e) => {
                self.set_status(format!("✗ Failed to save: {}", e));
            }
        }

        Ok(())
    }

    fn validate_config_value(&self, key: &str, value: &str) -> Option<String> {
        use crate::utils::{is_valid_domain, is_valid_email, is_valid_hex};

        // Empty values are generally not allowed
        if value.trim().is_empty() {
            return Some("Value cannot be empty".to_string());
        }

        // Domain validation
        if key.contains("DOMAIN") && !is_valid_domain(value) {
            return Some("Invalid domain format".to_string());
        }

        // Email validation
        if key.contains("EMAIL") && !is_valid_email(value) {
            return Some("Invalid email format".to_string());
        }

        // Hex validation for keys/secrets
        if (key.contains("_KEY") || key.contains("_SECRET")) && key != "OVH_APPLICATION_KEY" && key != "OVH_APPLICATION_SECRET" {
            if !is_valid_hex(value) {
                return Some("Must be a valid hex string".to_string());
            }
        }

        // Port validation
        if key.contains("PORT") {
            if value.parse::<u16>().is_err() {
                return Some("Must be a valid port number (1-65535)".to_string());
            }
        }

        // URL validation (basic)
        if key.contains("URL") || key.contains("ENDPOINT") {
            if !value.starts_with("http://") && !value.starts_with("https://") {
                return Some("Must start with http:// or https://".to_string());
            }
        }

        // Network validation
        if key == "NETWORK" {
            if value != "testnet" && value != "mainnet" {
                return Some("Must be either 'testnet' or 'mainnet'".to_string());
            }
        }

        None
    }

    fn next_screen(&mut self) {
        let screens = Screen::all();
        let current_idx = screens.iter().position(|s| *s == self.current_screen).unwrap_or(0);
        let next_idx = (current_idx + 1) % screens.len();
        self.current_screen = screens[next_idx];
    }

    fn prev_screen(&mut self) {
        let screens = Screen::all();
        let current_idx = screens.iter().position(|s| *s == self.current_screen).unwrap_or(0);
        let prev_idx = if current_idx == 0 {
            screens.len() - 1
        } else {
            current_idx - 1
        };
        self.current_screen = screens[prev_idx];
    }

    async fn handle_search_key(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Char(c) => {
                self.search_buffer.push(c);
                // Apply filter in real-time
                self.apply_search_filter();
                self.set_status(format!("Search: {} (Enter to apply, Esc to cancel)", self.search_buffer));
            }
            KeyCode::Backspace => {
                self.search_buffer.pop();
                self.apply_search_filter();
                self.set_status(format!("Search: {} (Enter to apply, Esc to cancel)", self.search_buffer));
            }
            KeyCode::Enter => {
                // Apply search and exit search mode
                self.apply_search_filter();
                let count = self.filtered_indices.len();
                self.search_mode = false;

                if self.search_buffer.is_empty() {
                    self.set_status("Search cleared".to_string());
                } else {
                    self.set_status(format!("Found {} matches for '{}'", count, self.search_buffer));
                    // Jump to first match if any
                    if !self.filtered_indices.is_empty() {
                        self.selected_index = self.filtered_indices[0];
                    }
                }
            }
            KeyCode::Esc => {
                // Cancel search
                self.search_mode = false;
                self.search_buffer.clear();
                self.filtered_indices.clear();
                self.set_status("Search cancelled".to_string());
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_tx_search_key(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Char(c) => {
                self.tx_search_buffer.push(c);
                // Apply filter in real-time
                self.apply_tx_search_filter();
                self.set_status(format!("Search: {} (Enter to apply, Esc to cancel)", self.tx_search_buffer));
            }
            KeyCode::Backspace => {
                self.tx_search_buffer.pop();
                self.apply_tx_search_filter();
                self.set_status(format!("Search: {} (Enter to apply, Esc to cancel)", self.tx_search_buffer));
            }
            KeyCode::Enter => {
                // Apply search and exit search mode
                self.apply_tx_search_filter();
                let count = self.filtered_tx_indices.len();
                self.tx_search_mode = false;

                if self.tx_search_buffer.is_empty() {
                    self.set_status("Search cleared".to_string());
                } else {
                    self.set_status(format!("Found {} matching transactions for '{}'", count, self.tx_search_buffer));
                    // Jump to first match if any
                    if !self.filtered_tx_indices.is_empty() {
                        self.detail_wallet_scroll = 0; // Reset scroll to show first match
                    }
                }
            }
            KeyCode::Esc => {
                // Cancel search
                self.tx_search_mode = false;
                self.tx_search_buffer.clear();
                self.filtered_tx_indices.clear();
                self.set_status("Search cancelled".to_string());
            }
            _ => {}
        }
        Ok(())
    }

    fn apply_tx_search_filter(&mut self) {
        self.filtered_tx_indices.clear();

        if self.tx_search_buffer.is_empty() {
            return;
        }

        let query = self.tx_search_buffer.to_lowercase();

        // Filter transactions by TxID, address, or amount
        for (idx, utxo) in self.detail_wallet_utxos.iter().enumerate() {
            let tx_id_match = utxo.tx_id.to_lowercase().contains(&query);
            let address_match = utxo.address.to_lowercase().contains(&query);
            let amount_str = format!("{:.8}", utxo.amount_kas);
            let amount_match = amount_str.contains(&query);

            if tx_id_match || address_match || amount_match {
                self.filtered_tx_indices.push(idx);
            }
        }
    }

    fn apply_search_filter(&mut self) {
        self.filtered_indices.clear();

        if self.search_buffer.is_empty() {
            return;
        }

        let query = self.search_buffer.to_lowercase();

        match self.current_screen {
            Screen::Services => {
                // Filter services by name or status
                for (idx, container) in self.containers.iter().enumerate() {
                    if container.name.to_lowercase().contains(&query)
                        || container.status.to_lowercase().contains(&query)
                        || container.image.to_lowercase().contains(&query) {
                        self.filtered_indices.push(idx);
                    }
                }
            }
            Screen::Config => {
                // Filter config by key or value
                for (idx, (key, value)) in self.config_data.iter().enumerate() {
                    if key.to_lowercase().contains(&query)
                        || value.to_lowercase().contains(&query) {
                        self.filtered_indices.push(idx);
                    }
                }
            }
            Screen::Wallets => {
                // Filter wallets by worker ID or address
                for (idx, wallet) in self.wallets.iter().enumerate() {
                    let worker_str = format!("worker {}", wallet.worker_id).to_lowercase();
                    let addr_match = wallet.address.as_ref()
                        .map(|a| a.to_lowercase().contains(&query))
                        .unwrap_or(false);

                    if worker_str.contains(&query) || addr_match {
                        self.filtered_indices.push(idx);
                    }
                }
            }
            _ => {}
        }
    }

    fn render(&self, frame: &mut ratatui::Frame) {
        // Get container info for detail view
        let detail_container = self.detail_view_service.as_ref().and_then(|service_name| {
            self.containers.iter().find(|c| &c.name == service_name)
        });

        // Get wallet info for detail view
        let detail_wallet = self.detail_view_wallet.and_then(|worker_id| {
            self.wallets.iter().find(|w| w.worker_id == worker_id)
        });

        self.dashboard.render(
            frame,
            self.current_screen,
            self.services_view,
            self.config_section,
            self.selected_index,
            self.status_message.as_deref(),
            self.edit_mode,
            self.edit_buffer.as_str(),
            detail_container,
            &self.detail_logs,
            &self.system_resources,
            self.show_help,
            self.logs_selected_service.as_deref(),
            &self.logs_data,
            self.logs_follow_mode,
            self.logs_compact_mode,
            self.logs_live_mode,
            self.logs_grouping_enabled,
            self.logs_filter.as_deref(),
            self.logs_scroll_offset,
            &self.containers,
            self.search_mode,
            &self.search_buffer,
            &self.filtered_indices,
            self.show_send_dialog,
            &self.send_amount,
            &self.send_address,
            self.send_input_field,
            self.send_use_wallet_selector,
            self.send_selected_wallet_index,
            &self.send_source_address,
            &self.wallets,
            self.reth_metrics.as_ref(),
            detail_wallet,
            &self.detail_wallet_addresses,
            &self.detail_wallet_utxos,
            self.detail_wallet_scroll,
            self.show_tx_detail,
            self.selected_tx_index,
            self.tx_search_mode,
            &self.tx_search_buffer,
            &self.filtered_tx_indices,
            self.watch_statistics.as_ref(),
            &self.watch_transactions,
            &self.watch_filter,
            self.watch_scroll_offset,
        );
    }

    /// Start background polling task for logs live mode
    fn start_logs_live_mode(&mut self) {
        // Stop any existing task first
        self.stop_logs_live_mode();

        if let Some(service_name) = self.logs_selected_service.clone() {
            let docker = self.docker.clone();
            let logs_tx = self.logs_live_tx.clone();

            let handle = tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(1));

                loop {
                    interval.tick().await;

                    // Fetch logs
                    if let Ok(logs) = docker.get_logs(&service_name, Some(500)).await {
                        let log_lines: Vec<String> = logs.lines().map(|s| s.to_string()).collect();
                        // Send to main thread (non-blocking send)
                        if logs_tx.send(log_lines).is_err() {
                            // Channel closed, stop task
                            break;
                        }
                    }
                }
            });

            self.logs_live_task_handle = Some(handle);
        }
    }

    /// Stop background polling task for logs live mode
    fn stop_logs_live_mode(&mut self) {
        if let Some(handle) = self.logs_live_task_handle.take() {
            handle.abort();
        }
    }

    /// Write a transaction to file in the specified format
    fn write_transaction_to_file(
        file: &mut std::fs::File,
        tx: &crate::core::l2_monitor::TransactionInfo,
        format: &str,
    ) -> anyhow::Result<()> {
        use std::io::Write;

        match format {
            "json" => {
                let json = serde_json::to_string(tx)?;
                writeln!(file, "{}", json)?;
            }
            "csv" => {
                // CSV format
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
}
