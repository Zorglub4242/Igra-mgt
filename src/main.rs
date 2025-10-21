mod app;
mod cli;
mod core;
mod screens;
mod utils;
mod widgets;

use anyhow::Result;
use clap::Parser;

use app::App;
use cli::{BackupCommands, Cli, Commands, ConfigCommands, RpcCommands, TokenCommands, WalletCommands};
use core::{ConfigManager, DockerManager};
use core::rpc::RpcTester;
use core::wallet::WalletManager;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        None => {
            // No command - run interactive TUI
            let mut app = App::new()?;
            app.run().await?;
        }
        Some(Commands::Status) => {
            handle_status().await?;
        }
        Some(Commands::Start { profile, service }) => {
            handle_start(profile, service).await?;
        }
        Some(Commands::Stop { all, service }) => {
            handle_stop(all, service).await?;
        }
        Some(Commands::Restart { service }) => {
            handle_restart(service).await?;
        }
        Some(Commands::Logs {
            service,
            follow,
            tail,
        }) => {
            handle_logs(service, follow, tail).await?;
        }
        Some(Commands::Rpc { command }) => {
            handle_rpc(command).await?;
        }
        Some(Commands::Wallet { command }) => {
            handle_wallet(command).await?;
        }
        Some(Commands::Backup { command }) => {
            handle_backup(command).await?;
        }
        Some(Commands::Config { command }) => {
            handle_config(command).await?;
        }
        Some(Commands::Monitor) => {
            println!("Monitor mode is available in the TUI dashboard.");
            println!("Run 'igra-cli' without arguments to launch the interactive dashboard.");
        }
        Some(Commands::Health) => {
            println!("Health checks are available in the TUI dashboard.");
            println!("Run 'igra-cli' without arguments to view service health status.");
        }
        Some(Commands::Upgrade { check, pull, apply }) => {
            handle_upgrade(check, pull, apply).await?;
        }
        Some(Commands::Diag { report }) => {
            handle_diagnostics(report).await?;
        }
        Some(Commands::Setup) => {
            println!("Setup wizard - Use the TUI dashboard to configure your installation.");
            println!("Run 'igra-cli' without arguments to launch the interactive dashboard.");
            println!("\nFor initial setup, ensure:");
            println!("  1. Docker and Docker Compose are installed");
            println!("  2. .env file is configured (see .env.example)");
            println!("  3. Run: docker compose --profile <profile> up -d");
        }
        Some(Commands::Watch { filter, record, format }) => {
            handle_watch(filter, record, format).await?;
        }
    }

    Ok(())
}

async fn handle_status() -> Result<()> {
    let docker = DockerManager::new().await?;
    let containers = docker.list_containers().await?;

    println!("IGRA Orchestra Status\n");
    println!("{:<25} {:<15} {:<15}", "Service", "Status", "Health");
    println!("{}", "-".repeat(60));

    for container in containers {
        let health = container.health.as_deref().unwrap_or("N/A");
        println!(
            "{:<25} {:<15} {:<15}",
            container.name, container.status, health
        );
    }

    Ok(())
}

async fn handle_start(profile: Option<String>, service: Option<String>) -> Result<()> {
    let docker = DockerManager::new().await?;

    if let Some(profile) = profile {
        println!("Starting profile: {}", profile);
        docker.start_profile(&profile).await?;
        println!("Profile {} started", profile);
    } else if let Some(service) = service {
        println!("Starting service: {}", service);
        docker.start_service(&service).await?;
        println!("Service {} started", service);
    } else {
        println!("Error: Specify either --profile or service name");
    }

    Ok(())
}

async fn handle_stop(all: bool, service: Option<String>) -> Result<()> {
    let docker = DockerManager::new().await?;

    if all {
        println!("Stopping all services...");
        docker.stop_all().await?;
        println!("All services stopped");
    } else if let Some(service) = service {
        println!("Stopping service: {}", service);
        docker.stop_service(&service).await?;
        println!("Service {} stopped", service);
    } else {
        println!("Error: Specify either --all or service name");
    }

    Ok(())
}

