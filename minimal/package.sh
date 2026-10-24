#!/usr/bin/env sh
# Build a local release archive for the minimal CLI (macOS host only).
set -eu
cd "$(dirname "$0")/.."
if [ -z "${CARGO_HOME:-}" ] && [ -d "$PWD/.cache-cargo" ]; then
  export CARGO_HOME="$PWD/.cache-cargo"
fi
cargo build --release --locked --manifest-path minimal/Cargo.toml \
  || cargo build --release --locked --offline --manifest-path minimal/Cargo.toml
TARGET="$(rustc -vV | sed -n 's/^host: //p')"
mkdir -p minimal/dist
cp minimal/target/release/proxybot minimal/dist/proxybot
ASSET="proxybot-min-${TARGET}.tar.gz"
tar -czf "minimal/dist/${ASSET}" -C minimal/dist proxybot
( cd minimal/dist && shasum -a 256 "${ASSET}" > "${ASSET}.sha256" )
echo "built minimal/dist/${ASSET}"
