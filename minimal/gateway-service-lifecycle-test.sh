#!/bin/sh
# Service-mode lifecycle regression:
# install LaunchDaemon -> run the sleep/Wi-Fi lifecycle matrix -> uninstall.
# Interactive only where the lifecycle matrix already requires sleep/network
# switches. Always attempts stop/uninstall on exit.
set -u

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="${PROXYBOT_BIN:-$(command -v proxybot 2>/dev/null || true)}"
[ -n "$BIN" ] || BIN="$ROOT/minimal/target/release/proxybot"
[ -x "$BIN" ] || { echo "proxybot binary not found; run sh minimal/dev-install.sh" >&2; exit 2; }
[ "$(id -u)" = "0" ] || { echo "run with sudo: sudo sh minimal/gateway-service-lifecycle-test.sh" >&2; exit 2; }
SINGBOX="${PROXYBOT_SINGBOX_BIN:-/tmp/singbox-src/sing-box}"
[ -x "$SINGBOX" ] || { echo "sing-box not found: $SINGBOX (set PROXYBOT_SINGBOX_BIN)" >&2; exit 2; }
curl -fsS -m 3 http://127.0.0.1:8899/api/setup >/dev/null 2>&1 || {
  echo "ProxyBot is not running on 127.0.0.1:8899; start it as your user first." >&2; exit 2;
}

LOG_DIR="/tmp/proxybot-service-lifecycle-$(date +%Y%m%d-%H%M%S)"
mkdir -p "$LOG_DIR"
PIPE="$LOG_DIR/.pipe"; TEE_PID=""
mkfifo "$PIPE"; tee -a "$LOG_DIR/run.log" < "$PIPE" & TEE_PID=$!
exec > "$PIPE" 2>&1

cleanup() {
  echo; echo "===== service cleanup ====="
  "$BIN" gateway stop >/dev/null 2>&1 || true
  "$BIN" gateway uninstall-service >/dev/null 2>&1 || true
  [ -n "$TEE_PID" ] && kill "$TEE_PID" >/dev/null 2>&1 || true
  rm -f "$PIPE"
  echo "logs: $LOG_DIR"
}
trap cleanup EXIT INT TERM

echo "===== install service (no autostart) ====="
"$BIN" gateway stop >/dev/null 2>&1 || true
"$BIN" gateway uninstall-service >/dev/null 2>&1 || true
"$BIN" gateway install-service --singbox "$SINGBOX" || { echo "install-service failed"; exit 1; }
sleep 1
"$BIN" gateway status
echo "service runner: $(pgrep -f 'proxybot gateway service' | wc -l | tr -d ' ') process(es)"
echo

echo "===== lifecycle matrix under launchd ====="
# The lifecycle runner starts/stops via `proxybot gateway start/stop`; with the
# service installed, those commands delegate to launchd. The environment marker
# makes it fail if that delegation did not happen.
PROXYBOT_EXPECT_SERVICE=1 sh "$ROOT/minimal/gateway-lifecycle-test.sh"
MATRIX=$?

echo
echo "===== uninstall and final cleanup ====="
"$BIN" gateway uninstall-service || true
sleep 1
"$BIN" gateway status
if [ -e /Library/LaunchDaemons/com.proxybot.gateway.plist ]; then
  echo "FAIL: plist remains"; MATRIX=1
else
  echo "PASS: plist removed"
fi
if pgrep -f 'proxybot gateway service|sing-box run -c /tmp/proxybot-gateway' >/dev/null 2>&1; then
  echo "FAIL: gateway process remains"; MATRIX=1
else
  echo "PASS: no gateway process"
fi
if ifconfig utun6 >/dev/null 2>&1; then
  echo "FAIL: utun6 remains"; MATRIX=1
else
  echo "PASS: no utun6"
fi
if [ "$(sysctl -n net.inet.ip.forwarding 2>/dev/null)" = "0" ]; then
  echo "PASS: forwarding 0"
else
  echo "FAIL: forwarding not 0"; MATRIX=1
fi
curl -sS -m 8 -o /dev/null -w 'Mac https: %{http_code}\n' https://example.com || MATRIX=1
echo
echo "SERVICE-MODE RESULT: $([ "$MATRIX" -eq 0 ] && echo PASS || echo FAIL)"
exit "$MATRIX"
