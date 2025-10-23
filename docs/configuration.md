# Configuration Guide

`igra-cli` reads configuration from the IGRA Orchestra `.env` file located in the repository root.

## Configuration File Location

The tool expects to find configuration at:
```
~/igra-orchestra-public/.env
```

Or run from the IGRA Orchestra directory:
```bash
cd ~/igra-orchestra-public
igra-cli
```

## Auto-Discovery of IGRA Orchestra

`igra-cli` automatically discovers your IGRA Orchestra installation using multiple detection methods (in order of priority):

### 1. Saved Configuration
Reads `~/.config/igra-cli/config.toml` for previously detected path:
```toml
project_root = "/home/user/igra-orchestra-public"
```

### 2. Environment Variable
Set `IGRA_PROJECT_ROOT` to specify the installation path:
```bash
export IGRA_PROJECT_ROOT=/path/to/igra-orchestra-public
igra-cli
```

### 3. Docker Container Inspection
Automatically scans running containers for bind mounts:
- Inspects containers: `traefik`, `kaswallet-0`, `execution-layer`, `kaspad`, `viaduct`
- Extracts host paths from bind mounts (excludes Docker volumes)
- Walks up directory tree looking for `docker-compose.yml`
- Saves discovered path to config for future use

### 4. Current Directory Search
Walks up from current working directory searching for `docker-compose.yml`

### 5. Common Paths
Checks these standard locations:
- `/home/kaspa/igra2/igra-orchestra-public`
- `/home/kaspa/igra-orchestra-public`
- `./igra-orchestra-public` (relative to current directory)
- `../igra-orchestra-public` (parent directory)

### 6. Manual Specification
If auto-detection fails, you'll see a helpful error message with options to manually specify the path.

The discovered path is automatically saved to `~/.config/igra-cli/config.toml` for faster startup next time.

## Key Configuration Variables

### Docker Configuration
- `COMPOSE_PROJECT_NAME` - Docker Compose project name
- `DOCKER_HOST` - Docker daemon host (default: unix:///var/run/docker.sock)

### Wallet Configuration
- `W0_WALLET_TO_ADDRESS` through `W4_WALLET_TO_ADDRESS` - Kaspa wallet addresses for RPC workers
- Wallet keys are stored in `keys/keys.kaswallet-N.json` files

### RPC Access Tokens
- `RPC_ACCESS_TOKEN_1` through `RPC_ACCESS_TOKEN_46` - Access tokens for RPC endpoints
- Used for secure API access via Traefik reverse proxy

### Web Server Configuration
- `IGRA_WEB_TOKEN` - Authentication token for Web Management UI
- Set as environment variable when starting the web server:
  ```bash
  IGRA_WEB_TOKEN=your-secret-token igra-cli serve
  ```

### Network Configuration
- `NETWORK` - Network type (testnet or mainnet)
- `IGRA_CHAIN_ID` - L2 chain ID (19416 for testnet)

### Monitoring Configuration
- `NODE_ID` - Unique identifier for health monitoring
- Used by Node Health Check Client to report status

## Viewing Configuration

### Using the TUI

Launch the TUI and press `5` or navigate to the Config screen:
```bash
igra-cli
# Press 5 for Config screen
```

Features:
- View all environment variables
- Search for specific keys with `/`
- Configuration validation

### Using the Web UI

1. Start the web server:
   ```bash
   IGRA_WEB_TOKEN=your-secret-token igra-cli serve
   ```

2. Access the Config API endpoint:
   ```bash
   curl http://localhost:3000/api/config \
     -H "Authorization: Bearer your-secret-token"
   ```

## Environment Variable Priority

Configuration is loaded in the following order (later sources override earlier ones):

1. `.env` file in IGRA Orchestra directory
2. System environment variables
3. Command-line arguments (for web server options)

## Web Server Options

When running the web server, additional options can be configured:

```bash
igra-cli serve [OPTIONS]
```

**Options:**
- `--host <HOST>` - Bind address (default: 127.0.0.1)
- `--port <PORT>` - Port number (default: 3000)
- `--cors` - Enable CORS for cross-origin requests

**Example:**
```bash
IGRA_WEB_TOKEN=my-token igra-cli serve --host 0.0.0.0 --port 8080 --cors
```

## Security Best Practices

1. **Protect your tokens:**
   - Never commit `.env` with real tokens to git
   - Use strong, random tokens for `IGRA_WEB_TOKEN` and `RPC_ACCESS_TOKEN_*`
   - Rotate tokens periodically

2. **Web server access:**
   - Use `--host 127.0.0.1` for localhost-only access
   - Use `--host 0.0.0.0` only when remote access is needed
   - Consider using a reverse proxy (nginx/Traefik) with SSL/TLS

3. **File permissions:**
   - Ensure `.env` has restricted permissions:
     ```bash
     chmod 600 .env
     ```
   - Protect wallet key files in `keys/` directory:
     ```bash
     chmod 600 keys/keys.kaswallet-*.json
     ```

## Troubleshooting

**Config not loading:**
- Verify you're in the correct directory
- Check `.env` file exists and is readable
- Ensure no syntax errors in `.env` (use `KEY=value` format)

**Missing variables:**
- Check the Config screen for validation errors
- Compare with `.env.backend.example` in the repository

**Web UI authentication fails:**
- Verify `IGRA_WEB_TOKEN` is set when starting server
- Check token matches what you enter in browser login

For more issues, see [Troubleshooting Guide](troubleshooting.md).
