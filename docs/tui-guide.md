# TUI (Terminal User Interface) Guide

The `igra-cli` TUI provides an interactive terminal dashboard for managing and monitoring IGRA Orchestra nodes.

## Launching the TUI

```bash
# From your IGRA Orchestra directory
cd ~/igra-orchestra-public
igra-cli
```

## Screens

The TUI has 8 full-featured screens accessible via arrow keys or number keys:

1. **Services** - Docker container monitoring and management
2. **Wallets** - Wallet addresses and daemon status
3. **Watch** - Real-time service metrics
4. **Logs** - Enhanced log viewer with filtering
5. **Config** - Environment variable configuration
6. **Storage** - Disk usage and Docker storage analysis
7. **RPC/SSL** - Access tokens and SSL certificates
8. **Help** - Context-sensitive help

## Keyboard Shortcuts

### Navigation
- **Left/Right Arrows** - Navigate between main screens (Services ↔ Wallets ↔ Watch ↔ Logs ↔ Config)
- **Tab** - Switch sub-views within screens (e.g., Services ↔ Profiles)
- **Up/Down Arrows** / **j/k** - Navigate lists
- **Ctrl+Up/Down** - Fast scroll (10 lines at a time)
- **Ctrl+Shift+Up/Down** - Jump to beginning/end
- **Number Keys (1-8)** - Direct screen access

### Actions
- **Enter** - Select / Activate
- **r** - Restart selected service
- **s** - Stop service
- **d** - View detailed logs
- **/** - Universal search (on Services, Wallets, Config screens)
- **?** - Show help
- **q** - Quit

### Log Viewer Specific
- **g** - Toggle log grouping (by level/module vs chronological)
- **l** - Toggle live mode (auto-refresh every 250ms)
- **Arrow keys** - Scroll 5 lines
- **Ctrl+Arrow** - Scroll 50 lines
- **PageUp/PageDown** - Scroll 100 lines
- **Ctrl+Shift+Up/Down** - Jump to top/bottom

## Features by Screen

### 📊 Services Screen

**Features:**
- Container status with health indicators
- Real-time CPU, Memory, Disk usage per container
- Network I/O monitoring (RX/TX)
- Color-coded alerts (Red >80%, Yellow >60%)
- Service control (start, stop, restart)

**Actions:**
- Press **r** on a service to restart it
- Press **s** to stop a service
- Press **d** to view logs
- Press **/** to search by name, status, or image

### 💼 Wallets Screen

**Features:**
- View wallet addresses from keys files
- Multi-wallet support (kaswallet-0 through kaswallet-4)
- Container status tracking
- Transaction UI (requires gRPC integration)

**Actions:**
- Press **/** to search by wallet address

### 🔍 Watch Screen

**Features:**
- Real-time service metrics
- 2-second refresh rate
- Resource consumption tracking

### 📋 Logs Screen (Enhanced in v0.7.0)

**Features:**
- High-performance rendering (100× faster scrolling)
- Live mode with auto-refresh (250ms)
- Intelligent parsing for multiple log formats
- Dual display modes: grouped (by level/module) or chronological
- Level filtering: Error, Warn, Info, Debug, Trace
- Rolling buffer: 10,000 lines with automatic trimming
- Ultra-compact layout for maximum viewing space

**Actions:**
- Press **l** to toggle live mode
- Press **g** to toggle grouping mode
- Use scroll shortcuts for navigation

### ⚙️ Config Screen

**Features:**
- View all .env environment variables
- Configuration validation
- Search for specific keys

**Actions:**
- Press **/** to search configuration keys

### 💾 Storage Screen (NEW in v0.8.0)

**Features:**
- System disk monitoring
- Docker images, volumes, containers, build cache tracking
- Volume details with size and status
- 90-day historical tracking with growth prediction
- Capacity alerts for approaching limits
- One-key cleanup tools for build cache and unused images
- Space reclamation tracking

**Actions:**
- Navigate through volumes with arrow keys
- Prune build cache and images with designated keys

### 🔐 RPC/SSL Screen

**Features:**
- View all RPC access tokens with endpoints
- SSL certificate status and expiry info
- DNS-01 challenge configuration

### 📈 System Monitoring (Header)

The header shows system-wide metrics:
- CPU usage
- Memory usage
- Disk usage
- Health check status

## Search & Filter

**Universal Search** (press `/` on supported screens):
- Services: Search by name, status, or image
- Wallets: Search by address
- Config: Search by configuration key
- Real-time filtering with highlighted results

## Known Limitations

These features require additional integration:
- **Wallet Balances** - Needs kaswallet-daemon gRPC API integration
- **Send Transactions** - Requires kaswallet-daemon gRPC API
- **RPC Token Generation** - Automated token creation
- **Backup/Restore** - Automated backup functionality

## Tips

1. **Press ?** on any screen for context-sensitive help
2. Use **number keys** for quick screen switching
3. Enable **live mode** in Logs screen for real-time monitoring
4. Use **Ctrl+Shift** shortcuts to quickly jump to list extremes
5. **Search** is your friend - use `/` to filter large lists
