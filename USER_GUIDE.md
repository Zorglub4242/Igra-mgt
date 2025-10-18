# IGRA Orchestra CLI - User Guide

Complete guide to using the IGRA Orchestra CLI for managing your node infrastructure.

## Table of Contents

1. [Getting Started](#getting-started)
2. [TUI Overview](#tui-overview)
3. [Screen-by-Screen Guide](#screen-by-screen-guide)
4. [Keyboard Shortcuts](#keyboard-shortcuts)
5. [Common Workflows](#common-workflows)
6. [CLI Commands](#cli-commands)
7. [Advanced Features](#advanced-features)
8. [Troubleshooting](#troubleshooting)
9. [Tips & Best Practices](#tips--best-practices)

---

## Getting Started

### First Launch

```bash
# Navigate to your IGRA Orchestra directory
cd ~/igra-orchestra-public

# Launch the TUI
igra-cli
```

You should see the **Services** screen with a list of Docker containers and their status.

### Understanding the Interface

```
╔════════════════════════════════════════════════════════════════╗
║  IGRA Orchestra Dashboard                                      ║
║  CPU: 45.2%  |  Memory: 62.1%  |  Disk: 78.3%                 ║  ← Header
╠════════════════════════════════════════════════════════════════╣
║                                                                 ║
║  [Services Screen Content]                                     ║  ← Main Content
║                                                                 ║
╠════════════════════════════════════════════════════════════════╣
║  [1] Services  [2] Profiles  [3] Wallets ... [?] Help [q] Quit ║  ← Footer
╚════════════════════════════════════════════════════════════════╝
```

**Components:**
- **Header**: System-wide metrics (CPU, Memory, Disk)
- **Main Content**: Current screen's data
- **Footer**: Navigation hints and status messages

---

## TUI Overview

### Navigation

| Action | Keys |
|--------|------|
| Switch Screens | `1-7` or `Tab` |
| Move Up/Down | `↑`/`↓` or `j`/`k` |
| Scroll Logs | `↑`/`↓` or `PgUp`/`PgDn` |
| Show Help | `?` |
| Quit | `q` or `Ctrl+C` |

### Screen Numbers

1. **Services** - Monitor and manage containers
2. **Profiles** - Start service groups
3. **Wallets** - View wallet addresses and balances
4. **RPC Tokens** - Manage RPC access tokens
5. **SSL Info** - Check SSL certificates
6. **Config** - View environment configuration
7. **Logs** - Real-time log viewer

---

## Screen-by-Screen Guide

### 1. Services Screen

**Purpose**: Monitor and manage Docker containers

**View Information:**
- Service name and status (Running, Stopped, Paused, Restarting)
- Health status (Healthy, Unhealthy, Starting, N/A)
- Resource usage: CPU %, Memory %, Network I/O (RX/TX)
- Docker image name
- Uptime

**Color Coding:**
- **Green**: Healthy status
- **Red**: Unhealthy or >80% resource usage
- **Yellow**: 60-80% resource usage
- **Gray**: Stopped or N/A
- **Cyan**: Search match highlight

**Actions:**

| Key | Action |
|-----|--------|
| `↑`/`↓` | Select service |
| `r` | Restart selected service |
| `s` | Stop selected service |
| `d` | View detailed logs (switches to Logs screen) |
| `/` | Search/filter services |
| `?` | Show help |

**Search Example:**
```
1. Press `/` to enter search mode
2. Type "viaduct"
3. See matching services highlighted in cyan
4. Press Enter to jump to first match
5. Press Esc to cancel search
```

**Common Tasks:**

*Restart a crashed service:*
1. Navigate to the service using arrow keys
2. Press `r` to restart
3. Watch status change from "Restarting" to "Running"

*Monitor resource usage:*
- Check CPU/Memory columns for high usage (red = >80%)
- Network I/O shows live traffic in KB/s or MB/s

---

### 2. Profiles Screen

**Purpose**: Start services by predefined groups

**Available Profiles:**
- `kaspad` - Layer 1 (kaspad, kaspa-miner)
- `backend` - Layer 2 (execution-layer, block-builder, viaduct)
- `frontend-w1` through `frontend-w5` - Frontend workers (rpc-provider, kaswallet)
- `all` - All services

**Actions:**

| Key | Action |
|-----|--------|
| `↑`/`↓` | Select profile |
| `Enter` or `s` | Start selected profile |
| `?` | Show help |

**Workflow Example:**

*Starting a frontend worker:*
1. Switch to Profiles screen (press `2`)
2. Navigate to `frontend-w3`
3. Press `Enter` to start
4. Switch to Services screen (press `1`) to verify containers started

---

### 3. Wallets Screen

**Purpose**: View wallet addresses and manage transactions

**Display Information:**
- Worker ID (0-4)
- Container status (Running/Stopped)
- Wallet address (from keys file)
- Balance (requires gRPC integration - shows "N/A" currently)

**Actions:**

| Key | Action |
|-----|--------|
| `↑`/`↓` | Select wallet |
| `t` | Open send transaction dialog (UI only) |
| `/` | Search/filter wallets |
| `?` | Show help |

**Understanding Wallet Data:**

- **Address**: Read from `keys/keys.kaswallet-{worker_id}.json`
- **Balance**: Requires kaswallet-daemon gRPC API (future feature)
- **Container Status**: Must be "Running" to access wallet

**Send Transaction Dialog** (UI Demo):
```
Press 't' to open dialog:

┌─────────────────────────────────────┐
│     Send KAS Transaction            │
│                                     │
│ Amount (KAS): [100.5      ]        │  ← Active field highlighted
│                                     │
│ Destination Address:                │
│ [kaspa:qqr7x...abc123]             │
│                                     │
│ [Tab] Switch field                  │
│ [Enter] Send  [Esc] Cancel         │
└─────────────────────────────────────┘

Navigation:
- Tab: Switch between Amount and Address fields
- Enter: Submit transaction (requires gRPC integration)
- Esc: Cancel dialog
```

**Note**: Full transaction functionality requires kaswallet-daemon gRPC integration (planned for v0.3.0).

---

### 4. RPC Tokens Screen

**Purpose**: View and manage RPC access tokens

**Display Information:**
- Token number (0-4)
- Access token (UUID format)
- HTTP endpoint URL
- HTTPS endpoint URL

**Actions:**

| Key | Action |
|-----|--------|
| `↑`/`↓` | Navigate token list |
| `?` | Show help |

**Token Information:**
- Tokens are read from `.env` file
- Format: `RPC_TOKEN_0`, `RPC_TOKEN_1`, etc.
- Endpoints use ports 8545 + token number

**Example Display:**
```
Token  Access Token                          HTTP Endpoint
─────────────────────────────────────────────────────────────────────
  0    5e7f294e-4c92-9aa6-61fa-e8d347d832d  http://your-domain:8545
  1    a3b2c1d0-1234-5678-90ab-cdef12345678  http://your-domain:8546
```

**Future Features** (v0.3.0+):
- Generate new tokens
- Test token endpoints
- Revoke/regenerate tokens

---

### 5. SSL Info Screen

**Purpose**: Check SSL certificate and DNS configuration

**Display Information:**
- Certificate status (Valid, Expired, Not Found)
- Expiration date
- Domain name
- DNS-01 challenge configuration
- Cloudflare API status

**Example Display:**
```
SSL Certificate Information
──────────────────────────────────────────
Status:       Valid ✓
Expires:      2025-12-31 23:59:59 UTC
Domain:       igramerlin.kasbah.xyz
Days Left:    74 days

DNS-01 Challenge Configuration
──────────────────────────────────────────
Provider:     Cloudflare
API Token:    cf_*********************abc (configured)
Email:        admin@example.com
Zone ID:      1234567890abcdef
```

**Indicators:**
- **Green "Valid ✓"**: Certificate is valid and current
- **Red "Expired ✗"**: Certificate has expired
- **Yellow "Expiring Soon"**: Less than 30 days remaining
- **Gray "Not Found"**: No certificate file

**Actions:**

| Key | Action |
|-----|--------|
| `?` | Show help |

---

### 6. Config Screen

**Purpose**: View environment configuration

**Display Information:**
- Environment variable names
- Current values
- Source: `.env` file

**Actions:**

| Key | Action |
|-----|--------|
| `↑`/`↓` | Scroll configuration |
| `/` | Search configuration keys |
| `?` | Show help |

**Search Example:**
```
Press '/' and type "RPC" to find all RPC-related settings:
- RPC_TOKEN_0
- RPC_TOKEN_1
- RPC_HTTP_PORT
- etc.
```

**Common Configuration Keys:**

| Key | Purpose |
|-----|---------|
| `DOMAIN` | Your domain name |
| `ACME_EMAIL` | Let's Encrypt email |
| `RPC_TOKEN_*` | RPC access tokens |
| `CF_API_TOKEN` | Cloudflare API token |
| `CF_ZONE_ID` | Cloudflare zone ID |

**Tips:**
- Use search (`/`) to quickly find specific settings
- Check for "N/A" or empty values (indicates missing config)
- Sensitive values (tokens, passwords) are displayed in full - be cautious when screen sharing

---

### 7. Logs Screen

**Purpose**: View real-time container logs

**Features:**
- **Real-time Updates**: Auto-scrolls to show latest logs
- **Multi-Container**: Shows logs from selected service
- **Scroll History**: Use arrow keys to scroll back
- **Auto-Scroll**: Automatically follows new log entries

**Actions:**

| Key | Action |
|-----|--------|
| `↑`/`↓` | Scroll up/down |
| `PgUp`/`PgDn` | Scroll page up/down |
| `Home` | Jump to top |
| `End` | Jump to bottom (auto-scroll) |
| `Esc` | Return to Services screen |
| `?` | Show help |

**Using the Log Viewer:**

1. From Services screen, select a container
2. Press `d` to view detailed logs
3. Logs screen opens showing last 100 lines
4. New logs append in real-time
5. Scroll up to view history
6. Press `Esc` to return

**Example Log Display:**
```
╔═══════════════════════════════════════════════════════════╗
║ Logs: viaduct                                             ║
╠═══════════════════════════════════════════════════════════╣
║ 2025-10-18T14:30:45 INFO  Starting viaduct service       ║
║ 2025-10-18T14:30:46 INFO  Connected to execution layer   ║
║ 2025-10-18T14:30:47 INFO  Listening on port 3000          ║
║ 2025-10-18T14:30:48 DEBUG Processing transaction abc123  ║
║ 2025-10-18T14:30:49 INFO  Transaction confirmed           ║
║ ...                                                        ║
╠═══════════════════════════════════════════════════════════╣
║ [Esc] Back  [↑↓] Scroll  [?] Help  [q] Quit              ║
╚═══════════════════════════════════════════════════════════╝
```

**Tips:**
- Logs update every 2 seconds
- Color coding: INFO (white), WARN (yellow), ERROR (red)
- Auto-scroll disables when you scroll up manually
- Press `End` to re-enable auto-scroll

---

## Keyboard Shortcuts

### Global Shortcuts

| Key | Action |
|-----|--------|
| `q` | Quit application |
| `Ctrl+C` | Force quit |
| `?` | Show context-sensitive help |
| `1-7` | Switch to screen 1-7 |
| `Tab` | Next screen |
| `Shift+Tab` | Previous screen |

### Navigation

| Key | Action |
|-----|--------|
| `↑` / `k` | Move up |
| `↓` / `j` | Move down |
| `Home` | Jump to top |
| `End` | Jump to bottom |
| `PgUp` | Scroll page up |
| `PgDn` | Scroll page down |

### Service Actions

| Key | Screen | Action |
|-----|--------|--------|
| `r` | Services | Restart service |
| `s` | Services | Stop service |
| `d` | Services | View detailed logs |
| `Enter` | Profiles | Start profile |
| `t` | Wallets | Send transaction dialog |

### Search & Filter

| Key | Action |
|-----|--------|
| `/` | Enter search mode |
| `Esc` | Cancel search |
| `Enter` | Apply search filter |
| `Backspace` | Delete search character |

---

## Common Workflows

### 1. Starting Your Node Infrastructure

**Step-by-step:**

```
1. Press '2' → Switch to Profiles screen
2. Navigate to 'kaspad' → Press Enter
   Wait 2-3 minutes for kaspad to sync

3. Navigate to 'backend' → Press Enter
   Wait for execution-layer, viaduct to start

4. Navigate to 'frontend-w1' → Press Enter
   Starts rpc-provider-0 and kaswallet-0

5. Press '1' → Return to Services screen
   Verify all containers show "Running" and "Healthy"
```

### 2. Monitoring Service Health

**Watch for issues:**

```
1. Press '1' → Services screen
2. Look for:
   - Red status (Unhealthy, Stopped)
   - High CPU/Memory (red percentages >80%)
   - Restarting status (service crash loop)

3. Press 'd' on problematic service → View logs
4. Check error messages
5. Press 'r' to restart if needed
```

### 3. Checking Wallet Balances

**Current workflow:**

```
1. Press '3' → Wallets screen
2. Verify containers show "Running"
3. View addresses (read from keys files)
4. Balance shows "N/A" (requires gRPC integration)

Alternative - Use CLI:
$ docker exec kaswallet-0 kaswallet-cli balance
```

### 4. Finding Configuration Values

**Using search:**

```
1. Press '6' → Config screen
2. Press '/' → Enter search mode
3. Type "DOMAIN" → See highlighted matches
4. Press Enter → Jump to first match
5. Review configuration value
```

### 5. Troubleshooting Service Errors

**Debug workflow:**

```
1. Press '1' → Services screen
2. Find failed service (red status)
3. Press 'd' → View logs
4. Scroll through logs (↑/↓)
5. Look for ERROR or WARN messages
6. Press 'Esc' → Back to Services
7. Press 'r' → Restart service
8. Watch status change
```

### 6. Monitoring System Resources

**Resource overview:**

```
1. Check header: CPU/Memory/Disk usage
2. Press '1' → Services screen
3. Sort mentally by CPU or Memory column
4. Find resource-heavy services (red = >80%)
5. Consider:
   - Is this normal for the service?
   - Is it stuck in a loop?
   - Does it need restart?
```

---

## CLI Commands

### Status Commands

```bash
# View all services
igra-cli status

# Check Docker connection
docker ps
```

### Service Management

```bash
# Start by profile
igra-cli start --profile backend
igra-cli start --profile frontend-w1

# Start specific service
igra-cli start execution-layer

# Stop services
igra-cli stop --all
igra-cli stop viaduct

# Restart service
igra-cli restart execution-layer
```

### Logs

```bash
# View recent logs
igra-cli logs viaduct

# Last 500 lines
igra-cli logs -n 500 execution-layer

# Follow logs (Ctrl+C to stop)
igra-cli logs -f viaduct
```

### Configuration

```bash
# View configuration
igra-cli config view

# Validate configuration
igra-cli config validate
```

### RPC Management

```bash
# List RPC tokens
igra-cli rpc tokens list
```

### Upgrades

```bash
# Pull latest Docker images
igra-cli upgrade --pull
```

### Diagnostics

```bash
# Generate diagnostic report
igra-cli diagnostics --report
```

---

## Advanced Features

### Search & Filter

**How it works:**

1. Available on: Services, Wallets, Config screens
2. Press `/` to activate
3. Type query (case-insensitive)
4. Results highlight in cyan as you type
5. Press `Enter` to jump to first match
6. Press `Esc` to cancel

**Search Targets:**

- **Services**: Name, status, image name
- **Wallets**: Worker ID, address
- **Config**: Key name, value

**Example:**
```
Services screen:
Press '/' → Type "rpc" → Highlights all rpc-provider containers
Press Enter → Jumps to rpc-provider-0
```

### Resource Alerts

**Color-coded warnings:**

- **Red (>80%)**: Critical - investigate immediately
- **Yellow (60-80%)**: Warning - monitor closely
- **White (<60%)**: Normal

**Common scenarios:**

| Alert | Service | Likely Cause |
|-------|---------|--------------|
| High CPU | execution-layer | Transaction processing spike |
| High Memory | kaspad | Blockchain sync in progress |
| High Disk | All | Logs or blockchain data growing |

### Log Auto-Scroll

**Behavior:**

- **Auto-scroll ON**: Stays at bottom, shows new logs
- **Auto-scroll OFF**: Manual scroll position maintained

**Controls:**

- Scroll up → Auto-scroll disabled
- Press `End` → Re-enable auto-scroll
- New logs append but don't move viewport

---

## Troubleshooting

### TUI Won't Start

**Problem**: `Error: docker-compose.yml not found`

**Solution**:
```bash
# Run from IGRA Orchestra directory
cd ~/igra-orchestra-public
igra-cli

# Or find your installation
find ~ -name "docker-compose.yml" -path "*/igra-orchestra-public/*"
```

### Services Show "N/A"

**Problem**: All services show "N/A" status

**Solution**:
```bash
# Check Docker daemon
docker ps

# Restart Docker
sudo systemctl restart docker

# Verify connection
docker version
```

### Wallet Shows No Address

**Problem**: Wallet address shows "N/A"

**Solution**:
```bash
# Check if keys file exists
ls -la keys/keys.kaswallet-0.json

# If missing, generate wallet
docker exec kaswallet-0 /app/kaswallet-create --testnet --create

# Follow prompts to create wallet
```

### High CPU/Memory Alerts

**Problem**: Container shows red (>80%) resource usage

**Investigation**:
```bash
# View detailed logs
Press 'd' on service in TUI

# Check container stats
docker stats container-name

# Check for errors
docker logs container-name --tail 100
```

**Common fixes:**
- Restart container (press `r`)
- Wait for sync to complete
- Increase Docker resource limits

### SSL Shows "Not Found"

**Problem**: SSL screen shows "Certificate not found"

**Causes**:
- Let's Encrypt not configured
- Certificate path incorrect
- First time setup

**Solution**:
```bash
# Check Traefik logs
igra-cli logs traefik -n 200

# Verify DNS-01 configuration
igra-cli config view | grep CF_

# Test DNS propagation
dig TXT _acme-challenge.yourdomain.com
```

### Search Not Working

**Problem**: Pressing `/` does nothing

**Check**:
- Are you on Services, Wallets, or Config screen?
- Search only works on these 3 screens
- Profiles, RPC Tokens, SSL Info, Logs don't support search

### Logs Not Updating

**Problem**: Logs screen frozen

**Solution**:
```bash
# Press 'End' to re-enable auto-scroll
# Or press 'Esc' and 'd' again to refresh
# Or restart TUI: press 'q' then run igra-cli again
```

---

## Tips & Best Practices

### Performance Tips

1. **Run from project directory**
   ```bash
   cd ~/igra-orchestra-public && igra-cli
   ```

2. **Use CLI for one-time tasks**
   ```bash
   # Better for scripting
   igra-cli status | grep "execution-layer"
   ```

3. **Close TUI when not monitoring**
   - TUI refreshes every 2 seconds
   - Uses minimal resources but press `q` when done

### Monitoring Best Practices

1. **Regular health checks**
   - Check Services screen 2-3 times daily
   - Watch for red/yellow alerts
   - Verify all critical services "Healthy"

2. **Log review**
   - Check logs after restarts
   - Look for ERROR messages
   - Monitor sync progress

3. **Resource tracking**
   - Note baseline CPU/Memory usage
   - Compare to current values
   - Investigate significant changes

### Security Practices

1. **Token management**
   - Don't share RPC tokens publicly
   - Rotate tokens periodically
   - Use unique tokens per service

2. **Screen sharing**
   - Be cautious showing Config screen
   - Contains sensitive tokens/keys
   - Use CLI commands instead when demoing

3. **Wallet safety**
   - Backup `keys/` directory regularly
   - Keep wallet passwords secure
   - Test recovery procedures

### Operational Tips

1. **Start services in order**
   ```
   kaspad → backend → frontend-w1 → frontend-w2, etc.
   ```

2. **Wait for health checks**
   - Don't start next profile until previous shows "Healthy"
   - kaspad: ~2-3 minutes
   - execution-layer: ~1-2 minutes
   - Others: ~30-60 seconds

3. **Use profiles for batch operations**
   - Easier than starting individual containers
   - Ensures correct dependency order
   - Consistent configuration

### Debugging Workflow

```
1. Identify problem (Services screen)
   ↓
2. View logs (press 'd')
   ↓
3. Search error messages (Logs screen)
   ↓
4. Check configuration (Config screen)
   ↓
5. Try restart (press 'r')
   ↓
6. Verify fix (Services screen)
   ↓
7. Monitor (watch for recurrence)
```

### Keyboard Efficiency

Learn these 10 keys:
- `1-7`: Screen switching
- `j/k` or `↑/↓`: Navigation
- `d`: Detailed logs
- `r`: Restart
- `/`: Search
- `?`: Help
- `q`: Quit

### Update Practices

```bash
# Pull latest images
igra-cli upgrade --pull

# Restart services to use new images
# Via TUI:
Press '1' → Select service → Press 'r'

# Via CLI:
docker compose --profile backend restart
```

---

## Getting Help

### In-App Help

Press `?` on any screen for context-sensitive help showing:
- Available keyboard shortcuts
- Screen-specific actions
- Navigation tips

### Command-Line Help

```bash
# Main help
igra-cli --help

# Command-specific help
igra-cli start --help
igra-cli logs --help
```

### Documentation

- **README.md**: Installation and overview
- **USER_GUIDE.md**: This comprehensive guide
- **IGRA Docs**: See main repository documentation

### Support Channels

- **GitHub Issues**: [https://github.com/Zorglub4242/Igra-mgt/issues](https://github.com/Zorglub4242/Igra-mgt/issues)
- **IGRA Discord**: Community support and discussions
- **Main Repository**: [https://github.com/igralabs/igra-orchestra-public](https://github.com/igralabs/igra-orchestra-public)

---

## Appendix: Service Reference

### Layer 1 (Kaspa)
- **kaspad**: Kaspa node
- **kaspa-miner**: Mining service

### Layer 2 (Backend)
- **execution-layer**: Main L2 execution
- **block-builder**: Block production
- **viaduct**: Transaction relay

### Frontend
- **rpc-provider-{0-4}**: RPC endpoints
- **kaswallet-{0-4}**: Wallet services

### Infrastructure
- **traefik**: Reverse proxy and SSL
- **node-health-check-client**: Monitoring

---

**Last Updated**: October 2025
**Version**: 0.2.0
**Author**: Merlin for IGRA Community
