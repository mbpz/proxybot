#!/usr/bin/env sh
# Build a local release archive for the minimal CLI (macOS host only).
set -eu
cd "$(dirname "$0")"
cargo build --release --locked
TARGET="$(rustc -vV | sed -n 's/^host: //p')"
mkdir -p dist
cp target/release/proxybot dist/proxybot
ASSET="proxybot-min-${TARGET}.tar.gz"
tar -czf "dist/${ASSET}" -C dist proxybot
shasum -a 256 "dist/${ASSET}" > "dist/${ASSET}.sha256"
echo "built minimal/dist/${ASSET}"
