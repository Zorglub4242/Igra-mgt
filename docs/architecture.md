# Architecture

`igra-cli` is a comprehensive management tool for IGRA Orchestra node operators, built with Rust for performance, reliability, and single-binary distribution.

## Technology Stack

### Backend (Rust)
- **Language**: Rust 1.70+
- **TUI Framework**: Ratatui (terminal user interface)
- **Web Framework**: Axum (HTTP server for Web UI)
- **Docker Integration**: Bollard (Docker API client)
- **Configuration**: dotenv for .env file parsing
- **Async Runtime**: Tokio

### Frontend (React)
- **Framework**: React 18+
- **Build Tool**: Vite
- **UI Style**: Custom CSS with dark theme
- **API Client**: Fetch API
- **Real-time**: WebSocket for log streaming

### Distribution
- **Single Binary**: All assets embedded via `rust-embed`
- **Cross-Platform**: Linux, macOS, Windows
- **No Dependencies**: Self-contained executable

## Project Structure

```
tools/igra-cli/
├── src/
│   ├── main.rs              # Entry point, CLI argument parsing
│   ├── tui/                 # Terminal UI implementation
│   │   ├── mod.rs           # TUI app state and event loop
│   │   ├── screens/         # Individual TUI screens
│   │   └── components/      # Reusable TUI components
│   ├── server/              # Web server implementation
│   │   ├── mod.rs           # Axum server setup
│   │   ├── handlers.rs      # API endpoint handlers
│   │   └── websocket.rs     # WebSocket log streaming
│   ├── docker/              # Docker API integration
│   │   ├── mod.rs           # Docker client wrapper
│   │   ├── services.rs      # Service management
│   │   ├── stats.rs         # Resource metrics
│   │   └── storage.rs       # Storage analysis
│   ├── config/              # Configuration management
│   │   ├── mod.rs           # Config loading and parsing
│   │   └── validation.rs    # Config validation
│   └── wallets/             # Wallet management
│       ├── mod.rs           # Wallet key parsing
│       └── addresses.rs     # Address derivation
├── igra-web-ui/             # React web frontend
│   ├── src/
│   │   ├── main.jsx         # React entry point
│   │   ├── App.jsx          # Main app component
│   │   ├── components/      # React components
│   │   │   ├── ServicesPanel.jsx
│   │   │   ├── WalletsPanel.jsx
│   │   │   ├── StoragePanel.jsx
│   │   │   ├── TransactionsPanel.jsx
│   │   │   └── MonitoringPanel.jsx
│   │   └── services/
│   │       └── api.js       # API client
│   ├── dist/                # Built assets (embedded in binary)
│   ├── package.json
│   └── vite.config.js
├── docs/                    # Documentation
├── Cargo.toml               # Rust dependencies
└── README.md
```

## Component Architecture

### TUI (Terminal User Interface)

```
┌─────────────────────────────────────┐
│  Terminal (Ratatui)                 │
│                                     │
│  ┌───────────────────────────────┐ │
│  │  Event Loop                   │ │
│  │  - Keyboard input             │ │
│  │  - Screen updates (2s refresh)│ │
│  └───────────────────────────────┘ │
│                                     │
│  ┌───────────────────────────────┐ │
│  │  Screens                      │ │
│  │  1. Services                  │ │
│  │  2. Wallets                   │ │
│  │  3. Watch                     │ │
│  │  4. Logs                      │ │
│  │  5. Config                    │ │
│  │  6. Storage                   │ │
│  │  7. RPC/SSL                   │ │
│  │  8. Help                      │ │
│  └───────────────────────────────┘ │
│                                     │
│  ┌───────────────────────────────┐ │
│  │  Docker API Client (Bollard)  │ │
│  └───────────────────────────────┘ │
└─────────────────────────────────────┘
```

### Web UI Architecture

