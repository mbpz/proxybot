#!/usr/bin/env bash
# One-command lifecycle test runner for Gateway Beta.
#
# It automates the reproducible parts (status/TUN/forwarding/Mac checks, sleep,
# Wi-Fi power off/on, child kill, manual-stop-before-sleep) and pauses only for
# the two things a script cannot do safely: choosing a different Wi-Fi network
# and confirming the Mac may sleep.
#
#   sudo sh minimal/gateway-lifecycle-test.sh
#
# The script always tries to stop the gateway and restore Wi-Fi power on exit.
set -u

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="${PROXYBOT_BIN:-$(command -v proxybot 2>/dev/null || true)}"
[ -n "$BIN" ] || BIN="$ROOT/minimal/target/release/proxybot"
[ -x "$BIN" ] || { echo "proxybot binary not found; run sh minimal/dev-install.sh first" >&2; exit 2; }

LOG_DIR="/tmp/proxybot-lifecycle-$(date +%Y%m%d-%H%M%S)"
mkdir -p "$LOG_DIR"
LOG_PIPE="$LOG_DIR/.logpipe"
TEE_PID=""
mkfifo "$LOG_PIPE"
tee -a "$LOG_DIR/run.log" < "$LOG_PIPE" &
TEE_PID=$!
exec > "$LOG_PIPE" 2>&1

WIFI_IF="$(networksetup -listallhardwareports 2>/dev/null | awk '/Hardware Port: (Wi-Fi|AirPort)/{found=1;next} found&&/Device:/{print $2;exit}')"
PASS=0; FAIL=0; SKIP=0

pass() { PASS=$((PASS+1)); echo "PASS: $*"; }
fail() { FAIL=$((FAIL+1)); echo "FAIL: $*"; }
skip() { SKIP=$((SKIP+1)); echo "SKIP: $*"; }
banner() { echo; echo "========== $* =========="; }

cleanup() {
  banner "cleanup"
  "$BIN" gateway stop >/dev/null 2>&1 || true
  [ -n "$WIFI_IF" ] && networksetup -setairportpower "$WIFI_IF" on >/dev/null 2>&1 || true
  echo "logs: $LOG_DIR"
  echo "summary: PASS=$PASS FAIL=$FAIL SKIP=$SKIP"
  if [ -n "$TEE_PID" ]; then
    kill "$TEE_PID" >/dev/null 2>&1 || true
  fi
  rm -f "$LOG_PIPE"
}
trap cleanup EXIT INT TERM

require() {
  echo
  read -r -p "$1 [Enter to continue, Ctrl+C to abort] " _
}

capture() {
  local name="$1"
  {
    echo "### $name $(date -u +%FT%TZ)"
    "$BIN" gateway status
    echo "--- route"
    route -n get default 2>&1 | egrep 'interface|gateway' || true
    echo "--- utun6"
    ifconfig utun6 2>&1 || true
    echo "--- forwarding"
    sysctl net.inet.ip.forwarding 2>&1 || true
    echo "--- mac HTTPS"
    curl -sS -m 8 -o /dev/null -w 'mac=%{http_code}\n' https://example.com 2>&1 || true
    echo "--- pf anchor"
    pfctl -a com.apple/proxybot-tun -sr 2>&1 || true
  } | tee "$LOG_DIR/$name.txt"
}

has_stale() { grep -q '^gateway: stale' "$LOG_DIR/$1.txt"; }
has_running() { grep -q '^gateway: running' "$LOG_DIR/$1.txt"; }
has_stopped() { grep -q '^gateway: stopped' "$LOG_DIR/$1.txt"; }

banner "preconditions"
if ! curl -fsS -m 3 http://127.0.0.1:8899/api/setup >/dev/null 2>&1; then
  echo "ProxyBot is not listening on 127.0.0.1:8899; start it as your user first." >&2
  exit 2
fi
"$BIN" gateway stop >/dev/null 2>&1 || true
capture baseline-stopped
[ "$(sysctl -n net.inet.ip.forwarding 2>/dev/null)" = "0" ] && pass "forwarding starts at 0" || fail "forwarding not 0 before test"

banner "A. start gateway"
"$BIN" gateway start || { fail "gateway start"; exit 1; }
sleep 2
capture a-started
has_running a-started && pass "start reaches running" || fail "start not running"

banner "B. sleep / wake while running"
require "The Mac will sleep for the next test."
capture b-before-sleep
pmset sleepnow || true
sleep 5
capture b-after-wake
if has_running b-after-wake; then pass "awake: running"; else fail "awake: not running"; fi
grep -q 'health tun=up' "$LOG_DIR/b-after-wake.txt" && pass "awake: TUN up" || fail "awake: TUN missing"
grep -q 'mac=200' "$LOG_DIR/b-after-wake.txt" && pass "awake: Mac internet" || fail "awake: Mac internet"

