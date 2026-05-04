#!/bin/bash
set -e

echo "=== Building Yew GUI ==="

# Build WASM with wasm-pack
cd src-tauri
wasm-pack build --target web --out-dir pkg

# Build Tauri app
cargo build --bin proxybot-gui --release

echo "=== Build complete ==="