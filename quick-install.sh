#!/usr/bin/env bash
# IGRA CLI Quick Installer
# One-line install: curl -fsSL https://raw.githubusercontent.com/Zorglub4242/Igra-mgt/main/quick-install.sh | bash

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# GitHub repository
REPO="Zorglub4242/Igra-mgt"
INSTALL_PATH="/usr/local/bin/igra-cli"

echo -e "${BLUE}🚀 IGRA CLI Web UI Quick Installer${NC}"
echo "=================================="
echo ""

# Detect OS and architecture
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$ARCH" in
    x86_64) ARCH="x86_64" ;;
    aarch64|arm64) ARCH="arm64" ;;
    *)
        echo -e "${RED}Error: Unsupported architecture: $ARCH${NC}"
        echo "Supported: x86_64, arm64"
        exit 1
        ;;
esac

case "$OS" in
    linux)
        PLATFORM="linux"
        FILE_EXT="tar.gz"
        ;;
    darwin)
        PLATFORM="macos"
        FILE_EXT="tar.gz"
        ;;
    *)
        echo -e "${RED}Error: Unsupported OS: $OS${NC}"
        echo "Supported: Linux, macOS"
        exit 1
        ;;
esac

echo "Detected: $OS $ARCH"

# Get latest release version
echo "Fetching latest release..."
LATEST_VERSION=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name"' | sed -E 's/.*"([^"]+)".*/\1/')

if [ -z "$LATEST_VERSION" ]; then
    echo -e "${RED}Error: Could not fetch latest version${NC}"
    exit 1
fi

echo -e "Latest version: ${GREEN}$LATEST_VERSION${NC}"
echo ""

# Download URL
DOWNLOAD_FILENAME="igra-cli-${PLATFORM}-${ARCH}.tar.gz"
DOWNLOAD_URL="https://github.com/$REPO/releases/download/$LATEST_VERSION/$DOWNLOAD_FILENAME"

# Check if binary already installed
if [ -f "$INSTALL_PATH" ]; then
    CURRENT_VERSION=$($INSTALL_PATH --version 2>/dev/null | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' || echo "unknown")
    echo -e "${YELLOW}igra-cli is already installed (version: $CURRENT_VERSION)${NC}"
    read -p "Overwrite with $LATEST_VERSION? [y/N]: " -n 1 -r OVERWRITE
    echo
    if [[ ! $OVERWRITE =~ ^[Yy]$ ]]; then
        echo "Installation cancelled"
        exit 0
    fi
fi

# Download and install
echo "Downloading $DOWNLOAD_FILENAME..."
TMP_DIR=$(mktemp -d)
cd "$TMP_DIR"

if ! curl -fsSL -o "$DOWNLOAD_FILENAME" "$DOWNLOAD_URL"; then
    echo -e "${RED}Error: Download failed${NC}"
    echo "URL: $DOWNLOAD_URL"
    rm -rf "$TMP_DIR"
    exit 1
fi

echo "Extracting..."
tar -xzf "$DOWNLOAD_FILENAME"

echo "Installing to $INSTALL_PATH..."
if [ "$EUID" -ne 0 ]; then
    sudo mv igra-cli "$INSTALL_PATH"
    sudo chmod +x "$INSTALL_PATH"
else
    mv igra-cli "$INSTALL_PATH"
    chmod +x "$INSTALL_PATH"
fi

rm -rf "$TMP_DIR"

echo -e "${GREEN}✓ Binary installed${NC}"
echo ""

# Web UI Configuration
echo -e "${BLUE}Web UI Configuration${NC}"
echo "===================="
echo ""

read -p "Port [3000]: " PORT
PORT=${PORT:-3000}

read -p "Host [0.0.0.0]: " HOST
HOST=${HOST:-0.0.0.0}

read -p "Enable CORS? [Y/n]: " CORS
CORS=${CORS:-Y}

echo -n "IGRA_WEB_TOKEN (required): "
read -s TOKEN
echo ""

if [ -z "$TOKEN" ]; then
    echo -e "${RED}Error: IGRA_WEB_TOKEN is required${NC}"
    exit 1
fi

read -p "Service user [$(whoami)]: " USER
USER=${USER:-$(whoami)}

read -p "Install as systemd service? [Y/n]: " INSTALL_SERVICE
INSTALL_SERVICE=${INSTALL_SERVICE:-Y}

echo ""

if [[ $INSTALL_SERVICE =~ ^[Yy]$ ]]; then
    echo "Installing systemd service..."

    CORS_FLAG=""
    if [[ $CORS =~ ^[Yy]$ ]]; then
        CORS_FLAG="--cors"
    fi

    # Run install-service with token piped
    if [ "$EUID" -ne 0 ]; then
        echo "$TOKEN" | sudo -S igra-cli install-service --port "$PORT" --host "$HOST" $CORS_FLAG --user "$USER" 2>/dev/null || {
            echo -e "${YELLOW}Note: Running with sudo${NC}"
            sudo bash -c "echo '$TOKEN' | igra-cli install-service --port $PORT --host $HOST $CORS_FLAG --user $USER"
        }
    else
        echo "$TOKEN" | igra-cli install-service --port "$PORT" --host "$HOST" $CORS_FLAG --user "$USER"
    fi

    echo -e "${GREEN}✓ Systemd service installed and started${NC}"
else
    echo -e "${YELLOW}To run manually:${NC}"
    CORS_CMD=""
    if [[ $CORS =~ ^[Yy]$ ]]; then
        CORS_CMD=" --cors"
    fi
    echo "  IGRA_WEB_TOKEN=\"$TOKEN\" igra-cli serve --host $HOST --port $PORT$CORS_CMD"
fi

echo ""
echo -e "${GREEN}✅ Installation Complete!${NC}"
echo ""

# Try to get public IP
PUBLIC_IP=$(hostname -I 2>/dev/null | awk '{print $1}' || echo "your-server-ip")
echo -e "🌐 Web UI: ${BLUE}http://$PUBLIC_IP:$PORT${NC}"
echo -e "📚 Documentation: ${BLUE}https://github.com/$REPO/blob/main/docs/web-ui.md${NC}"
echo -e "💬 Support: ${BLUE}https://github.com/$REPO/issues${NC}"
echo ""

if [[ $INSTALL_SERVICE =~ ^[Yy]$ ]]; then
    echo "Service commands:"
    echo "  sudo systemctl status igra-web-ui"
    echo "  sudo systemctl restart igra-web-ui"
    echo "  sudo journalctl -u igra-web-ui -f"
fi
