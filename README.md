# IGRA Orchestra CLI

A comprehensive management tool for IGRA Orchestra node operators. Built with Rust for performance, reliability, and single-binary distribution.

![IGRA CLI Dashboard](https://img.shields.io/badge/version-0.10.0-blue) ![Rust](https://img.shields.io/badge/rust-1.70%2B-orange) ![License](https://img.shields.io/badge/license-MIT-green)

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

## Screenshots

### TUI Dashboard
![Services Screen](https://via.placeholder.com/800x400?text=TUI+Services+Screen)

### Web UI
![Web Dashboard](https://via.placeholder.com/800x400?text=Web+Management+UI)

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

```bash
# Launch TUI
igra-cli

# Start web server
igra-cli serve [OPTIONS]

# View service status
igra-cli status

# View logs
igra-cli logs <service> [OPTIONS]

# Show help
igra-cli --help
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
