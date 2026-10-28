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
cp LICENSE minimal/dist/LICENSE
cp minimal/THIRD_PARTY.md minimal/dist/THIRD_PARTY.md
cp minimal/uninstall.sh minimal/dist/uninstall.sh
ASSET="proxybot-min-${TARGET}.tar.gz"
tar -czf "minimal/dist/${ASSET}" -C minimal/dist proxybot LICENSE THIRD_PARTY.md uninstall.sh
( cd minimal/dist && shasum -a 256 "${ASSET}" > "${ASSET}.sha256" )
echo "built minimal/dist/${ASSET}"
