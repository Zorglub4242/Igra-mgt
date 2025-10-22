# Changelog

All notable changes to `igra-cli` will be documented in this file.

## [0.10.0] - 2025-10-22

### Added
- **Web Management UI**: Full browser-based interface for remote management
  - Modern React web UI embedded in single binary
  - Real-time monitoring with auto-refresh
  - System dashboard with CPU, RAM, Disk, OS info
  - Services panel with start/stop/restart controls
  - Profile management for Docker Compose profiles
  - Wallets panel with addresses, balances, and fees
  - Transaction history viewer (UTXO details)
  - Storage monitor with Docker cleanup tools
  - Embedded Grafana dashboard integration
  - Real-time log viewer with WebSocket streaming
  - Token authentication via `IGRA_WEB_TOKEN`
  - CORS support for cross-origin requests
  - Single binary deployment with embedded assets

### Fixed
- MonitoringPanel layout now respects header/footer (removed full-screen overlay)
- Footer version updated from 0.9.0 to 0.10.0

### Documentation
- Added comprehensive Web UI documentation
- Systemd service setup guide
- API endpoint reference

## [0.9.1] - 2024

### Changed
- Version badge update
- Documentation improvements

## [0.9.0] - 2024

### Added
- Enhanced performance optimizations
- UI refinements

## [0.8.0] - 2024

### Added
- **Storage Analysis Screen**: Comprehensive disk monitoring
  - System disk usage tracking
  - Docker images, volumes, containers, build cache breakdown
  - Volume details with size and status
  - 90-day historical tracking with growth prediction
  - Capacity alerts for approaching disk limits
  - One-key cleanup tools for build cache and unused images
  - Space reclamation tracking

### Enhanced
- Storage monitoring capabilities
- Disk usage visualization

## [0.7.0] - 2024

### Added
- **Enhanced Log Viewer**:
  - High-performance rendering with parse-once architecture (100× faster scrolling)
  - Live mode with 250ms auto-refresh
  - Intelligent parsing for multiple log formats (block-builder, viaduct, reth)
  - Dual display modes: grouped (by level/module) and chronological
  - Level filtering: Error, Warn, Info, Debug, Trace
  - Smart scrolling with multiple speeds
  - Ultra-compact layout maximizing viewing space
  - Visual indicators for live mode, scroll position, and filters
  - Rolling 10,000 line buffer with automatic trimming

### Improved
- Log viewer performance
- Log parsing accuracy
- User experience with better visual feedback

## [0.6.0] - 2024

### Added
- RPC & SSL Management screen
- Token listing with endpoints
- SSL certificate status and expiry info
- DNS-01 challenge configuration view

## [0.5.0] - 2024

### Added
- Wallet management features
- Address display from key files
- Multi-wallet support (kaswallet-0 through kaswallet-4)
- Container status tracking for wallet daemons

## [0.4.0] - 2024

### Added
- Configuration management screen
- Environment variable viewing
- Configuration validation
- Search functionality for config keys

## [0.3.0] - 2024

### Added
- Service monitoring and management
- Container status with health indicators
- Real-time CPU, Memory, Disk usage per container
- Network I/O monitoring (RX/TX)
- Color-coded alerts (Red >80%, Yellow >60%)
- Service control (start, stop, restart)

## [0.2.0] - 2024

### Added
- Interactive TUI with Ratatui
- Multiple screens with keyboard navigation
- Real-time 2-second refresh
- Docker API integration with Bollard

## [0.1.0] - 2024

### Added
- Initial release
- Basic CLI functionality
- Docker Compose integration
- Service status viewing

---

## Version History Summary

- **v0.10.0**: Web Management UI, full browser-based remote management
- **v0.9.x**: Performance and UI refinements
- **v0.8.0**: Storage analysis and disk monitoring
- **v0.7.0**: Enhanced log viewer with high-performance rendering
- **v0.6.0**: RPC & SSL management
- **v0.5.0**: Wallet management
- **v0.4.0**: Configuration management
- **v0.3.0**: Service monitoring
- **v0.2.0**: Interactive TUI
- **v0.1.0**: Initial release

## Versioning

This project follows [Semantic Versioning](https://semver.org/):
- **MAJOR** version for incompatible API changes
- **MINOR** version for new functionality in a backward compatible manner
- **PATCH** version for backward compatible bug fixes
