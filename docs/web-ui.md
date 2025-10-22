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

## Usage

### Starting the Web Server

**Localhost only (default):**
```bash
IGRA_WEB_TOKEN=your-secret-token igra-cli serve
```

**Accessible from network:**
```bash
IGRA_WEB_TOKEN=your-secret-token igra-cli serve --host 0.0.0.0 --port 3000 --cors
```

### Accessing the Web UI

1. Open browser: `http://your-server:3000`
2. Enter your `IGRA_WEB_TOKEN` to login
3. Use the navigation tabs to access different panels

## Running as a System Service

For production deployments, run the web server as a systemd service:

### Create Service File

```bash
sudo nano /etc/systemd/system/igra-web.service
```

### Service File Content

```ini
[Unit]
Description=IGRA Orchestra Web Management Interface
After=network.target docker.service
Requires=docker.service

[Service]
Type=simple
User=your-username
WorkingDirectory=/path/to/igra-orchestra-public
Environment="IGRA_WEB_TOKEN=your-secret-token"
ExecStart=/usr/local/bin/igra-cli serve --host 0.0.0.0 --port 3000 --cors
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

### Enable and Start Service

```bash
# Reload systemd
sudo systemctl daemon-reload

# Enable service to start on boot
sudo systemctl enable igra-web

# Start the service
sudo systemctl start igra-web

# Check status
sudo systemctl status igra-web

# View logs
sudo journalctl -u igra-web -f
```

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
