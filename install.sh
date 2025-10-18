#!/usr/bin/env bash
# Installation script for igra-cli
# Installs the binary to /usr/local/bin

set -e

BINARY="target/release/igra-cli"
INSTALL_PATH="/usr/local/bin/igra-cli"

# Check if binary exists
if [ ! -f "$BINARY" ]; then
    echo "Error: Binary not found at $BINARY"
    echo "Please run ./build.sh first"
    exit 1
fi

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo "Installing igra-cli to $INSTALL_PATH (requires sudo)..."
    sudo cp "$BINARY" "$INSTALL_PATH"
    sudo chmod +x "$INSTALL_PATH"
else
    echo "Installing igra-cli to $INSTALL_PATH..."
    cp "$BINARY" "$INSTALL_PATH"
    chmod +x "$INSTALL_PATH"
fi

echo ""
echo "✓ Installation complete!"
echo ""
echo "Run 'igra-cli' to start the interactive interface"
echo "Run 'igra-cli --help' to see available commands"
