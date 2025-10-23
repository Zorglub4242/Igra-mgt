#!/usr/bin/env bash
# Build script for igra-cli
# Compiles both TUI and Web UI components

set -e

echo "Building IGRA CLI..."

# Check if cargo is installed
if ! command -v cargo &> /dev/null; then
    echo "Error: Cargo is not installed. Please install Rust from https://rustup.rs/"
    exit 1
fi

# Check for Web UI sources
if [ -d "igra-web-ui" ]; then
    echo "📦 Building Web UI..."

    # Check for npm
    if ! command -v npm &> /dev/null; then
        echo "⚠️  Warning: npm not found. Skipping Web UI build."
        echo "   Install Node.js and npm to build Web UI."
    else
        cd igra-web-ui
        npm install
        npm run build
        cd ..
        echo "✓ Web UI built successfully"
    fi
fi

# Build Rust binary
echo "🔨 Building Rust binary..."

# Build with server feature if Web UI assets exist
if [ -d "igra-web-ui/dist" ]; then
    echo "   Building with Web UI (--features server)..."
    cargo build --release --features server
else
    echo "   Building TUI only (no Web UI)..."
    cargo build --release
fi

echo ""
echo "✅ Build complete!"
echo ""
echo "Binary location: target/release/igra-cli"
echo "Binary size: $(du -h target/release/igra-cli | cut -f1)"
echo ""
echo "To install, run: ./install.sh"
echo "Or copy manually: sudo cp target/release/igra-cli /usr/local/bin/"
