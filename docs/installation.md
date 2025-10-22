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

## Next Steps

- See [Web UI Guide](web-ui.md) to launch the web management interface
- See [TUI Guide](tui-guide.md) to use the terminal user interface
- See [Configuration Guide](configuration.md) for environment setup
