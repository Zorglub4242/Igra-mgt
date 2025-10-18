#!/usr/bin/env bash
# Build script for igra-cli
# Compiles the CLI tool for the target platform

set -e

echo "Building IGRA CLI..."

# Check if cargo is installed
if ! command -v cargo &> /dev/null; then
    echo "Error: Cargo is not installed. Please install Rust from https://rustup.rs/"
    exit 1
fi

# Build in release mode
cargo build --release

echo ""
echo "✓ Build complete!"
echo ""
echo "Binary location: target/release/igra-cli"
echo ""
echo "To install, run: ./install.sh"
echo "Or copy manually: sudo cp target/release/igra-cli /usr/local/bin/"
