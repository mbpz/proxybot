#!/usr/bin/env sh
# Remove the installed ProxyBot CLI and (when root) the Gateway service.
# User data/CA is kept unless --purge is passed.
set -eu

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
DEFAULT_PREFIX="$(cd "$SCRIPT_DIR/../.." && pwd)"
PREFIX="${PROXYBOT_PREFIX:-$DEFAULT_PREFIX}"
BIN="$PREFIX/proxybot"
PURGE=0
[ "${1:-}" = "--purge" ] && PURGE=1

if [ "$(id -u)" = "0" ]; then
  if [ -x "$BIN" ]; then
    "$BIN" gateway uninstall-service >/dev/null 2>&1 || true
    "$BIN" gateway stop >/dev/null 2>&1 || true
  fi
else
  if [ -e /Library/LaunchDaemons/com.proxybot.gateway.plist ] || pgrep -f 'proxybot gateway service|proxybot gateway supervise' >/dev/null 2>&1; then
    echo "Gateway cleanup needs root; run:"
    echo "  sudo \"$BIN\" gateway uninstall-service"
    echo "  sudo \"$BIN\" gateway stop"
  fi
fi

rm -f "$BIN"
rm -rf "$PREFIX/share/proxybot"
echo "removed $BIN"
if [ "$PURGE" = "1" ]; then
  DATA="$HOME/Library/Application Support/ProxyBot"
  rm -rf "$DATA"
  echo "removed user data $DATA"
else
  echo "kept user data; use --purge to remove it"
fi
