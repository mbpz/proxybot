#!/bin/bash
set -e

echo "=== Building Yew GUI ==="

cd src-tauri
wasm-pack build --target web --out-dir pkg

echo "=== Build complete ==="