# KASPA L2 Management CLI

A comprehensive management tool for KASPA L2 node operators. Built with Rust for performance, reliability, and single-binary distribution.

![IGRA CLI Dashboard](https://img.shields.io/badge/version-0.12.0-blue) ![Rust](https://img.shields.io/badge/rust-1.70%2B-orange) ![License](https://img.shields.io/badge/license-MIT-green)

## Quick Install

```bash
curl -fsSL https://raw.githubusercontent.com/Zorglub4242/Igra-mgt/main/quick-install.sh | bash
```

Interactive setup with auto-detection, Web UI configuration, and optional systemd service.

## Overview

`igra-cli` provides both a terminal user interface (TUI) and a modern web-based UI for real-time monitoring and management of your KASPA L2 node infrastructure. It replaces multiple Docker and CLI commands with intuitive, easy-to-use interfaces.

## Features

### 🌐 Web Management UI (v0.12.0)
- Browser-based remote management with modern React interface
- **NEW in v0.12.0:** Improved metrics display with plugin-aware visibility
- **NEW in v0.12.0:** Smart metrics detection - services with plugin-based metrics now display correctly
- **NEW in v0.12.0:** Fixed port display deduplication for better clarity
- Clickable service names open detailed service information pages
- Service Details page with 5 tabs showing comprehensive info
- Real-time monitoring with auto-refresh
- Service control (start, stop, restart) with one click
- Wallet viewer with balances and transaction history
- Storage monitoring with Docker cleanup tools
- Real-time log streaming via WebSocket
- **NEW in v0.13.0:** Multi-user session-based authentication with role-based access control
- **NEW in v0.13.0:** IP allowlisting for network security
- **NEW in v0.13.0:** Comprehensive audit logging
- Single binary deployment with embedded assets

### 🖥️ Terminal User Interface (TUI)
- 8 full-featured screens for comprehensive management
- **NEW:** Service Details screen with 6 tabs (Overview, Metrics, Configuration, Storage, Network, Logs)
- Real-time updates every 2 seconds
- Keyboard-driven navigation (press Enter on any service to view details)
- Service monitoring with resource metrics
- Plugin-based metrics system with customizable TOML configs
- Enhanced log viewer with intelligent parsing
- Storage analysis and disk monitoring
- Configuration management

## Quick Start

### Install

**Quick install with Web UI setup:**
```bash
curl -fsSL https://raw.githubusercontent.com/Zorglub4242/Igra-mgt/main/quick-install.sh | bash
```

Or inspect first:
```bash
wget https://raw.githubusercontent.com/Zorglub4242/Igra-mgt/main/quick-install.sh
chmod +x quick-install.sh
./quick-install.sh
```

**Manual install:**
```bash
# Download latest release
wget https://github.com/Zorglub4242/Igra-mgt/releases/latest/download/igra-cli-linux-x86_64.tar.gz
tar -xzf igra-cli-linux-x86_64.tar.gz
sudo mv igra-cli /usr/local/bin/
sudo chmod +x /usr/local/bin/igra-cli
```

For other platforms and installation methods, see **[Installation Guide](docs/installation.md)**.

### Launch the TUI

```bash
cd ~/igra-orchestra-public
igra-cli
```

Use arrow keys to navigate, `?` for help, `q` to quit.

See **[TUI Guide](docs/tui-guide.md)** for keyboard shortcuts and features.

### Launch the Web UI

```bash
# Start web server
IGRA_WEB_TOKEN=your-secret-token igra-cli serve --host 0.0.0.0 --port 3000 --cors

# Open browser: http://your-server:3000
# Login with your IGRA_WEB_TOKEN
```

See **[Web UI Guide](docs/web-ui.md)** for features and systemd service setup.

## Documentation

- **[Installation Guide](docs/installation.md)** - Prerequisites, installation options, verification
- **[Web UI Guide](docs/web-ui.md)** - Web interface features, server usage, API endpoints
- **[TUI Guide](docs/tui-guide.md)** - Terminal interface screens, keyboard shortcuts
- **[Configuration Guide](docs/configuration.md)** - Environment variables, security best practices
- **[Authentication Setup Guide](AUTH_SETUP_GUIDE.md)** - Multi-user authentication, roles, IP security, audit logs
- **[Architecture](docs/architecture.md)** - Technology stack, project structure, data flow
- **[Development Guide](docs/development.md)** - Building from source, contributing
- **[Troubleshooting](docs/troubleshooting.md)** - Common issues and solutions
- **[Changelog](docs/changelog.md)** - Version history and release notes

## Key Features

### Service Management
- Monitor Docker container status and health
- **NEW:** Comprehensive service details with tabbed interface
- **NEW:** Plugin-based metrics system (8 built-in plugins: execution-layer, block-builder, viaduct, traefik, kaswallet, rpc-provider, kaspad, kaspa-miner)
- **NEW:** Customizable metrics via TOML configuration files
- **NEW:** User-editable service notes/descriptions
- Real-time CPU, memory, and network metrics
- Start, stop, restart services
- View detailed logs with filtering
- Configuration inspection (environment variables, volumes, networks, ports)

### Wallet Management
- Display wallet addresses from key files
- View balances and transaction fees
- Transaction history (UTXO details)
- Multi-wallet support (kaswallet-0 through kaswallet-4)

### Storage Monitoring
- System disk usage tracking
- Docker volumes, images, containers breakdown
- Growth prediction and capacity alerts
- One-click cleanup tools

### Configuration
- View all environment variables
- Configuration validation
- Search functionality

### Authentication & Security (v0.13.0)
- **Multi-user authentication** with session-based login
- **Role-based access control (RBAC):**
  - **Admin:** Full system access, user management, configuration
  - **Operator:** Service control, view logs and metrics
  - **Viewer:** Read-only access to all data
- **IP allowlisting** with CIDR notation support
- **Audit logging** for compliance and security monitoring
- Automatic default admin user creation on first launch
- Secure password hashing with Argon2id
- Session expiry and logout functionality

## Requirements

- **Docker** 23.0+ with Docker Compose V2
- **IGRA Orchestra** repository with valid `.env` file
- **Rust** 1.70+ (only for building from source)

## CLI Commands

### Basic Commands

```bash
# Launch interactive TUI
igra-cli

# Show service status
igra-cli status

# View logs for a service
igra-cli logs <service> [-f] [-n LINES]
  -f, --follow          Follow log output
  -n, --tail <LINES>    Number of lines to show (default: 100)
```

### Service Management

```bash
# Start a service or profile
igra-cli start [--profile PROFILE | SERVICE]
  --profile kaspad          # Start kaspad profile
  --profile backend         # Start backend profile
  --profile frontend-w1     # Start frontend with 1 worker

# Stop services
igra-cli stop [--all | SERVICE]
  --all                     # Stop all services

# Restart a service
igra-cli restart <SERVICE>
```

### Web Server

```bash
# Start web management UI (requires 'server' feature)
igra-cli serve [OPTIONS]
  --port <PORT>             # Port number (default: 3000)
  --host <HOST>             # Bind address (default: 127.0.0.1)
  --cors                    # Enable CORS

# Install as system service (requires 'server' feature)
# Linux (systemd):
sudo igra-cli install-service [OPTIONS]
  --port <PORT>             # Port number (default: 3000)
  --host <HOST>             # Bind address (default: 0.0.0.0)
  --cors                    # Enable CORS
  --user <USER>             # Service user (default: current user)

# Windows (Windows Service):
igra-cli.exe install-service [OPTIONS]  # Run as Administrator
  --port <PORT>             # Port number (default: 3000)
  --host <HOST>             # Bind address (default: 0.0.0.0)
  --cors                    # Enable CORS
  # Service name: IgraWebUI
  # Manage with: sc start/stop/query IgraWebUI
```

### RPC Management

```bash
# List RPC tokens
igra-cli rpc tokens list

# Generate RPC tokens
igra-cli rpc tokens generate

# Test RPC token
igra-cli rpc tokens test <TOKEN_NUMBER>

# Test RPC endpoint
igra-cli rpc test-endpoint [--token N]
```

### Wallet Management

```bash
# List all wallets
igra-cli wallet list

# Check wallet balance
igra-cli wallet balance <WORKER_ID>

# Generate new wallet
igra-cli wallet generate <WORKER_ID>
```

### Authentication & Security (v0.13.0)

```bash
# User Management
igra-cli user list                               # List all users
igra-cli user add <username> --roles <roles>     # Add user (roles: admin, operator, viewer)
igra-cli user remove <username>                  # Remove user
igra-cli user reset-password <username>          # Reset user password
igra-cli user set-enabled <username> <true|false> # Enable/disable user
igra-cli user show <username>                    # Show user details

# IP Security
igra-cli security show                           # View security configuration
igra-cli security ip add <network>               # Add allowed network (e.g., 192.168.1.0/24)
igra-cli security ip remove <network>            # Remove allowed network

# Audit Logging
igra-cli audit show [--limit N]                  # Show audit logs
igra-cli audit export --output <file>            # Export audit logs
igra-cli audit clear                             # Clear audit logs

# Examples:
igra-cli user add operator1 --roles operator
igra-cli security ip add 192.168.1.0/24
igra-cli audit show --limit 100
```

### Configuration

```bash
# View configuration
igra-cli config view

# Edit configuration
igra-cli config edit

# Validate configuration
igra-cli config validate

# Generate RPC tokens
igra-cli config generate-tokens
```

### Backup & Restore

```bash
# Create backup
igra-cli backup create <SERVICE>

# List backups
igra-cli backup list

# Restore from backup
igra-cli backup restore <SERVICE> <FILE>
```

### Monitoring & Diagnostics

```bash
# Resource monitoring
igra-cli monitor

# Health check report
igra-cli health

# Run diagnostics
igra-cli diag [--report]

# Check for updates
igra-cli upgrade [--check] [--pull] [--apply]
```

### Transaction Watching

```bash
# Watch L2 transactions in real-time
igra-cli watch [OPTIONS]
  --filter <TYPE>           # Filter by type: all, transfer, contract, entry (default: all)
  --record <FILE>           # Record transactions to file
  --format <FORMAT>         # Output format: json, csv, text (default: text)
```

### Service Details & Metrics

**TUI:**
- Press `Enter` on any service in the Services screen to view detailed information
- Navigate tabs with `Tab` / `Shift+Tab`
- Scroll content with `↑` / `↓` arrows
- Press `Esc` or `q` to return to services list

**Web UI:**
- Click on any service name to open the service details page
- View tabs: Overview, Metrics, Configuration, Storage, Network
- Auto-refreshes every 5 seconds
- Use back button to return to services list

**Plugin System:**
```bash
# 8 built-in plugins embedded in binary:
#   - reth.toml (execution-layer)
#   - geth.toml
#   - block-builder.toml
#   - viaduct.toml
#   - traefik.toml
#   - kaswallet.toml
#   - rpc-provider.toml
#   - kaspad.toml
#   - kaspa-miner.toml

# Plugins are auto-extracted on first run to:
# Linux:   ~/.config/igra-cli/plugins/
# Windows: %APPDATA%\igra-cli\plugins\

# System-wide locations (optional):
# Linux:   /etc/igra-cli/plugins/
# Windows: %PROGRAMDATA%\igra-cli\plugins\

# Development fallback: ./plugins/

# Create custom plugin (no code changes needed):
# 1. Add my-service.toml to your plugins directory
# 2. Define matchers, fetcher, and metrics
# 3. Restart igra-cli
```

See **[QUICK_REFERENCE.md](QUICK_REFERENCE.md)** for plugin configuration details.

### Other

```bash
# Run setup wizard
igra-cli setup

# Show help
igra-cli --help

# Show version
igra-cli --version
```

## Development

To build from source:

```bash
git clone https://github.com/Zorglub4242/Igra-mgt.git
cd Igra-mgt

# Build Rust binary
cargo build --release

# Build with Web UI
cd igra-web-ui && npm install && npm run build && cd ..
cargo build --release --features server
```

See **[Development Guide](docs/development.md)** for detailed instructions.

## Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests: `cargo test`
5. Submit a Pull Request

See **[Development Guide](docs/development.md)** for coding standards and guidelines.

## Troubleshooting

Having issues? Check the **[Troubleshooting Guide](docs/troubleshooting.md)** for common problems and solutions.

For additional help:
- [Open an issue](https://github.com/Zorglub4242/Igra-mgt/issues)
- Check [existing issues](https://github.com/Zorglub4242/Igra-mgt/issues)

## License

MIT License - see LICENSE file for details.

## Acknowledgments

Built for the IGRA Orchestra project by the community.

---

**Version:** 0.12.0
**Repository:** https://github.com/Zorglub4242/Igra-mgt
**Documentation:** [docs/](docs/)
