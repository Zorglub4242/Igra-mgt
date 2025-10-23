# Web Management UI Guide

The Web Management UI provides a modern browser-based interface for managing IGRA Orchestra nodes remotely. Built with React and embedded into the `igra-cli` binary.

## Features

### 🌐 Dashboard Panels

- **System Dashboard**: Real-time CPU, RAM, Disk, and OS info in header
- **Services Panel**: Monitor and control Docker containers
  - Real-time service status with health indicators
  - Start, stop, restart services with one click
  - Auto-refresh every 5 seconds
- **Profile Controls**: Manage Docker Compose profiles
  - Start/stop kaspad, backend, frontend profiles
  - Quick service group management
- **Wallets Panel**: Full Kaspa wallet management
  - View all wallet addresses with copy button
  - Check balances and fees
  - Monitor wallet daemon status
- **Transaction History**: Click on wallet to view UTXO history
- **Storage Monitor**: Track Docker disk usage
  - System disk usage visualization
  - Docker storage breakdown (volumes, images, containers)
  - One-click cleanup of unused images/cache
  - Reclaimable space tracking
- **Monitoring Integration**: Embedded Grafana dashboard for metrics
- **Log Viewer**: Real-time service logs with WebSocket streaming

### 🔐 Security

- **Token Authentication**: Secure API with `IGRA_WEB_TOKEN` environment variable
- **CORS Support**: Enable cross-origin requests with `--cors` flag
- **Embedded Assets**: Single binary includes full web UI (no separate files)

## Getting Started

To install and run the Web UI, see the **[Installation Guide](installation.md#web-ui-installation-optional)**.

Quick reference:
```bash
# Run temporarily
IGRA_WEB_TOKEN=your-secret-token igra-cli serve --host 0.0.0.0 --port 3000 --cors

# Install as systemd service (production)
sudo igra-cli install-service --port 3000 --host 0.0.0.0 --cors
```

Access at `http://your-server:3000` and login with your token.

For detailed installation instructions, systemd service setup, and troubleshooting, see [Installation Guide](installation.md#web-ui-installation-optional).

## API Endpoints

The web UI communicates with the backend via these REST API endpoints:

### Services
- `GET /api/services` - List all Docker services
- `POST /api/services/:name/start` - Start a service
- `POST /api/services/:name/stop` - Stop a service
- `POST /api/services/:name/restart` - Restart a service
- `GET /api/services/:name/logs` - Get service logs

### Profiles
- `GET /api/profiles` - List compose profiles
- `POST /api/profiles/:name/start` - Start a profile
- `POST /api/profiles/:name/stop` - Stop a profile

### Wallets
- `GET /api/wallets` - List all wallets with balances and fees
- `GET /api/wallets/:id/detail` - Get wallet transaction history (UTXOs)

### System
- `GET /api/storage` - Get storage information
- `GET /api/system` - Get system resources (CPU, RAM, disk, OS)
- `GET /api/config` - Get configuration
- `GET /api/health` - Health check

### WebSocket
- `GET /ws/logs/:service` - WebSocket log stream for real-time logs

## Development

For Web UI development information, see [Development Guide](development.md) and [igra-web-ui/README.md](../igra-web-ui/README.md).

## Troubleshooting

See [Troubleshooting Guide](troubleshooting.md) for common Web UI issues.
