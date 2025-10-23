#!/usr/bin/env bash
# Build script for igra-cli
# Compiles both TUI and Web UI components
#
# IMPORTANT: This script handles the correct build order to ensure
# rust-embed picks up the latest Web UI assets. Always use this script
# for building releases to avoid caching issues.

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$SCRIPT_DIR"

echo "Building IGRA CLI..."
echo ""

# Check if cargo is installed
if ! command -v cargo &> /dev/null; then
    echo "❌ Error: Cargo is not installed. Please install Rust from https://rustup.rs/"
    exit 1
fi

# Parse command line arguments
CLEAN=false
BUILD_TYPE="release"

while [[ $# -gt 0 ]]; do
    case $1 in
        --clean)
            CLEAN=true
            shift
            ;;
        --debug)
            BUILD_TYPE="debug"
            shift
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 [--clean] [--debug]"
            exit 1
            ;;
    esac
done

# Clean build artifacts if requested
if [ "$CLEAN" = true ]; then
    echo "🧹 Cleaning build artifacts..."
    cargo clean
    if [ -d "igra-web-ui/dist" ]; then
        rm -rf igra-web-ui/dist
        echo "   Removed Web UI dist folder"
    fi
    if [ -d "igra-web-ui/node_modules" ]; then
        rm -rf igra-web-ui/node_modules
        echo "   Removed node_modules"
    fi
    echo ""
fi

# Check for Web UI sources
BUILD_WEB_UI=false
if [ -d "igra-web-ui" ]; then
    echo "📦 Building Web UI..."

    # Check for npm
    if ! command -v npm &> /dev/null; then
        echo "⚠️  Warning: npm not found. Skipping Web UI build."
        echo "   Install Node.js and npm to build Web UI."
        echo ""
    else
        cd igra-web-ui

        # Install dependencies
        echo "   Installing npm dependencies..."
        npm install --silent

        # Build React app
        echo "   Building React app..."
        npm run build
        cd ..

        # Verify dist files were created
        if [ -d "igra-web-ui/dist" ] && [ -f "igra-web-ui/dist/index.html" ]; then
            echo "✓ Web UI built successfully"

            # Show what JS file was generated (for debugging)
            JS_FILE=$(ls igra-web-ui/dist/assets/index-*.js 2>/dev/null | head -1)
            if [ -n "$JS_FILE" ]; then
                echo "   Generated: $(basename $JS_FILE)"
            fi

            BUILD_WEB_UI=true
        else
            echo "❌ Error: Web UI build failed - dist folder not created"
            exit 1
        fi
        echo ""
    fi
fi

# Build Rust binary
echo "🔨 Building Rust binary..."

# CRITICAL: Clean specific build artifacts to force rust-embed to re-process assets
# This prevents rust-embed from using cached assets when dist files change
if [ "$BUILD_WEB_UI" = true ] && [ "$BUILD_TYPE" = "release" ]; then
    echo "   Cleaning rust-embed cache..."
    rm -f target/release/.fingerprint/*igra-cli*/lib-igra_cli* 2>/dev/null || true
    rm -f target/release/deps/libigra_cli* 2>/dev/null || true
fi

# Build with server feature if Web UI assets exist
if [ "$BUILD_WEB_UI" = true ]; then
    echo "   Building with Web UI (--features server)..."

    if [ "$BUILD_TYPE" = "debug" ]; then
        cargo build --features server
        BINARY_PATH="target/debug/igra-cli"
    else
        cargo build --release --features server
        BINARY_PATH="target/release/igra-cli"
    fi

    # Verify the correct assets were embedded
    echo ""
    echo "🔍 Verifying embedded assets..."
    if [ -f "$BINARY_PATH" ]; then
        # Extract JS filename from binary
        EMBEDDED_JS=$(strings "$BINARY_PATH" | grep -o 'assets/index-[^"]*\.js' | head -1 | sed 's/assets\///')
        EXPECTED_JS=$(ls igra-web-ui/dist/assets/index-*.js 2>/dev/null | head -1 | xargs basename)

        if [ "$EMBEDDED_JS" = "$EXPECTED_JS" ]; then
            echo "✓ Correct assets embedded: $EMBEDDED_JS"
        else
            echo "⚠️  Warning: Asset mismatch detected!"
            echo "   Expected: $EXPECTED_JS"
            echo "   Embedded: $EMBEDDED_JS"
            echo "   This may indicate a caching issue. Try --clean flag."
        fi
    fi
else
    echo "   Building TUI only (no Web UI)..."

    if [ "$BUILD_TYPE" = "debug" ]; then
        cargo build
        BINARY_PATH="target/debug/igra-cli"
    else
        cargo build --release
        BINARY_PATH="target/release/igra-cli"
    fi
fi

echo ""
echo "✅ Build complete!"
echo ""
echo "Binary location: $BINARY_PATH"
if [ -f "$BINARY_PATH" ]; then
    echo "Binary size: $(du -h "$BINARY_PATH" | cut -f1)"
fi
echo ""

if [ "$BUILD_TYPE" = "release" ]; then
    echo "To install, run: ./install.sh"
    echo "Or copy manually: sudo cp $BINARY_PATH /usr/local/bin/"
    echo ""
    echo "To create a release package:"
    echo "  tar -czf igra-cli-linux-x86_64.tar.gz -C target/release igra-cli"
else
    echo "Debug build complete. Use --release for production builds."
fi
echo ""
