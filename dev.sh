#!/usr/bin/env bash
# Development server launcher for IGRA CLI
# Starts both Rust backend and React frontend with hot-reload

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$SCRIPT_DIR"

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${BLUE}🚀 Starting IGRA CLI Development Environment${NC}"
echo ""

# Check if cargo-watch is installed
if ! command -v cargo-watch &> /dev/null; then
    echo -e "${RED}❌ Error: cargo-watch is not installed${NC}"
    echo "Install it with: cargo install cargo-watch"
    exit 1
fi

# Check if npm is installed
if ! command -v npm &> /dev/null; then
    echo -e "${RED}❌ Error: npm is not installed${NC}"
    echo "Install Node.js and npm to run the development frontend"
    exit 1
fi

# Ensure npm dependencies are installed
if [ ! -d "igra-web-ui/node_modules" ]; then
    echo -e "${YELLOW}📦 Installing npm dependencies...${NC}"
    cd igra-web-ui
    npm install
    cd ..
fi

# Create log directory
mkdir -p logs

# Function to cleanup background processes on exit
cleanup() {
    echo ""
    echo -e "${YELLOW}🛑 Stopping development servers...${NC}"
    if [ ! -z "$BACKEND_PID" ]; then
        kill $BACKEND_PID 2>/dev/null || true
    fi
    if [ ! -z "$FRONTEND_PID" ]; then
        kill $FRONTEND_PID 2>/dev/null || true
    fi
    # Kill any remaining cargo-watch processes
    pkill -f "cargo-watch.*igra-cli" 2>/dev/null || true
    # Kill any remaining vite processes
    pkill -f "vite.*igra-web-ui" 2>/dev/null || true
    echo -e "${GREEN}✅ Development servers stopped${NC}"
    exit 0
}

trap cleanup SIGINT SIGTERM EXIT

# Get local IP address for display
LOCAL_IP=$(ip addr show | grep "inet " | grep -v "127.0.0.1" | head -1 | awk '{print $2}' | cut -d/ -f1)

echo -e "${GREEN}📋 Development Configuration:${NC}"
echo "  Backend (Rust):  http://localhost:8787"
echo "  Frontend (Vite): http://localhost:5173"
echo "  LAN Access:      http://${LOCAL_IP}:5173"
echo ""
echo -e "${YELLOW}Press Ctrl+C to stop both servers${NC}"
echo ""

# Start Rust backend with cargo-watch
echo -e "${BLUE}🔨 Starting Rust backend (port 8787)...${NC}"
cargo watch -q -x 'run --features server -- serve --host 0.0.0.0 --port 8787' > logs/backend-dev.log 2>&1 &
BACKEND_PID=$!

# Wait a moment for backend to start
sleep 2

# Start Vite frontend
echo -e "${BLUE}⚛️  Starting React frontend (port 5173)...${NC}"
cd igra-web-ui
npm run dev > ../logs/frontend-dev.log 2>&1 &
FRONTEND_PID=$!
cd ..

echo ""
echo -e "${GREEN}✅ Development servers started!${NC}"
echo ""
echo -e "${BLUE}View logs:${NC}"
echo "  Backend:  tail -f logs/backend-dev.log"
echo "  Frontend: tail -f logs/frontend-dev.log"
echo ""
echo -e "${YELLOW}Waiting for servers to initialize...${NC}"
echo ""

# Keep script running and show log output
tail -f logs/backend-dev.log logs/frontend-dev.log
