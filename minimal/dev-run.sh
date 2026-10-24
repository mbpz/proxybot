#!/usr/bin/env sh
# Local development loop: build the CLI from this checkout and run it.
# No GitHub release/download involved. Ctrl+C stops the binary as usual.
#
#   sh minimal/dev-run.sh                       # same as `proxybot start`
#   sh minimal/dev-run.sh start --gateway --block-quic
#   sh minimal/dev-run.sh list
#   PROXYBOT_SKIP_BUILD=1 sh minimal/dev-run.sh # skip the build step
set -eu
cd "$(dirname "$0")/.."

# Reuse the repository-local Cargo cache when present so local builds are
# offline and instant; a fresh checkout can fall back to ~/.cargo.
if [ -z "${CARGO_HOME:-}" ] && [ -d "$PWD/.cache-cargo" ]; then
  export CARGO_HOME="$PWD/.cache-cargo"
fi
if [ "${PROXYBOT_SKIP_BUILD:-0}" != "1" ]; then
  # Offline by default: debugging should not block on crates.io.
  cargo build --release --offline --manifest-path minimal/Cargo.toml \
    || cargo build --release --manifest-path minimal/Cargo.toml
fi
exec ./minimal/target/release/proxybot "$@"
