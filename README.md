# IGRA Orchestra CLI

A comprehensive management tool for IGRA Orchestra node operators. Built with Rust for performance, reliability, and single-binary distribution.

![IGRA CLI Dashboard](https://img.shields.io/badge/version-0.10.0-blue) ![Rust](https://img.shields.io/badge/rust-1.70%2B-orange) ![License](https://img.shields.io/badge/license-MIT-green)

## Quick Install

```bash
curl -fsSL https://raw.githubusercontent.com/Zorglub4242/Igra-mgt/main/quick-install.sh | bash
```

Interactive setup with auto-detection, Web UI configuration, and optional systemd service.

## Overview

`igra-cli` provides both a terminal user interface (TUI) and a modern web-based UI for real-time monitoring and management of your IGRA Orchestra node infrastructure. It replaces multiple Docker and CLI commands with intuitive, easy-to-use interfaces.

## Features

### 🌐 Web Management UI (v0.10.0)
- Browser-based remote management with modern React interface
- Real-time monitoring with auto-refresh
- Service control (start, stop, restart) with one click
- Wallet viewer with balances and transaction history
- Storage monitoring with Docker cleanup tools
- Real-time log streaming via WebSocket
- Token-based authentication
- Single binary deployment with embedded assets

### 🖥️ Terminal User Interface (TUI)
- 8 full-featured screens for comprehensive management
- Real-time updates every 2 seconds
- Keyboard-driven navigation
- Service monitoring with resource metrics
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
- **[Architecture](docs/architecture.md)** - Technology stack, project structure, data flow
- **[Development Guide](docs/development.md)** - Building from source, contributing
- **[Troubleshooting](docs/troubleshooting.md)** - Common issues and solutions
- **[Changelog](docs/changelog.md)** - Version history and release notes

## Key Features

### Service Management
- Monitor Docker container status and health
- Real-time CPU, memory, and network metrics
- Start, stop, restart services
- View detailed logs with filtering

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

# Install as systemd service (requires 'server' feature)
sudo igra-cli install-service [OPTIONS]
  --port <PORT>             # Port number (default: 3000)
  --host <HOST>             # Bind address (default: 0.0.0.0)
  --cors                    # Enable CORS
  --user <USER>             # Service user (default: current user)
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

**Version:** 0.10.0
**Repository:** https://github.com/Zorglub4242/Igra-mgt
**Documentation:** [docs/](docs/)