async fn handle_restart(service: String) -> Result<()> {
    let docker = DockerManager::new().await?;
    println!("Restarting service: {}", service);
    docker.restart_service(&service).await?;
    println!("Service {} restarted", service);

    Ok(())
}

async fn handle_logs(service: String, follow: bool, tail: usize) -> Result<()> {
    let docker = DockerManager::new().await?;

    if follow {
        println!("Following logs for {}... (Ctrl+C to stop)", service);
        println!("Note: For better log viewing with filtering and search, use the TUI dashboard (Screen 7 - Logs)");
        println!();

        // Basic implementation - show initial logs
        // For real-time following, use: docker compose logs -f <service>
        let logs = docker.get_logs(&service, Some(tail)).await?;
        print!("{}", logs);

        println!("\nTip: Use 'docker compose logs -f {}' for continuous log streaming", service);
    } else {
        let logs = docker.get_logs(&service, Some(tail)).await?;
        print!("{}", logs);
    }

    Ok(())
}

async fn handle_rpc(command: RpcCommands) -> Result<()> {
    match command {
        RpcCommands::Tokens { command } => {
            match command {
                Some(TokenCommands::List) | None => {
                    let config = ConfigManager::load(".env")?;
                    println!("RPC Access Tokens:\n");
                    for (i, token) in config.get_rpc_tokens() {
                        if let Some(t) = token {
                            println!("TOKEN_{:02}: {}...{}", i, &t[..8], &t[t.len() - 8..]);
                        } else {
                            println!("TOKEN_{:02}: <not set>", i);
                        }
                    }
                }
                Some(TokenCommands::Generate) => {
                    let mut config = ConfigManager::load(".env")?;
                    println!("Generating all RPC access tokens...\n");

                    let tokens = config.generate_all_rpc_tokens()?;
                    config.save()?;

                    println!("✓ Generated {} tokens", tokens.len());
                    println!("\nTokens have been saved to .env file");
                    println!("You can view them with: igra-cli rpc tokens list");
                }
                Some(TokenCommands::Test { token_number }) => {
                    let config = ConfigManager::load(".env")?;
                    let domain = config.get("IGRA_ORCHESTRA_DOMAIN")
                        .ok_or_else(|| anyhow::anyhow!("IGRA_ORCHESTRA_DOMAIN not set in .env"))?;

                    let tokens = config.get_rpc_tokens();
                    let (_index, token_opt) = tokens.iter()
                        .find(|(i, _)| *i == token_number)
                        .ok_or_else(|| anyhow::anyhow!("Invalid token number"))?;

                    let token = token_opt.as_ref()
                        .ok_or_else(|| anyhow::anyhow!("Token {} is not set", token_number))?;

                    println!("Testing RPC token {}...\n", token_number);

                    let tester = RpcTester::new();
                    let (http_result, https_result) = tester.test_both_endpoints(domain, token).await?;

                    println!("HTTP Test:");
                    if http_result.success {
                        println!("  ✓ Success ({}ms)", http_result.response_time_ms);
                        if let Some(bn) = http_result.block_number {
                            println!("  Block Number: {}", bn);
                        }
                    } else {
                        println!("  ✗ Failed: {}", http_result.error.unwrap_or_default());
                    }

                    println!("\nHTTPS Test:");
                    if https_result.success {
                        println!("  ✓ Success ({}ms)", https_result.response_time_ms);
                        if let Some(bn) = https_result.block_number {
                            println!("  Block Number: {}", bn);
                        }
                    } else {
                        println!("  ✗ Failed: {}", https_result.error.unwrap_or_default());
                    }
                }
            }
        }
        RpcCommands::TestEndpoint { token } => {
            let config = ConfigManager::load(".env")?;
            let domain = config.get("IGRA_ORCHESTRA_DOMAIN")
                .ok_or_else(|| anyhow::anyhow!("IGRA_ORCHESTRA_DOMAIN not set in .env"))?
                .to_string();

            let test_token: String = if let Some(token_num) = token {
                let tokens = config.get_rpc_tokens();
                tokens.iter()
                    .find(|(i, _)| *i == token_num)
                    .and_then(|(_, t)| t.clone())
                    .ok_or_else(|| anyhow::anyhow!("Token {} not found", token_num))?
            } else {
                // Use first available token
                let tokens = config.get_rpc_tokens();
                tokens.iter()
                    .find_map(|(_, t)| t.clone())
                    .ok_or_else(|| anyhow::anyhow!("No RPC tokens configured"))?
            };

            println!("Testing RPC endpoints...\n");

            let tester = RpcTester::new();
            let (http_result, https_result) = tester.test_both_endpoints(&domain, &test_token).await?;

            println!("HTTP Endpoint (http://{}:8545):", domain);
            if http_result.success {
                println!("  ✓ Success ({}ms)", http_result.response_time_ms);
                if let Some(bn) = http_result.block_number {
                    println!("  Block Number: {}", bn);
                }
            } else {
                println!("  ✗ Failed: {}", http_result.error.unwrap_or_default());
            }

            println!("\nHTTPS Endpoint (https://{}:8545):", domain);
            if https_result.success {
                println!("  ✓ Success ({}ms)", https_result.response_time_ms);
                if let Some(bn) = https_result.block_number {
                    println!("  Block Number: {}", bn);
                }
            } else {
                println!("  ✗ Failed: {}", https_result.error.unwrap_or_default());
            }
        }
    }

    Ok(())
}

