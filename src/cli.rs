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
    Status {
        /// Filter by profiles (comma-separated, e.g. kaspad,backend)
        #[arg(short, long)]
        profiles: Option<String>,

        /// Filter by status (comma-separated: healthy,running,stopped,unhealthy)
        #[arg(short, long)]
        status: Option<String>,

        /// Filter by project name
        #[arg(long)]
        project: Option<String>,

        /// Filter by container name (partial match)
        #[arg(short, long)]
        name: Option<String>,

        /// Show all containers (not just current project)
        #[arg(short, long)]
        all: bool,
    },

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

        /// Path to TLS certificate file (enables HTTPS)
        #[arg(long)]
        tls_cert: Option<String>,

        /// Path to TLS private key file
        #[arg(long)]
        tls_key: Option<String>,
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

    /// User management commands
    #[cfg(feature = "server")]
    User {
        #[command(subcommand)]
        command: UserCommands,
    },

    /// Security management commands (IP allowlist, etc.)
    #[cfg(feature = "server")]
    Security {
        #[command(subcommand)]
        command: SecurityCommands,
    },

    /// View audit logs
    #[cfg(feature = "server")]
    Audit {
        #[command(subcommand)]
        command: AuditCommands,
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

#[derive(Subcommand)]
pub enum UserCommands {
    /// List all users
    List,

    /// Add a new user
    Add {
        /// Username
        username: String,

        /// Password (will prompt if not provided)
        #[arg(short, long)]
        password: Option<String>,

        /// Roles (comma-separated: admin,operator,viewer)
        #[arg(short, long, default_value = "viewer")]
        roles: String,
    },

    /// Remove a user
    Remove {
        /// Username to remove
        username: String,
    },

    /// Reset user password
    ResetPassword {
        /// Username
        username: String,

        /// New password (will prompt if not provided)
        #[arg(short, long)]
        password: Option<String>,
    },

    /// Enable/disable a user
    SetEnabled {
        /// Username
        username: String,

        /// Enable (true) or disable (false)
        #[arg(short, long)]
        enabled: bool,
    },

    /// Show user details
    Show {
        /// Username
        username: String,
    },
}

#[derive(Subcommand)]
pub enum SecurityCommands {
    /// IP allowlist management
    Ip {
        #[command(subcommand)]
        command: IpCommands,
    },

    /// Show security configuration
    Show,
}

#[derive(Subcommand)]
pub enum IpCommands {
    /// List allowed IP networks
    List,

    /// Add IP or network to allowlist
    Add {
        /// IP address or CIDR network (e.g., 192.168.1.0/24)
        network: String,
    },

    /// Remove IP or network from allowlist
    Remove {
        /// IP address or CIDR network
        network: String,
    },

    /// Test if an IP is allowed
    Test {
        /// IP address to test
        ip: String,
    },
}

#[derive(Subcommand)]
pub enum AuditCommands {
    /// Show recent audit log entries
    Show {
        /// Number of entries to show
        #[arg(short = 'n', long, default_value = "50")]
        limit: usize,
    },

    /// Export all audit logs to file
    Export {
        /// Output file path
        #[arg(short, long, default_value = "audit-export.json")]
        output: String,
    },

    /// Clear audit logs
    Clear {
        /// Confirm clearing logs
        #[arg(short, long)]
        confirm: bool,
    },
}
