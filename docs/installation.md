# Installation Guide

This guide covers how to install `igra-cli` using pre-built binaries or by building from source.

## Prerequisites

- **Docker** 23.0+ with Docker Compose V2
- **IGRA Orchestra** repository cloned
- **Rust** 1.70+ (only required if building from source)

## Option 1: Binary Release (Recommended)

Download pre-built binaries from [GitHub Releases](https://github.com/Zorglub4242/Igra-mgt/releases):

### Linux (x86_64)

```bash
# Download latest release
wget https://github.com/Zorglub4242/Igra-mgt/releases/latest/download/igra-cli-linux-x86_64.tar.gz

# Extract
tar -xzf igra-cli-linux-x86_64.tar.gz

# Install
sudo mv igra-cli /usr/local/bin/
sudo chmod +x /usr/local/bin/igra-cli
```

### Windows (x86_64)

```powershell
# Download from releases page
# https://github.com/Zorglub4242/Igra-mgt/releases/latest/download/igra-cli-windows-x86_64.zip

# Extract and add to PATH
# Move igra-cli.exe to a directory in your PATH
```

### macOS (Intel/Apple Silicon)

```bash
# Download from releases page
wget https://github.com/Zorglub4242/Igra-mgt/releases/latest/download/igra-cli-macos-universal.tar.gz

# Extract
tar -xzf igra-cli-macos-universal.tar.gz

# Install
sudo mv igra-cli /usr/local/bin/
sudo chmod +x /usr/local/bin/igra-cli
```

## Option 2: Build from Source

```bash
# Clone the repository
git clone https://github.com/Zorglub4242/Igra-mgt.git
cd Igra-mgt

# Build and install
./build.sh
sudo ./install.sh
```

## Option 3: Manual Build

```bash
# Build release binary
cargo build --release

# Install to system path
sudo cp target/release/igra-cli /usr/local/bin/
```

## Verify Installation

```bash
igra-cli --version
```

You should see output like:
```
igra-cli 0.10.0
```

## Web UI Installation (Optional)

The Web UI provides a browser-based interface for remote management. It requires the binary to be built with the `server` feature.

### Quick Start

**Run temporarily:**
```bash
IGRA_WEB_TOKEN=your-secret-token igra-cli serve --host 0.0.0.0 --port 3000 --cors
```

Access at: `http://your-server:3000` and login with your token.

### Production: Install as Systemd Service

For production deployments, install as a systemd service:

```bash
sudo igra-cli install-service [OPTIONS]
```

**Options:**
- `--port <PORT>` - Port number (default: 3000)
- `--host <HOST>` - Bind address (default: 0.0.0.0)
- `--cors` - Enable CORS for cross-origin requests
- `--user <USER>` - Service user (default: current user)

**Example:**
```bash
sudo igra-cli install-service --port 3000 --host 0.0.0.0 --cors
# You will be prompted to enter your IGRA_WEB_TOKEN
```

This command will:
1. Prompt for your `IGRA_WEB_TOKEN` (required for API authentication)
2. Create `/etc/systemd/system/igra-web-ui.service`
3. Reload systemd daemon
4. Enable service to start on boot
5. Start the service immediately

**Manage the service:**
```bash
# Check status
sudo systemctl status igra-web-ui

# Stop service
sudo systemctl stop igra-web-ui

# Restart service
sudo systemctl restart igra-web-ui

# View logs
sudo journalctl -u igra-web-ui -f

# Disable auto-start
sudo systemctl disable igra-web-ui
```

**Note:** The `install-service` command is only available if the binary was built with the `server` feature. Pre-built releases from GitHub include this feature.

## Next Steps

- See [Web UI Guide](web-ui.md) to learn about web management interface features
- See [TUI Guide](tui-guide.md) to use the terminal user interface
- See [Configuration Guide](configuration.md) for environment setup