async fn handle_wallet(command: WalletCommands) -> Result<()> {
    let wallet_manager = WalletManager::new()?;

    match command {
        WalletCommands::List => {
            println!("IGRA Wallet Status\n");
            let wallets = wallet_manager.list_wallets().await?;

            println!("{:<10} {:<12} {:<50} {:<15}", "Worker", "Status", "Address", "Balance");
            println!("{}", "-".repeat(90));

            for wallet in wallets {
                let status = if wallet.container_running {
                    "Running"
                } else {
                    "Stopped"
                };

                let address = wallet.address.as_deref().unwrap_or("N/A");
                let balance = wallet
                    .balance
                    .map(|b| format!("{:.8} KAS", b))
                    .unwrap_or_else(|| "N/A".to_string());

                println!(
                    "{:<10} {:<12} {:<50} {:<15}",
                    format!("Worker {}", wallet.worker_id),
                    status,
                    address,
                    balance
                );
            }
        }
        WalletCommands::Balance { worker_id } => {
            println!("Fetching balance for wallet {}...\n", worker_id);

            match wallet_manager.get_balance(worker_id).await {
                Ok(balance) => {
                    println!("Balance: {:.8} KAS", balance);

                    if let Ok(address) = wallet_manager.get_address(worker_id).await {
                        println!("Address: {}", address);
                    }
                }
                Err(e) => {
                    println!("✗ Failed to get balance: {}", e);
                    println!("\nMake sure kaswallet-{} container is running.", worker_id);
                }
            }
        }
        WalletCommands::Generate { worker_id } => {
            println!("Generating new wallet for worker {}...\n", worker_id);
            println!("⚠ Warning: This will create a new wallet. Make sure to backup existing wallet if any.");
            println!();

            // For now, just show instructions since wallet generation requires interactive input
            println!("To generate a wallet manually, run:");
            println!(
                "  docker exec -it kaswallet-{} kaswallet-create --testnet --create",
                worker_id
            );
            println!();
            println!("The wallet files will be stored in the container's data volume.");
            println!("Make sure to backup the wallet seed phrase!");
        }
    }

    Ok(())
}

async fn handle_backup(command: BackupCommands) -> Result<()> {
    println!("Backup functionality - Not yet implemented");
    println!("\nManual backup procedures:");
    println!("  1. Stop services: docker compose down");
    println!("  2. Backup volumes: docker run --rm -v igra-data:/data -v $(pwd):/backup alpine tar czf /backup/backup.tar.gz /data");
    println!("  3. Backup .env and keys: tar czf config-backup.tar.gz .env keys/");
    println!();

    match command {
        BackupCommands::Create { service } => {
            println!("To backup {}:", service);
            println!("  docker compose stop {}", service);
            println!("  # Copy relevant volumes and data");
        }
        BackupCommands::List => {
            println!("List your backup files in the backup directory");
        }
        BackupCommands::Restore { service, file } => {
            println!("To restore {} from {}:", service, file);
            println!("  docker compose stop {}", service);
            println!("  # Extract and restore from {}", file);
        }
    }

    Ok(())
}