```
┌──────────────────────────────────────┐
│  Browser                             │
│                                      │
│  ┌────────────────────────────────┐ │
│  │  React App                     │ │
│  │  - ServicesPanel               │ │
│  │  - WalletsPanel                │ │
│  │  - StoragePanel                │ │
│  │  - TransactionsPanel           │ │
│  │  - MonitoringPanel             │ │
│  └────────────────────────────────┘ │
│           │                          │
│           │ HTTP/WebSocket           │
│           ▼                          │
└──────────────────────────────────────┘
            │
            ▼
┌──────────────────────────────────────┐
│  Axum HTTP Server (Rust)             │
│                                      │
│  ┌────────────────────────────────┐ │
│  │  REST API Handlers             │ │
│  │  - /api/services               │ │
│  │  - /api/wallets                │ │
│  │  - /api/storage                │ │
│  │  - /api/system                 │ │
│  │  - /api/profiles               │ │
│  │  - /ws/logs/:service           │ │
│  └────────────────────────────────┘ │
│                                      │
│  ┌────────────────────────────────┐ │
│  │  Static Assets (rust-embed)    │ │
│  │  - index.html                  │ │
│  │  - JavaScript bundles          │ │
│  │  - CSS stylesheets             │ │
│  └────────────────────────────────┘ │
│                                      │
│  ┌────────────────────────────────┐ │
│  │  Docker API Client (Bollard)   │ │
│  └────────────────────────────────┘ │
└──────────────────────────────────────┘
            │
            ▼
┌──────────────────────────────────────┐
│  Docker Daemon                       │
│  - IGRA Orchestra containers         │
└──────────────────────────────────────┘
```

## Data Flow

### TUI Mode

1. User launches `igra-cli` (no arguments)
2. TUI initializes Ratatui terminal
3. Event loop starts:
   - Listen for keyboard input
   - Refresh screen every 2 seconds
   - Query Docker API for stats
   - Update display
4. User navigates with arrow keys/shortcuts
5. Actions (restart service, view logs) sent to Docker API
6. Results displayed in TUI

### Web Server Mode

1. User launches `igra-cli serve`
2. Axum HTTP server starts on specified host:port
3. React build assets loaded via rust-embed
4. Server listens for:
   - HTTP requests to API endpoints
   - WebSocket connections for log streaming
   - Static asset requests
5. Browser loads React app
6. React app polls API endpoints for data
7. User actions trigger API calls
8. Server executes Docker API calls
9. Results returned as JSON

## Authentication

### Web UI
- Token-based authentication via `IGRA_WEB_TOKEN` environment variable
- Token sent as Bearer token in Authorization header
- All API endpoints protected except `/api/health`

### TUI
- No authentication (local terminal access assumed)
- Requires Docker socket access permissions

## Performance Optimizations

### TUI
- **Parse-once architecture**: Logs parsed once and cached
- **Incremental updates**: Only changed data re-rendered
- **Rolling buffer**: 10,000 line limit for log viewer
- **Smart scrolling**: Batch updates during fast scroll

### Web UI
- **Production build**: Minified JS/CSS bundles
- **Embedded assets**: No separate file I/O
- **WebSocket streaming**: Efficient real-time log delivery
- **Auto-refresh**: Configurable polling intervals (5s default)

### Docker API
- **Connection pooling**: Reuse Docker API connections
- **Async I/O**: Non-blocking Tokio runtime
- **Lazy loading**: Fetch data only when needed

## Security Considerations

1. **Docker socket access**: Requires read/write access to Docker daemon
2. **Web token**: Must be kept secret, transmitted over HTTPS in production
3. **CORS**: Disabled by default, enable only when needed with `--cors`
4. **Localhost binding**: Default `--host 127.0.0.1` prevents external access
5. **No credential storage**: Reads wallet keys directly from files, doesn't cache

## Future Architecture Goals

1. **gRPC integration**: Connect to kaswallet-daemon for balances and transactions
2. **Plugin system**: Allow custom screens/panels
3. **Multi-node support**: Manage multiple IGRA nodes from single CLI
4. **Metrics export**: Prometheus exporter for monitoring integration
