# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned for v0.3.0
- gRPC integration for kaswallet-daemon (balance querying, transaction sending)
- RPC token generation and testing
- Configuration editing directly in TUI
- Enhanced log filtering and search capabilities

## [0.2.1] - 2025-10-18

### Added
- Intelligent log parsing for all IGRA services
- Real-time service metrics extraction and display
- Service-specific status indicators:
  - kaspad: Sync status (Synced/Syncing) and TPS
  - execution-layer: Block number, transaction count, peer count
  - viaduct: DAA score, bridge latency, queue length
  - block-builder: Build status and transaction count
  - rpc-provider: Request rate and average latency
  - kaswallet: Sync and readiness status
  - node-health-check: Sync lag detection
  - traefik: SSL status and error tracking
- New "Metrics" column in Services screen
- Color-coded health indicators (green=healthy, yellow=warning)
- MIT LICENSE file
- Comprehensive repository metadata in Cargo.toml
- This CHANGELOG

### Changed
- Services table layout adjusted to accommodate Metrics column
- Enhanced dashboard with real-time intelligent status parsing
- Improved user visibility into node sync status and performance

### Technical
- Created `log_parser.rs` module with regex-based pattern matching
- Added `ServiceMetrics` struct to `ContainerInfo`
- Integrated log fetching and parsing into `DockerManager::list_containers()`
- Optimized regex compilation with `OnceLock`
- Added comprehensive test coverage for parsers

## [0.2.0] - 2025-10-18

### Added
- Full-featured TUI with 7 interactive screens
  1. Services - Monitor and manage Docker containers
  2. Profiles - Start service groups
  3. Wallets - View wallet addresses
  4. RPC Tokens - Manage RPC access tokens
  5. SSL Info - Check SSL certificates
  6. Config - View environment configuration
  7. Logs - Real-time log viewer with auto-scroll
- Real-time service monitoring with 2-second refresh
- Resource metrics (CPU, Memory, Disk usage)
- Network I/O monitoring (RX/TX) for each service
- Color-coded resource alerts (>80% red, >60% yellow)
- Interactive log viewer with auto-scroll
- Search/filter functionality for Services, Wallets, and Config screens
- Wallet address display (read from keys files)
- Service control (start, stop, restart) directly from TUI
- Profile-based service groups
- Context-sensitive help system (press `?`)
- Comprehensive README.md and USER_GUIDE.md documentation

### Technical
- Built with Rust 2021 Edition
- Ratatui v0.26 for TUI framework
- Crossterm v0.27 for terminal backend
- Bollard v0.16 for Docker SDK integration
- Tokio async runtime
- Complete codebase with all TODOs resolved

### Known Limitations
- Wallet balances require kaswallet-daemon gRPC integration (shows "N/A")
- Transaction sending UI implemented but requires gRPC backend
- RPC token generation not yet automated
- Backup/restore requires manual procedures

## [0.1.0] - 2025-10-17

### Added
- Initial project structure
- Basic Docker integration via Bollard
- Docker Compose command execution
- Service status display
- Container listing and filtering
- Basic CLI commands (status, start, stop, restart, logs)
- Configuration management (view, validate)
- RPC token listing
- SSL certificate checking
- Wallet address reading from keys files
- Project root auto-discovery
- Service definitions for IGRA Orchestra

### Technical
- Core modules: docker, config, wallet, rpc, ssl
- Utils: constants, helpers, app_config
- CLI argument parsing with Clap
- Error handling with anyhow
- Initial documentation

---

## Version History Summary

- **v0.2.1** (2025-10-18): Intelligent log parsing and metrics
- **v0.2.0** (2025-10-18): Full TUI with 7 screens, search, monitoring
- **v0.1.0** (2025-10-17): Initial release with basic Docker integration

## Links

- [Repository](https://github.com/Zorglub4242/Igra-mgt)
- [Issues](https://github.com/Zorglub4242/Igra-mgt/issues)
- [IGRA Orchestra](https://github.com/igralabs/igra-orchestra-public)

---

**Made with Rust** 🦀 | **Powered by IGRA** ⚡
