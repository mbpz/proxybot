#!/usr/bin/env sh
# Build the CLI locally and install it to ~/.local/bin (or PROXYBOT_PREFIX).
# Intended for fast local debugging; no GitHub download involved.
#
#   sh minimal/dev-install.sh
#   PROXYBOT_PREFIX=/usr/local/bin sh minimal/dev-install.sh
set -eu
cd "$(dirname "$0")/.."

# Reuse the repository-local Cargo cache when present so local builds are
# offline and instant; a fresh checkout can fall back to ~/.cargo.
if [ -z "${CARGO_HOME:-}" ] && [ -d "$PWD/.cache-cargo" ]; then
  export CARGO_HOME="$PWD/.cache-cargo"
fi
cargo build --release --offline --manifest-path minimal/Cargo.toml \
  || cargo build --release --manifest-path minimal/Cargo.toml
PREFIX="${PROXYBOT_PREFIX:-$HOME/.local/bin}"
mkdir -p "$PREFIX"
install -m 0755 minimal/target/release/proxybot "$PREFIX/proxybot"
echo "Installed $("$PREFIX/proxybot" --version) to $PREFIX/proxybot"