async fn handle_config(command: ConfigCommands) -> Result<()> {
    match command {
        ConfigCommands::View => {
            let config = ConfigManager::load(".env")?;
            println!("Configuration:\n");
            for key in config.keys() {
                if let Some(value) = config.get(&key) {
                    // Mask sensitive values
                    let display_value = if key.contains("PASSWORD")
                        || key.contains("SECRET")
                        || key.contains("KEY")
                    {
                        "****"
                    } else {
                        value
                    };
                    println!("{}: {}", key, display_value);
                }
            }
        }
        ConfigCommands::Edit => {
            println!("Configuration editing is available in the TUI dashboard.");
            println!("Run 'igra-cli' and navigate to Screen 5 (Config) to edit values.");
            println!("\nAlternatively, edit .env file directly with your preferred editor:");
            println!("  nano .env");
            println!("  vim .env");
        }
        ConfigCommands::Validate => {
            let config = ConfigManager::load(".env")?;
            let errors = config.validate();

            if errors.is_empty() {
                println!("✓ Configuration is valid");
            } else {
                println!("✗ Configuration errors:");
                for error in errors {
                    println!("  - {}", error);
                }
            }
        }
        ConfigCommands::GenerateTokens => {
            println!("RPC token generation is available in the TUI dashboard.");
            println!("Run 'igra-cli' and navigate to Screen 4 (RPC Tokens), then press 'g'.");
        }
    }

    Ok(())
}

async fn handle_upgrade(check: bool, pull: bool, apply: bool) -> Result<()> {
    if check {
        println!("Checking for updates...");
        println!("To check for image updates: docker compose pull");
        println!("\nOr use the TUI dashboard (press 'u' for upgrade)");
    } else if pull {
        let docker = DockerManager::new().await?;
        println!("Pulling latest images...");
        docker.pull_images().await?;
        println!("✓ Images updated");
        println!("\nRestart services to use new images:");
        println!("  docker compose down && docker compose --profile <profile> up -d");
    } else if apply {
        println!("Applying upgrades...");
        println!("This will pull images and restart services.");
        println!("\nUse:");
        println!("  igra-cli upgrade --pull");
        println!("  docker compose down");
        println!("  docker compose --profile <profile> up -d");
    } else {
        println!("Specify --check, --pull, or --apply");
        println!("\nOr use the TUI dashboard (press 'u' for upgrade)");
    }

    Ok(())
}

async fn handle_diagnostics(report: bool) -> Result<()> {
    if report {
        println!("Generating diagnostic report...\n");

        let docker = DockerManager::new().await?;
        let containers = docker.list_containers().await?;

        println!("=== IGRA Orchestra Diagnostics ===\n");
        println!("Services:");
        for container in &containers {
            println!("  {} - {:?} ({})", container.name, container.state, container.status);
        }

        println!("\nFor detailed monitoring, use the TUI dashboard:");
        println!("  igra-cli");
        println!("\nFor logs:");
        println!("  docker compose logs");
    } else {
        println!("Running diagnostics...\n");
        println!("Use the TUI dashboard for comprehensive monitoring:");
        println!("  - System resources (CPU, Memory, Disk)");
        println!("  - Service health status");
        println!("  - Real-time logs with filtering");
        println!("  - SSL certificate status");
        println!("\nRun: igra-cli");
    }

    Ok(())
}

async fn handle_watch(filter: String, record: Option<String>, format: String) -> Result<()> {
    use screens::watch::run_watch_tui;

    println!("Starting L2 transaction monitor...");
    println!("Connecting to execution layer at http://localhost:9545");

    if let Some(ref path) = record {
        println!("Recording transactions to: {}", path);
        println!("Format: {}", format);
    }

    println!("\nPress 'q' to quit, '↑↓' to scroll, 'f' to toggle filter\n");

    run_watch_tui(filter, record, format).await
}
