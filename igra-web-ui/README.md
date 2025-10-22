# IGRA Orchestra Web UI

Modern web-based management interface for IGRA Orchestra nodes, built with React and embedded into the `igra-cli` binary.

## Features

- **🐳 Services Dashboard** - Monitor and control Docker containers
  - Real-time CPU, memory, and network stats
  - Start/stop/restart services
  - Health status monitoring
  - Auto-refresh every 5 seconds

- **💼 Wallets Panel** - Manage Kaspa wallets
  - View wallet addresses
  - Check balances
  - Monitor wallet daemon status

- **💾 Storage Analyzer** - Track disk usage
  - System disk usage visualization
  - Docker storage breakdown
  - One-click cleanup of unused images/cache
  - Reclaimable space tracking

## Architecture

```
┌────────────────────────────────┐
│   Single Rust Binary           │
│   (igra-cli)                   │
│                                │
│  ┌──────────────────────────┐ │
│  │  Axum HTTP Server        │ │
│  │  - /api/* → REST API     │ │
│  │  - /* → React UI         │ │
│  └──────────────────────────┘ │
│                                │
│  ┌──────────────────────────┐ │
│  │  Embedded React Build    │ │
│  │  (via rust-embed)        │ │
│  └──────────────────────────┘ │
└────────────────────────────────┘
```

## Development

### Prerequisites

- Node.js 18+ and npm
- Rust 1.75+

### Setup

```bash
cd tools/igra-web-ui
npm install
```

### Development Mode

```bash
# Terminal 1: Start Rust API server
cd tools/igra-cli
cargo run --features server -- serve --cors

# Terminal 2: Start React dev server
cd tools/igra-web-ui
npm run dev
```

Open http://localhost:5173 - Vite will proxy API calls to port 3000.

### Build for Production

```bash
# Build React UI
cd tools/igra-web-ui
npm run build  # Creates dist/

# Build Rust with embedded UI
cd ../igra-cli
cargo build --release --features server

# Run single binary
./target/release/igra-cli serve
```

The React build is embedded into the Rust binary using `rust-embed`.

## Deployment

### Option 1: Single Binary (Recommended)

```bash
# Build everything
./tools/build-ui.sh
cd tools/igra-cli
cargo build --release --features server

# Deploy just the binary
scp target/release/igra-cli user@server:/usr/local/bin/

# Run
igra-cli serve --host 0.0.0.0 --port 3000
```

### Option 2: Docker

```dockerfile
FROM node:20-alpine AS ui-builder
WORKDIR /app
COPY tools/igra-web-ui/ ./
RUN npm ci && npm run build

FROM rust:1.75 AS rust-builder
WORKDIR /app
COPY tools/igra-cli/ ./
COPY --from=ui-builder /app/dist ./dist/
RUN cargo build --release --features server

FROM debian:bookworm-slim
COPY --from=rust-builder /app/target/release/igra-cli /usr/local/bin/
EXPOSE 3000
CMD ["igra-cli", "serve", "--host", "0.0.0.0"]
```

## API Integration

The UI communicates with the Rust backend via REST API:

```javascript
import { api } from './services/api'

// Get services
const services = await api.getServices()

// Control service
await api.restartService('viaduct')

// Get storage info
const storage = await api.getStorage()
```

See `src/services/api.js` for the complete API client.

## Project Structure

```
igra-web-ui/
├── src/
│   ├── main.jsx              # Entry point
│   ├── App.jsx               # Main app component
│   ├── App.css               # Global styles
│   ├── components/           # UI components
│   │   ├── ServicesPanel.jsx
│   │   ├── WalletsPanel.jsx
│   │   └── StoragePanel.jsx
│   └── services/
│       └── api.js            # API client
├── index.html
├── vite.config.js
└── package.json
```

## Customization

### Adding New Components

1. Create component in `src/components/`
2. Add to `App.jsx` as new tab
3. Rebuild: `npm run build`

### Styling

Modify `src/App.css` for global styles. Uses CSS variables for theming.

### API Endpoints

Add new endpoints in the Rust backend (`tools/igra-cli/src/server/handlers.rs`), then update `src/services/api.js` to call them.

## Production Checklist

- [ ] Build UI: `npm run build`
- [ ] Build Rust: `cargo build --release --features server`
- [ ] Test embedded serving: `./target/release/igra-cli serve`
- [ ] Verify all features work at http://localhost:3000
- [ ] Check API endpoints return correct data
- [ ] Test service start/stop/restart
- [ ] Verify storage cleanup works

## Troubleshooting

**UI not loading:**
- Check that React build exists: `ls dist/`
- Rebuild UI: `npm run build`
- Rebuild Rust: `cargo build --features server`

**API errors:**
- Check Rust server is running with `--cors` flag
- Verify Docker is accessible from Rust server
- Check browser console for network errors

**Build errors:**
- Clear cache: `rm -rf node_modules dist && npm install && npm run build`
- Update dependencies: `npm update`

## License

MIT - Same as igra-cli
