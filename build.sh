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
BUILD_WINDOWS=false
DEPLOY=false

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
        --windows)
            BUILD_WINDOWS=true
            shift
            ;;
        --deploy)
            DEPLOY=true
            shift
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 [--clean] [--debug] [--windows] [--deploy]"
            echo ""
            echo "Options:"
            echo "  --clean     Clean build artifacts before building"
            echo "  --debug     Build in debug mode (faster compile, larger binary)"
            echo "  --windows   Cross-compile for Windows (x86_64-pc-windows-gnu)"
            echo "  --deploy    Stop running service, install binary, and restart"
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

# Setup target and binary paths
if [ "$BUILD_WINDOWS" = true ]; then
    CARGO_TARGET="x86_64-pc-windows-gnu"
    BINARY_NAME="igra-cli.exe"
    echo "   Target: Windows (${CARGO_TARGET})"

    # Check if target is installed
    if ! rustup target list --installed | grep -q "$CARGO_TARGET"; then
        echo "   Installing Windows target..."
        rustup target add "$CARGO_TARGET"
    fi

    # Check for mingw-w64 cross-compiler
    if ! command -v x86_64-w64-mingw32-gcc &> /dev/null; then
        echo "⚠️  Warning: mingw-w64 not found. Install with:"
        echo "   Ubuntu/Debian: sudo apt install mingw-w64"
        echo "   Fedora: sudo dnf install mingw64-gcc"
        echo "   Arch: sudo pacman -S mingw-w64-gcc"
        echo ""
        echo "Attempting build anyway..."
    fi
else
    CARGO_TARGET=""
    BINARY_NAME="igra-cli"
    echo "   Target: Native ($(uname -m)-$(uname -s | tr '[:upper:]' '[:lower:]'))"
fi

# CRITICAL: Clean specific build artifacts to force rust-embed to re-process assets
# This prevents rust-embed from using cached assets when dist files change
if [ "$BUILD_WEB_UI" = true ] && [ "$BUILD_TYPE" = "release" ]; then
    echo "   Cleaning rust-embed cache..."
    if [ -n "$CARGO_TARGET" ]; then
        rm -f target/${CARGO_TARGET}/release/.fingerprint/*igra-cli*/lib-igra_cli* 2>/dev/null || true
        rm -f target/${CARGO_TARGET}/release/deps/libigra_cli* 2>/dev/null || true
    else
        rm -f target/release/.fingerprint/*igra-cli*/lib-igra_cli* 2>/dev/null || true
        rm -f target/release/deps/libigra_cli* 2>/dev/null || true
    fi
fi

# Build with server feature if Web UI assets exist
if [ "$BUILD_WEB_UI" = true ]; then
    echo "   Building with Web UI (--features server)..."

    if [ "$BUILD_TYPE" = "debug" ]; then
        if [ -n "$CARGO_TARGET" ]; then
            cargo build --target "$CARGO_TARGET" --features server
            BINARY_PATH="target/${CARGO_TARGET}/debug/${BINARY_NAME}"
        else
            cargo build --features server
            BINARY_PATH="target/debug/${BINARY_NAME}"
        fi
    else
        if [ -n "$CARGO_TARGET" ]; then
            cargo build --target "$CARGO_TARGET" --release --features server
            BINARY_PATH="target/${CARGO_TARGET}/release/${BINARY_NAME}"
        else
            cargo build --release --features server
            BINARY_PATH="target/release/${BINARY_NAME}"
        fi
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
        if [ -n "$CARGO_TARGET" ]; then
            cargo build --target "$CARGO_TARGET"
            BINARY_PATH="target/${CARGO_TARGET}/debug/${BINARY_NAME}"
        else
            cargo build
            BINARY_PATH="target/debug/${BINARY_NAME}"
        fi
    else
        if [ -n "$CARGO_TARGET" ]; then
            cargo build --target "$CARGO_TARGET" --release
            BINARY_PATH="target/${CARGO_TARGET}/release/${BINARY_NAME}"
        else
            cargo build --release
            BINARY_PATH="target/release/${BINARY_NAME}"
        fi
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
    if [ "$BUILD_WINDOWS" = true ]; then
        echo "Windows executable built successfully!"
        echo ""
        echo "To create a release package:"
        echo "  tar -czf igra-cli-windows-x86_64.tar.gz -C target/${CARGO_TARGET}/release igra-cli.exe"
        echo ""
        echo "Or create a ZIP file:"
        echo "  cd target/${CARGO_TARGET}/release && zip ../../../igra-cli-windows-x86_64.zip igra-cli.exe"
        echo ""
        echo "⚠️  Note: Windows binary requires Microsoft Visual C++ Redistributable"
        echo "    Download: https://aka.ms/vs/17/release/vc_redist.x64.exe"
    else
        if [ "$DEPLOY" = true ]; then
            echo "🚀 Deploying to /usr/local/bin/..."
            echo ""

            # Stop running igra-cli serve process
            echo "   Stopping running igra-cli service..."
            RUNNING_PIDS=$(ps aux | grep "[i]gra-cli serve" | awk '{print $2}')
            if [ -n "$RUNNING_PIDS" ]; then
                echo "$RUNNING_PIDS" | xargs sudo kill
                echo "   ✓ Stopped PIDs: $RUNNING_PIDS"
                sleep 1
            else
                echo "   No running igra-cli service found"
            fi

            # Install binary
            echo "   Installing binary to /usr/local/bin/..."
            sudo cp "$BINARY_PATH" /usr/local/bin/
            sudo chmod +x /usr/local/bin/igra-cli
            echo "   ✓ Binary installed"

            # Restart service
            echo "   Restarting igra-cli service..."
            if [ -f /tmp/start-igra-cli.sh ]; then
                sudo nohup /tmp/start-igra-cli.sh > /tmp/igra-cli-server.log 2>&1 &
                sleep 2
                NEW_PID=$(ps aux | grep "[i]gra-cli serve" | awk '{print $2}')
                if [ -n "$NEW_PID" ]; then
                    echo "   ✓ Service restarted (PID: $NEW_PID)"
                else
                    echo "   ⚠️  Warning: Service may not have started. Check /tmp/igra-cli-server.log"
                fi
            else
                echo "   ⚠️  Warning: /tmp/start-igra-cli.sh not found. Start manually with:"
                echo "      sudo nohup igra-cli serve --host 0.0.0.0 --port 8787 > /tmp/igra-cli-server.log 2>&1 &"
            fi

            echo ""
            echo "✅ Deployment complete!"
        else
            echo "To install, run: ./install.sh"
            echo "Or copy manually: sudo cp $BINARY_PATH /usr/local/bin/"
            echo ""
            echo "To build and deploy in one step: ./build.sh --deploy"
            echo ""
            echo "To create a release package:"
            echo "  tar -czf igra-cli-linux-x86_64.tar.gz -C target/release igra-cli"
        fi
    fi
else
    echo "Debug build complete. Use --release for production builds."
fi
echo ""
