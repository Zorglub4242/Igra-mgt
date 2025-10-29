# IGRA CLI - Development Guide

This guide explains how to set up and use the development environment for IGRA CLI, which provides hot-reloading for both the Rust backend and React frontend.

## Prerequisites

- **Rust**: Install from https://rustup.rs/
- **Node.js & npm**: Install from https://nodejs.org/
- **cargo-watch**: Auto-rebuild tool for Rust
  ```bash
  cargo install cargo-watch
  ```

## Quick Start

The easiest way to start the development environment is using the provided script:

```bash
./dev.sh
```

This will:
1. Install npm dependencies if needed
2. Start the Rust backend on port **8787** with auto-reload
3. Start the Vite dev server on port **5173** with hot-reload
4. Display access URLs for local and LAN access

Press `Ctrl+C` to stop both servers.

## Manual Setup

If you prefer to run the servers separately in different terminals:

### Terminal 1: Backend (Rust)

```bash
cargo watch -x 'run --features server -- serve --host 0.0.0.0 --port 8787'
```

- Watches for changes in `src/` directory
- Auto-compiles on save (~5-10 seconds)
- Restarts the server automatically
- Runs on port **8787** (different from production port 3000)

### Terminal 2: Frontend (React)

```bash
cd igra-web-ui
npm run dev
```

- Watches for changes in `src/` directory
- Hot-reloads on save (instant)
- Runs on port **5173**
- Proxies API calls to backend on port 8787

## Access URLs

### Local Access
- **Frontend**: http://localhost:5173
- **Backend API**: http://localhost:8787/api/...

### LAN Access (from other devices)
Replace `192.168.1.234` with your server's IP:
- **Frontend**: http://192.168.1.234:5173
- **Backend API**: http://192.168.1.234:8787/api/...

To find your IP: `ip addr show | grep "inet "`

## Port Configuration

| Service | Development | Production | Firewall |
|---------|-------------|------------|----------|
| Backend | 8787 | 3000 | ✅ Allowed for LAN |
| Frontend | 5173 | (embedded) | ✅ Allowed for LAN |

The firewall rules allow access from `192.168.1.0/24` network.

## Development Workflow

### Frontend Changes (React/JavaScript)
1. Edit files in `igra-web-ui/src/`
2. Save the file
3. Browser automatically reloads (hot-reload)
4. Changes visible immediately

### Backend Changes (Rust)
1. Edit files in `src/`
2. Save the file
3. cargo-watch detects change and recompiles
4. Server restarts automatically (~5-10 seconds)
5. Refresh browser to see changes

### Simultaneous Testing
You can run both dev and production servers at the same time:
- **Dev**: http://localhost:5173 (ports 8787 + 5173)
- **Prod**: http://localhost:3000 (port 3000)

## Troubleshooting

### Port Already in Use

**Error**: `Address already in use`

**Solution**: Stop the production server or change dev ports in:
- Backend: `dev.sh` (change `--port 8787`)
- Frontend: `igra-web-ui/vite.config.js` (change `port: 5173`)

### Firewall Blocking Access

**Error**: Can't access from other devices on LAN

**Solution**: Verify firewall rules:
```bash
sudo ufw status | grep -E '5173|8787'
```

Should show:
```
5173                       ALLOW       192.168.1.0/24
8787                       ALLOW       192.168.1.0/24
```

If missing, add rules:
```bash
sudo ufw allow from 192.168.1.0/24 to any port 5173 comment 'Vite dev server'
sudo ufw allow from 192.168.1.0/24 to any port 8787 comment 'Rust dev server'
```

### API Requests Failing

**Error**: 404 or connection refused for `/api/*` requests

**Solution**: Check proxy configuration in `igra-web-ui/vite.config.js`:
```javascript
proxy: {
  '/api': {
    target: 'http://localhost:8787',
    changeOrigin: true,
  }
}
```

Ensure backend is running on port 8787.

### cargo-watch Not Found

**Error**: `cargo-watch: command not found`

**Solution**:
```bash
cargo install cargo-watch
```

Add `~/.cargo/bin` to PATH if needed:
```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

## Logs

When using `dev.sh`, logs are saved to:
- Backend: `logs/backend-dev.log`
- Frontend: `logs/frontend-dev.log`

View logs in real-time:
```bash
tail -f logs/backend-dev.log
tail -f logs/frontend-dev.log
```

## Production Build

When ready to deploy changes:

```bash
./build.sh --deploy
```

This will:
1. Build the React frontend (production bundle)
2. Embed assets into Rust binary
3. Build Rust in release mode
4. Stop production server
5. Install new binary to `/usr/local/bin/`
6. Restart production server on port 3000

## Tips

### Fast Iteration on Frontend
If only working on frontend (no backend changes):
```bash
# Keep backend running
cargo run --features server -- serve --host 0.0.0.0 --port 8787

# In another terminal, run Vite dev server
cd igra-web-ui && npm run dev
```

This avoids cargo-watch overhead when you're only editing React components.

### Fast Iteration on Backend
If only working on backend (no frontend changes):
```bash
cargo watch -x 'run --features server -- serve --host 0.0.0.0 --port 8787'
```

Access directly at `http://localhost:8787` (no Vite needed).

### Testing on Mobile Devices
1. Connect phone/tablet to same WiFi
2. Open browser
3. Navigate to `http://192.168.1.234:5173` (use your server's IP)
4. Test responsive design and touch interactions

## Configuration Files

- **`igra-web-ui/vite.config.js`**: Frontend dev server and proxy settings
- **`dev.sh`**: Development launcher script
- **`build.sh`**: Production build script

## Architecture

```
┌─────────────────────┐
│   Browser / Device  │
│  (any on LAN)       │
└──────────┬──────────┘
           │ http://192.168.1.234:5173
           ↓
┌─────────────────────┐
│  Vite Dev Server    │
│  (port 5173)        │
│  - Hot reload       │
│  - Proxy /api → 8787│
└──────────┬──────────┘
           │ proxy
           ↓
┌─────────────────────┐
│  Rust Backend       │
│  (port 8787)        │
│  - cargo-watch      │
│  - Auto-restart     │
└─────────────────────┘
```

## Next Steps

After development, remember to:
1. Test production build locally: `./build.sh`
2. Deploy to production: `./build.sh --deploy`
3. Verify production server: `http://localhost:3000`
