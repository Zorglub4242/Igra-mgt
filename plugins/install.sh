#!/bin/bash
#
# Plugin Installation Script for igra-cli
#
# This script installs metric plugin configuration files to system locations.
# Plugins can be installed system-wide or per-user.
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGINS_SOURCE="$SCRIPT_DIR"

# Default to system-wide installation
INSTALL_MODE="${1:-system}"

case "$INSTALL_MODE" in
    system)
        INSTALL_DIR="/etc/l2-mgt/plugins"
        echo "Installing plugins to system location: $INSTALL_DIR"
        echo "Note: This requires root/sudo privileges"
        ;;
    user)
        INSTALL_DIR="$HOME/.config/l2-mgt/plugins"
        echo "Installing plugins to user location: $INSTALL_DIR"
        ;;
    *)
        echo "Usage: $0 [system|user]"
        echo ""
        echo "  system - Install to /etc/l2-mgt/plugins (default, requires sudo)"
        echo "  user   - Install to ~/.config/l2-mgt/plugins"
        exit 1
        ;;
esac

# Create directory if it doesn't exist
if [ "$INSTALL_MODE" = "system" ]; then
    sudo mkdir -p "$INSTALL_DIR"
else
    mkdir -p "$INSTALL_DIR"
fi

# Count TOML files
PLUGIN_COUNT=$(find "$PLUGINS_SOURCE" -maxdepth 1 -name "*.toml" | wc -l)

if [ "$PLUGIN_COUNT" -eq 0 ]; then
    echo "Error: No .toml plugin files found in $PLUGINS_SOURCE"
    exit 1
fi

echo "Found $PLUGIN_COUNT plugin(s) to install"
echo ""

# Copy all .toml files
for plugin_file in "$PLUGINS_SOURCE"/*.toml; do
    if [ -f "$plugin_file" ]; then
        plugin_name=$(basename "$plugin_file")
        echo "  Installing: $plugin_name"

        if [ "$INSTALL_MODE" = "system" ]; then
            sudo cp "$plugin_file" "$INSTALL_DIR/"
            sudo chmod 644 "$INSTALL_DIR/$plugin_name"
        else
            cp "$plugin_file" "$INSTALL_DIR/"
            chmod 644 "$INSTALL_DIR/$plugin_name"
        fi
    fi
done

echo ""
echo "Installation complete!"
echo "Installed to: $INSTALL_DIR"
echo ""
echo "Verify with: ls -la $INSTALL_DIR"
