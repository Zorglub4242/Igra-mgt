# IGRA Orchestra CLI

A comprehensive terminal-based management tool for IGRA Orchestra node operators. Built with Rust for performance, reliability, and single-binary distribution.

![IGRA CLI Dashboard](https://img.shields.io/badge/version-0.8.0-blue) ![Rust](https://img.shields.io/badge/rust-1.70%2B-orange) ![License](https://img.shields.io/badge/license-MIT-green)

## Overview

The IGRA CLI is a powerful terminal user interface (TUI) that provides real-time monitoring and management of your IGRA Orchestra node infrastructure. It replaces multiple Docker and CLI commands with an intuitive, keyboard-driven interface.

## Features

### ✅ Fully Implemented

#### 🖥️ Interactive TUI Dashboard
- **8 Full-Featured Screens**: Services, Wallets, Watch, Config, Storage, and more
- **Real-time Updates**: 2-second refresh for live monitoring
- **Keyboard Navigation**: Arrow keys, Tab, numbers for screen switching
- **Help System**: Press `?` on any screen for context-sensitive help

#### 📊 Service Monitoring & Management
- **Container Status**: View all running services with health status
- **Resource Metrics**: Real-time CPU, Memory, Disk usage per container
- **Network I/O**: Monitor network traffic (RX/TX) for each service
- **Color-Coded Alerts**: Red (>80%), Yellow (>60%) for resource warnings
- **Service Control**: Start, stop, restart services directly from TUI
- **Enhanced Log Viewer** (NEW in v0.7.0):
  - **High-performance rendering**: Parse-once architecture (100× faster scrolling)
  - **Live mode**: Auto-refresh every 250ms with automatic viewport scroll
  - **Intelligent parsing**: Supports multiple log formats (block-builder, viaduct, execution-layer/reth)
  - **Dual display modes**: Toggle between grouped (by level/module) and chronological views
  - **Level filtering**: Filter by Error, Warn, Info, Debug, Trace
  - **Smart scrolling**:
    - Arrow keys: 5 lines
    - Ctrl+Arrow: 50 lines
    - PageUp/PageDown: 100 lines
    - Ctrl+Shift+Up/Down: Jump to top/bottom
  - **Ultra-compact layout**: Single-line title bar maximizes log viewing space
  - **Visual indicators**: Live mode status, scroll position, filter info in title
  - **Rolling buffer**: 10,000 line buffer with automatic trimming

#### 🔍 Search & Filter
- **Universal Search**: Press `/` to search on Services, Wallets, Config screens
- **Real-time Filtering**: Results highlight as you type
- **Smart Matching**: Search by name, status, image, address, or configuration keys

#### 💼 Wallet Management
- **Address Display**: View wallet addresses from keys files
- **Multi-Wallet Support**: kaswallet-0 through kaswallet-4
- **Container Status**: Track which wallet services are running
- **Transaction UI**: Send dialog interface (requires gRPC integration)

#### 🔐 RPC & SSL Management
- **Token Listing**: View all RPC access tokens with endpoints
- **SSL Certificate Info**: Check Let's Encrypt certificate status and expiry
- **DNS Configuration**: View DNS-01 challenge settings

#### ⚙️ Configuration Management
- **Environment Variables**: View all .env configuration
- **Validation**: Check for missing or invalid settings
- **Search**: Find specific config keys quickly

#### 💾 Storage Analysis (NEW in v0.8.0)
- **Comprehensive Monitoring**: System disk, Docker images, volumes, containers, build cache
- **Volume Details**: All Docker volumes with size, status, and critical marking
- **Growth Prediction**: 90-day historical tracking with trend analysis
- **Capacity Alerts**: Visual warnings for approaching disk limits
- **Cleanup Tools**: One-key pruning of build cache and unused images
- **Scrollable Lists**: Navigate through all volumes with arrow keys
- **Space Reclamation**: Track and report freed space after cleanup

#### 📈 System Monitoring
- **Global Metrics**: System-wide CPU, Memory, Disk usage in header
- **Container Stats**: Per-service resource consumption
- **Health Checks**: Docker health check status for all services

### 🚧 Requires Additional Integration

- **Wallet Balances**: Needs kaswallet-daemon gRPC API integration
- **Send Transactions**: Requires kaswallet-daemon gRPC API
- **RPC Token Generation**: Automated token creation
- **Backup/Restore**: Automated backup functionality

## Installation

### Prerequisites

- **Docker** 23.0+ with Docker Compose V2
- **IGRA Orchestra** repository cloned
- **Rust** 1.70+ (only required if building from source)

### Option 1: Binary Release (Recommended)

Download pre-built binaries from [GitHub Releases](https://github.com/Zorglub4242/Igra-mgt/releases):

**Linux (x86_64):**
```bash
# Download latest release
wget https://github.com/Zorglub4242/Igra-mgt/releases/latest/download/igra-cli-linux-x86_64.tar.gz

# Extract
tar -xzf igra-cli-linux-x86_64.tar.gz

# Install
sudo mv igra-cli /usr/local/bin/
sudo chmod +x /usr/local/bin/igra-cli
```

**Windows (x86_64):**
```powershell
# Download from releases page
# https://github.com/Zorglub4242/Igra-mgt/releases/latest/download/igra-cli-windows-x86_64.zip

# Extract and add to PATH
# Move igra-cli.exe to a directory in your PATH
```

**macOS (Intel/Apple Silicon):**
```bash
# Download from releases page
wget https://github.com/Zorglub4242/Igra-mgt/releases/latest/download/igra-cli-macos-universal.tar.gz

# Extract
tar -xzf igra-cli-macos-universal.tar.gz

# Install
sudo mv igra-cli /usr/local/bin/
sudo chmod +x /usr/local/bin/igra-cli
```

### Option 2: Build from Source

```bash
# Clone the repository
git clone https://github.com/Zorglub4242/Igra-mgt.git
cd Igra-mgt

# Build and install
./build.sh
sudo ./install.sh
```

### Option 3: Manual Build

```bash
# Build release binary
cargo build --release

# Install to system path
sudo cp target/release/igra-cli /usr/local/bin/
```

### Verify Installation

```bash
igra-cli --version
```

## Quick Start

### Launch the TUI

```bash
# From your IGRA Orchestra directory
cd ~/igra-orchestra-public
igra-cli
```

The TUI will open with the Services screen. Use the following keys:

- **Left/Right Arrows**: Navigate between main screens (Services ↔ Wallets ↔ Watch ↔ Logs ↔ Config)
- **Tab**: Switch sub-views within screens (e.g., Services ↔ Profiles)
- **Up/Down Arrows** / **j/k**: Navigate lists
- **Ctrl+Up/Down**: Fast scroll (10 lines at a time)
- **Ctrl+Shift+Up/Down**: Jump to beginning/end
- **Number Keys (1-7)**: Direct screen access
- **Enter**: Select / Activate
- **r**: Restart selected service
- **s**: Stop service
- **d**: View detailed logs
- **g**: Toggle log grouping
- **l**: Toggle live mode
- **?**: Show help
- **q**: Quit

### CLI Commands

```bash
# View service status
igra-cli status

# Start services by profile
igra-cli start --profile backend

# View logs
igra-cli logs viaduct -n 100

# View configuration
igra-cli config view

# Watch L2 transactions in real-time
igra-cli watch

# Watch with filtering
igra-cli watch --filter entry

# Record transactions to file
igra-cli watch --record transactions.json --format json
```

## Usage Guide

See [USER_GUIDE.md](USER_GUIDE.md) for comprehensive documentation including:

- Detailed screen-by-screen walkthrough
- Keyboard shortcuts reference
- Common workflows and tasks
- Troubleshooting guide
- Advanced features

## TUI Screens

| Screen | Key | Description |
|--------|-----|-------------|
| Services | `1` | Monitor and manage Docker containers |
| Profiles | `2` | Start services by profile groups |
| Wallets | `3` | View wallet addresses and balances |
| RPC Tokens | `4` | Manage RPC access tokens |
| SSL Info | `5` | Check SSL certificates and DNS |
| Config | `6` | View environment configuration |
| Logs | `7` | Real-time log viewer |

## Configuration

The CLI automatically discovers your IGRA Orchestra installation by searching for `docker-compose.yml` in the current directory and parent directories.

**Recommended Setup:**
```bash
# Run from IGRA Orchestra root
cd ~/igra-orchestra-public
igra-cli
```

**Environment Variables:**
- Configuration is read from `.env` in the project root
- Wallet keys are read from `keys/keys.kaswallet-*.json`

## Architecture

### Technology Stack

- **Language**: Rust 2021 Edition
- **TUI Framework**: [Ratatui](https://github.com/ratatui-org/ratatui) v0.26
- **Terminal Backend**: [Crossterm](https://github.com/crossterm-rs/crossterm) v0.27
- **Docker SDK**: [Bollard](https://github.com/fussybeaver/bollard) v0.16
- **Async Runtime**: [Tokio](https://tokio.rs/)

### Project Structure

```
src/
├── main.rs              # CLI entry point and command handlers
├── cli.rs               # Command-line argument parsing
├── app.rs               # TUI application state and event handling
├── core/                # Core business logic
│   ├── docker.rs        # Docker API operations
│   ├── config.rs        # Configuration management
│   ├── wallet.rs        # Wallet operations
│   ├── rpc.rs           # RPC token management
│   ├── ssl.rs           # SSL certificate checking
│   ├── backup.rs        # Backup procedures (manual)
│   ├── health.rs        # Health check documentation
│   └── metrics.rs       # Metrics collection documentation
├── screens/
│   ├── mod.rs           # Screen implementations
│   └── dashboard.rs     # TUI rendering logic
├── widgets/
│   └── mod.rs           # Widget documentation
└── utils/
    ├── constants.rs     # Service definitions
    └── helpers.rs       # Utility functions
```

## Development

### Build Commands

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test

# Format code
cargo fmt

# Lint
cargo clippy
```

### Adding Features

1. **New Screens**: Add to `src/screens/dashboard.rs` and update `Screen` enum in `app.rs`
2. **Core Logic**: Add modules to `src/core/`
3. **CLI Commands**: Update `src/cli.rs` and add handlers to `main.rs`
4. **Service Definitions**: Modify `src/utils/constants.rs`

## Troubleshooting

### Docker Connection Issues

```bash
# Check Docker daemon
docker ps

# Add user to docker group
sudo usermod -aG docker $USER
# Log out and back in
```

### Project Root Not Found

```bash
# Ensure you're in IGRA Orchestra directory
cd ~/igra-orchestra-public
igra-cli

# Or specify path
cd /path/to/igra-orchestra-public && igra-cli
```

### Wallet Shows "N/A"

- **Addresses**: Requires `keys/keys.kaswallet-*.json` file
- **Balances**: Requires kaswallet-daemon gRPC integration (not yet implemented)

### Binary Already Running

```bash
# If getting "text file busy" during install
sudo systemctl stop igra-cli  # if running as service
# Or kill the running process
sudo killall igra-cli
./install.sh
```

## Performance

- **Binary Size**: ~8MB (release build, stripped)
- **Memory Usage**: ~10-20MB typical
- **CPU Usage**: <1% idle, ~5% during intensive operations
- **Startup Time**: <100ms

## Roadmap

### v0.3.0 (Planned)
- gRPC integration for kaswallet-daemon (balance, transactions)
- RPC token generation and testing
- Configuration editing in TUI
- Enhanced log filtering and search

### v0.4.0 (Planned)
- Automated backup/restore functionality
- Upgrade manager with version comparison
- Performance graphs and historical metrics
- Alert notifications

### v1.0.0 (Future)
- Complete feature parity with manual operations
- Comprehensive test coverage
- Official stable release

## Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Code Standards

- Follow Rust naming conventions
- Run `cargo fmt` before committing
- Ensure `cargo clippy` passes
- Add documentation for public APIs

## License

MIT License - see LICENSE file for details

## Support & Community

- **Issues**: [GitHub Issues](https://github.com/Zorglub4242/Igra-mgt/issues)
- **Documentation**: See USER_GUIDE.md and inline help (`?` key)
- **IGRA Community**: [Discord Server](https://discord.gg/igra)
- **IGRA Orchestra**: [Main Repository](https://github.com/igralabs/igra-orchestra-public)

## Acknowledgments

Built with ❤️ by Merlin for the IGRA Community

Special thanks to:
- [Ratatui](https://github.com/ratatui-org/ratatui) - Excellent TUI framework
- [Bollard](https://github.com/fussybeaver/bollard) - Docker SDK for Rust
- IGRA Labs team and community
- All contributors and testers

## Screenshots

### Services Screen
Monitor all containers with real-time metrics and health status

### Wallets Screen
Manage multiple kaspa wallets with address and balance information

### Logs Screen
Interactive real-time log viewer with auto-scroll and search

---

## Changelog

### v0.9.0 (2025-10-22) - Historical Storage Charts & Smart Sampling

**New Features: Storage History Visualization** 📈
- 📊 **Historical storage chart**: ASCII line chart showing 90-day storage trends
- 📅 **Time range selection**: Toggle between 7, 30, or 90-day views (`[`, `t`, `]`)
- 📋 **Details table**: View exact measurements with timestamps (`[D]`)
- 🎨 **Color-coded series**: Total (cyan), Volumes (green), Images (yellow)
- ⚡ **Smart 12-hour sampling**: Background snapshots every 12 hours (2 per day)
- 🔄 **Auto-downsampling**: Migrates old high-frequency data automatically
- 🚀 **Dual-mode capture**: On startup + periodic (6-hour checks)
- 💾 **Efficient storage**: ~34 KB for 90 days vs 37 MB with old method
- 🧹 **Post-cleanup snapshots**: Automatic snapshot after prune operations

**Bug Fixes:**
- ✅ Fixed key conflicts: Changed `7/3/9` → `[/t/]` (no screen navigation clash)
- ✅ Fixed `d` key conflict: Changed to `[D]` (capital D for details toggle)
- ✅ Fixed volume list spacing: Now uses full available screen space
- ✅ Removed excessive storage sampling: From 30s to 12h intervals

**Technical Improvements:**
- Background tasks for passive data collection
- Downsampling algorithm with time bucketing
- Separate display cache (30s) from history sampling (12h)
- Maintains trend quality with 1440× less data

**Keyboard Shortcuts (Storage Screen):**
- `[` - Show last 7 days
- `t` - Show last 30 days
- `]` - Show last 90 days (default)
- `D` - Toggle details table
- `p` - Prune build cache
- `I` - Prune unused images

---

### v0.8.0 (2025-10-21) - Storage Analyzer

**New Feature: Storage Analysis Screen** 📊
- 💾 **Comprehensive storage monitoring**: System disk, Docker images, volumes, containers, build cache
- 📈 **Growth prediction**: Track storage usage trends with 90-day historical data
- ⚠️ **Capacity alerts**: Visual warnings when approaching 90% disk usage
- 🧹 **Cleanup tools**: One-key pruning of build cache (`[p]`) and unused images (`[I]`)
- 📋 **Volume analysis**: All Docker volumes listed with size, status, and critical marking
- ↕️ **Scrollable lists**: Navigate through all volumes with `↑↓` arrow keys
- 💡 **Reclaimable space tracking**: See total space that can be freed

**Storage Screen Features:**
- System disk usage with progress visualization
- Docker storage breakdown by category
- Individual volume sizes (requires sudo)
- Critical volume highlighting (viaduct_data)
- Growth rate calculation (bytes/day)
- Days-to-full prediction
- Auto-refresh with 30-second cache
- Space reclamation reporting after cleanup

**Technical Implementation:**
- New `core::storage` module with comprehensive analysis
- JSON-based history tracking (`~/.config/igra-cli/storage_history.json`)
- Docker system df integration
- Volume size detection via sudo du
- Linear trend analysis for predictions

---

### v0.7.0 (2025-10-21) - Performance & UX Overhaul

**Major Performance Improvements:**
- 🚀 **100× faster log scrolling**: Parse-once architecture eliminates redundant regex operations
- ⚡ **Optimized rendering**: Pre-parsed log cache with instant filtering and windowing
- 📊 **Reduced CPU usage**: From 200,000 regex ops/sec to ~200 ops/sec during scrolling

**Live Mode Enhancements:**
- 🔴 **Real-time updates**: 250ms polling interval for near-instant log visibility
- 📺 **Auto-scroll viewport**: Logs automatically appear at bottom without manual scrolling
- 🎯 **Smart deduplication**: Overlap handling prevents duplicate log lines
- 💾 **Rolling buffer**: 10,000 line buffer with automatic trimming

**UI/UX Improvements:**
- 🎨 **Ultra-compact layout**: Single-line title bar reclaims 8 lines for log viewing
- 📍 **Visual indicators**: Live mode badge, scroll position counter, mode indicator in title
- 🔄 **Dual display modes**: Toggle between grouped and chronological log views
- 🎛️ **Enhanced filtering**: Level-based filtering (Error/Warn/Info/Debug/Trace)
- ⌨️ **Improved scroll speeds**: 5/50/100 line jumps for faster navigation
- 🔧 **Compact metrics panel**: Reth metrics optimized to eliminate empty space

**Bug Fixes:**
- ✅ Fixed scroll offset not affecting visible logs
- ✅ Fixed chronological mode rendering empty screen
- ✅ Fixed live mode hanging on `docker compose logs --since`
- ✅ Fixed metrics panel showing 3 empty lines

**Technical Changes:**
- Created shared `core::log_parser` module for centralized parsing
- Added `ParsedLogLine` and `LogLevel` types with format auto-detection
- Implemented viewport scrolling with `.scroll()` for proper auto-follow
- Removed redundant type definitions and duplicate code paths

---

**Made with Rust** 🦀 | **Powered by IGRA** ⚡
