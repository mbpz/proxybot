#!/usr/bin/env sh
# Remove the installed ProxyBot CLI and, when run as root, the Gateway service.
# User data/CA files and trusted CA deletion are prompted by the binary; pass
# --purge/--yes for non-interactive runs.
set -eu

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
DEFAULT_PREFIX="$(cd "$SCRIPT_DIR/../.." && pwd)"
PREFIX="${PROXYBOT_PREFIX:-$DEFAULT_PREFIX}"
BIN="$PREFIX/proxybot"

if [ -x "$BIN" ]; then
  exec "$BIN" uninstall "$@"
fi

# Fallback when the binary is already gone.
rm -f "$BIN"
rm -rf "$PREFIX/share/proxybot"
echo "removed $BIN"
echo "removed $PREFIX/share/proxybot"
echo "user data was not removed; remove it manually if desired."
