#!/usr/bin/env sh
# Remove the installed ProxyBot CLI and, when run as root, the Gateway service.
# User data/CA files are kept unless --purge is passed. CA trust is never
# removed automatically; remove it in Keychain/iOS Settings separately.
set -eu

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
DEFAULT_PREFIX="$(cd "$SCRIPT_DIR/../.." && pwd)"
PREFIX="${PROXYBOT_PREFIX:-$DEFAULT_PREFIX}"
BIN="$PREFIX/proxybot"
PURGE=0
[ "${1:-}" = "--purge" ] && PURGE=1
PLIST="/Library/LaunchDaemons/com.proxybot.gateway.plist"
SUDO_HOME=""
if [ -n "${SUDO_USER:-}" ] && [ -d "/Users/${SUDO_USER}" ]; then
  SUDO_HOME="/Users/${SUDO_USER}"
fi
USER_HOME="${SUDO_HOME:-${HOME:-}}"
DATA="${USER_HOME}/Library/Application Support/ProxyBot"

if [ "$(id -u)" = "0" ]; then
  if [ -x "$BIN" ]; then
    "$BIN" gateway uninstall-service >/dev/null 2>&1 || true
    "$BIN" gateway stop >/dev/null 2>&1 || true
  fi
  # Fallback if the service exists but the binary is already gone.
  if [ -e "$PLIST" ]; then
    launchctl bootout system/com.proxybot.gateway >/dev/null 2>&1 || true
    rm -f "$PLIST"
    echo "removed leftover LaunchDaemon plist"
  fi
else
  if [ -e "$PLIST" ] || pgrep -f 'proxybot gateway service|proxybot gateway supervise' >/dev/null 2>&1; then
    echo "Gateway cleanup needs root; run:" >&2
    echo "  sudo \"$BIN\" gateway uninstall-service" >&2
    echo "  sudo \"$BIN\" gateway stop" >&2
    echo "Nothing was removed. Re-run this uninstaller after gateway cleanup." >&2
    exit 1
  fi
  if pgrep -f 'proxybot start' >/dev/null 2>&1; then
    echo "ProxyBot is still running. Stop it (Ctrl+C) before uninstalling." >&2
    exit 1
  fi
fi

rm -f "$BIN"
rm -rf "$PREFIX/share/proxybot"
echo "removed $BIN"
echo "removed $PREFIX/share/proxybot"

if [ "$PURGE" = "1" ]; then
  if [ -n "$DATA" ]; then
    rm -rf "$DATA"
    echo "removed user data $DATA"
  fi
else
  echo "kept user data: $DATA"
  echo "  CA file is under that directory; use --purge only if you intend to delete it."
fi
echo "CA trust is not removed automatically; remove 'ProxyBot Local CA' from Keychain"
echo "or delete the iOS profile/trusted root separately."
