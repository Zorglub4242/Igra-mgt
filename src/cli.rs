/// CLI argument parsing and command handling

use clap::{Parser, Subcommand};

// Build timestamp injected at compile time
pub const BUILD_TIMESTAMP: &str = env!("BUILD_TIMESTAMP");
pub const VERSION_WITH_BUILD: &str = concat!(env!("CARGO_PKG_VERSION"), " (built: ", env!("BUILD_TIMESTAMP"), ")");

// Get version with timestamp
pub fn get_version() -> &'static str {
    VERSION_WITH_BUILD
}

#[derive(Parser)]
#[command(name = "igra-cli")]
#[command(author, version = VERSION_WITH_BUILD, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Show service status
    Status,

    /// Start services or profiles
    Start {
        /// Profile to start (kaspad, backend, frontend-w1, etc.)
        #[arg(short, long)]
        profile: Option<String>,

        /// Specific service to start
        service: Option<String>,
    },

    /// Stop services
    Stop {
        /// Stop all services
        #[arg(short, long)]
        all: bool,

        /// Specific service to stop
        service: Option<String>,
    },

    /// Restart a service
    Restart {
        /// Service to restart
        service: String,
    },

    /// View logs
    Logs {
        /// Service name
        service: String,

        /// Follow log output
        #[arg(short, long)]
        follow: bool,

        /// Number of lines to show
        #[arg(short = 'n', long, default_value = "100")]
        tail: usize,
    },

    /// RPC management commands
    Rpc {
        #[command(subcommand)]
        command: RpcCommands,
    },

    /// Wallet management commands
    Wallet {
        #[command(subcommand)]
        command: WalletCommands,
    },

    /// Backup operations
    Backup {
        #[command(subcommand)]
        command: BackupCommands,
    },

    /// Configuration management
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },

    /// Resource monitoring
    Monitor,

    /// Health check report
    Health,

    /// Check for updates
    Upgrade {
        /// Check for updates without pulling
        #[arg(short, long)]
        check: bool,

        /// Pull new images
        #[arg(short, long)]
        pull: bool,

        /// Apply upgrades
        #[arg(short, long)]
        apply: bool,
    },

    /// Run diagnostics
    Diag {
        /// Generate diagnostic report
        #[arg(short, long)]
        report: bool,
    },

    /// Run setup wizard
    Setup,

    /// Watch L2 transactions in real-time
    Watch {
        /// Filter by type (all, transfer, contract, entry)
        #[arg(short, long, default_value = "all")]
        filter: String,

        /// Record transactions to file
        #[arg(short, long)]
        record: Option<String>,

        /// Output format for recording (json, csv, text)
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// Run HTTP API server mode
    #[cfg(feature = "server")]
    Serve {
        /// Port to listen on
        #[arg(short, long, default_value = "3000")]
        port: u16,

        /// Host to bind to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// Enable CORS for cross-origin requests
        #[arg(long)]
        cors: bool,
    },

    /// Install web UI as a systemd service
    #[cfg(feature = "server")]
    InstallService {
        /// Port for the web server
        #[arg(short, long, default_value = "3000")]
        port: u16,

        /// Host to bind to
        #[arg(long, default_value = "0.0.0.0")]
        host: String,

        /// Enable CORS
        #[arg(long)]
        cors: bool,

        /// Service user (default: current user)
        #[arg(short, long)]
        user: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum RpcCommands {
    /// List all RPC tokens
    Tokens {
        #[command(subcommand)]
        command: Option<TokenCommands>,
    },

    /// Test RPC endpoint
    TestEndpoint {
        /// Token number to test (1-46)
        #[arg(short, long)]
        token: Option<usize>,
    },
}

#[derive(Subcommand)]
pub enum TokenCommands {
    /// List all tokens
    List,

    /// Generate all tokens
    Generate,

    /// Test a specific token
    Test { token_number: usize },
}

#[derive(Subcommand)]
pub enum WalletCommands {
    /// List all wallets
    List,

    /// Check wallet balance
    Balance { worker_id: usize },

    /// Generate new wallet
    Generate { worker_id: usize },
}

#[derive(Subcommand)]
pub enum BackupCommands {
    /// Create backup
    Create { service: String },

    /// List backups
    List,

    /// Restore from backup
    Restore { service: String, file: String },
}

#[derive(Subcommand)]
pub enum ConfigCommands {
    /// View configuration
    View,

    /// Edit configuration
    Edit,

    /// Validate configuration
    Validate,

    /// Generate RPC tokens
    GenerateTokens,
}