banner "C. Wi-Fi off / on"
if [ -z "$WIFI_IF" ]; then
  skip "Wi-Fi interface not detected"
else
  networksetup -setairportpower "$WIFI_IF" off || true
  sleep 5
  capture c-wifi-off
  has_stale c-wifi-off && pass "Wi-Fi off: stale" || fail "Wi-Fi off: not stale"
  networksetup -setairportpower "$WIFI_IF" on || true
  sleep 10
  capture c-wifi-on
  if has_running c-wifi-on; then pass "Wi-Fi on: running"; else fail "Wi-Fi on: not running"; fi
  grep -q 'mac=200' "$LOG_DIR/c-wifi-on.txt" && pass "Wi-Fi on: Mac internet" || fail "Wi-Fi on: Mac internet"
fi

banner "D. different Wi-Fi / CIDR"
NET_BEFORE="$(route -n get default 2>/dev/null | egrep 'interface|gateway' || true) $(ipconfig getifaddr "$WIFI_IF" 2>/dev/null || true)"
echo "Current network:"
echo "$NET_BEFORE"
echo "Switch the Mac to a different Wi-Fi/hotspot with a DIFFERENT interface/IP/CIDR."
echo "Do NOT stop the gateway before this capture."
SWITCHED=0
for attempt in 1 2 3; do
  require "Switch to the other network now (attempt $attempt/3)."
  capture "d-switched-$attempt"
  NET_AFTER="$(route -n get default 2>/dev/null | egrep 'interface|gateway' || true) $(ipconfig getifaddr "$WIFI_IF" 2>/dev/null || true)"
  if [ "$NET_BEFORE" != "$NET_AFTER" ]; then
    cp "$LOG_DIR/d-switched-$attempt.txt" "$LOG_DIR/d-switched.txt"
    SWITCHED=1
    break
  fi
  echo "Network did not change (still: $NET_AFTER). The hotspot test was not executed."
done
if [ "$SWITCHED" -eq 1 ]; then
  echo "Network after switch: $NET_AFTER"
  has_stale d-switched && pass "network change: stale" || fail "network change: not stale"
  if grep -q 'network unknown' "$LOG_DIR/d-switched.txt"; then pass "network change: unknown/down detected"; fi
  echo "Old pf anchor (should not be treated as valid by status):"
  pfctl -a com.apple/proxybot-tun -sr 2>&1 | head -20 || true
else
  fail "network did not change after 3 attempts; cross-network test not executed"
fi
require "Switch back to the original Wi-Fi, then press Enter."
sleep 5
capture d-restored

banner "E. manual stop then sleep/wake"
"$BIN" gateway stop || fail "gateway stop"
sleep 2
capture e-stopped
has_stopped e-stopped && pass "manual stop -> stopped" || fail "manual stop -> not stopped"
if ifconfig utun6 >/dev/null 2>&1; then fail "utun6 remains"; else pass "utun6 removed"; fi
[ "$(sysctl -n net.inet.ip.forwarding 2>/dev/null)" = "0" ] && pass "forwarding restored" || fail "forwarding not restored"
require "The Mac will sleep again; after wake this phase verifies no auto-start."
pmset sleepnow || true
sleep 5
capture e-after-wake
has_stopped e-after-wake && pass "awake after manual stop: still stopped" || fail "awake after manual stop: changed state"

banner "F. kill sing-box / recovery"
"$BIN" gateway start || fail "restart gateway"
sleep 2
SB_PID="$(python3 -c 'import json; print(json.load(open("/tmp/proxybot-gateway/state.json"))["singbox_pid"])' 2>/dev/null || true)"
if [ -n "$SB_PID" ]; then
  kill -9 "$SB_PID" || true
  sleep 2
  capture f-killed
  grep -q 'gateway: exited' "$LOG_DIR/f-killed.txt" && pass "kill -9: exited" || fail "kill -9: not exited"
  grep -q 'exit signal 9' "$LOG_DIR/f-killed.txt" && pass "kill -9: signal recorded" || fail "kill -9: signal not recorded"
  grep -q 'recovery success' "$LOG_DIR/f-killed.txt" && pass "kill -9: recovery success" || fail "kill -9: recovery failed"
else
  skip "could not read sing-box pid"
fi
"$BIN" gateway stop >/dev/null 2>&1 || true

banner "summary"
echo "logs: $LOG_DIR"
echo "PASS=$PASS FAIL=$FAIL SKIP=$SKIP"
echo "Manual checks still required: iPhone browsing after wake/reconnect, and after switching"
echo "a different network use a fresh gateway start rather than the old state."
[ "$FAIL" -eq 0 ]
