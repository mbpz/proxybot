#!/bin/bash
# ProxyBot Local Development Runner
# Usage: ./scripts/run.sh [dev|build|check]
#
# dev    - Run in development mode with hot reload (default)
# build  - Build release binary
# check  - Type check and lint without building

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_ROOT"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[OK]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Parse command
MODE="${1:-dev}"

check_dependencies() {
    log_info "Checking dependencies..."

    # Check Rust
    if ! command -v cargo &> /dev/null; then
        log_error "Rust not found. Install from https://rustup.rs"
        exit 1
    fi
    log_success "Rust $(cargo --version | cut -d' ' -f2)"

    # Check Node
    if ! command -v node &> /dev/null; then
        log_error "Node not found. Install from https://nodejs.org"
        exit 1
    fi
    log_success "Node $(node --version)"

    # Check pnpm
    if ! command -v pnpm &> /dev/null; then
        log_warn "pnpm not found, installing..."
        npm install -g pnpm
    fi
    log_success "pnpm $(pnpm --version)"

    # Check Tauri CLI
    if ! command -v tauri &> /dev/null; then
        log_warn "Tauri CLI not found, installing..."
        pnpm add -D @tauri-apps/cli
    fi
    log_success "Tauri CLI available"

    # Install frontend deps
    if [ ! -d "node_modules" ]; then
        log_info "Installing frontend dependencies..."
        pnpm install
    fi
    log_success "Frontend dependencies ready"
}

run_dev() {
    log_info "Starting ProxyBot in development mode..."
    log_info "  - Frontend: http://localhost:1420"
    log_info "  - Rust backend on port 8080"
    log_info ""
    log_info "Press Ctrl+C to stop"
    log_info ""

    # Start Vite dev server in background (from project root)
    log_info "Starting Vite dev server..."
    pnpm vite --port 1420 &
    VITE_PID=$!

    # Start Rust backend in background (from src-tauri dir)
    log_info "Starting Rust backend..."
    (cd src-tauri && cargo run --bin proxybot) &
    CARGO_PID=$!

    # Wait for either process to exit
    trap "kill $VITE_PID $CARGO_PID 2>/dev/null; exit" INT TERM
    wait $CARGO_PID
}

run_build() {
    log_info "Building ProxyBot release binary..."
    pnpm tauri build
    log_success "Build complete!"
    log_info "Binary: src-tauri/target/release/proxybot"
}

run_check() {
    log_info "Running checks..."
    log_info ""

    log_info "1. TypeScript type check..."
    if pnpm exec tsc --noEmit 2>/dev/null; then
        log_success "TypeScript: OK"
    else
        log_error "TypeScript: FAILED"
    fi

    log_info "2. Rust cargo check..."
    if cargo check --message-format=short 2>&1 | grep -v "^$" | head -20; then
        log_success "Cargo check: OK"
    else
        log_error "Cargo check: FAILED"
    fi

    log_info "3. Rust clippy..."
    if cargo clippy -- -D warnings 2>&1 | grep -v "^$" | head -20; then
        log_success "Clippy: OK"
    else
        log_error "Clippy: FAILED"
    fi
}

# Main
case "$MODE" in
    dev)
        check_dependencies
        run_dev
        ;;
    build)
        check_dependencies
        run_build
        ;;
    check)
        run_check
        ;;
    *)
        echo "Usage: $0 [dev|build|check]"
        exit 1
        ;;
esac