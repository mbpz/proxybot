#!/bin/sh
# One-command launchd lifecycle test (root, no phone/network switching).
# Installs the LaunchDaemon, tests explicit start/stop/crash/no-auto-restart,
# uninstalls, and leaves networking clean.
set -u

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="${PROXYBOT_BIN:-$(command -v proxybot 2>/dev/null || true)}"
[ -n "$BIN" ] || BIN="$ROOT/minimal/target/release/proxybot"
[ -x "$BIN" ] || { echo "proxybot binary not found; run sh minimal/dev-install.sh" >&2; exit 2; }
[ "$(id -u)" = "0" ] || { echo "run with sudo: sudo sh minimal/gateway-launchd-test.sh" >&2; exit 2; }
curl -fsS -m 3 http://127.0.0.1:8899/api/setup >/dev/null 2>&1 || {
  echo "ProxyBot is not running on 127.0.0.1:8899; start it as your user first." >&2; exit 2;
}

LABEL="com.proxybot.gateway"
PLIST="/Library/LaunchDaemons/$LABEL.plist"
LOG_DIR="/tmp/proxybot-launchd-test-$(date +%Y%m%d-%H%M%S)"
mkdir -p "$LOG_DIR"
PIPE="$LOG_DIR/.pipe"; TEE_PID=""
mkfifo "$PIPE"; tee -a "$LOG_DIR/run.log" < "$PIPE" & TEE_PID=$!
exec > "$PIPE" 2>&1
PASS=0; FAIL=0
pass(){ PASS=$((PASS+1)); echo "PASS: $*"; }
fail(){ FAIL=$((FAIL+1)); echo "FAIL: $*"; }
banner(){ echo; echo "===== $* ====="; }
cleanup(){
  echo; echo "===== cleanup ====="
  "$BIN" gateway stop >/dev/null 2>&1 || true
  launchctl bootout "system/$LABEL" >/dev/null 2>&1 || true
  rm -f "$PLIST"
  [ -n "$TEE_PID" ] && kill "$TEE_PID" >/dev/null 2>&1 || true
  rm -f "$PIPE"
  echo "logs: $LOG_DIR"
  echo "PASS=$PASS FAIL=$FAIL"
}
trap cleanup EXIT INT TERM

status_text(){ "$BIN" gateway status 2>&1; }
is_stopped(){ case "$(status_text)" in *"gateway: stopped"*) return 0;; *) return 1;; esac; }
is_running(){ case "$(status_text)" in *"gateway: running"*) return 0;; *) return 1;; esac; }
has_stale(){ case "$(status_text)" in *"gateway: stale"*) return 0;; *) return 1;; esac; }
has_exited(){ case "$(status_text)" in *"gateway: exited"*) return 0;; *) return 1;; esac; }

banner "preconditions"
"$BIN" gateway stop >/dev/null 2>&1 || true
sleep 1
is_stopped && pass "baseline stopped" || fail "baseline not stopped"
[ ! -e "$PLIST" ] || { echo "existing LaunchDaemon found; removing for test"; launchctl bootout "system/$LABEL" >/dev/null 2>&1 || true; rm -f "$PLIST"; }

banner "install / no autostart"
if "$BIN" gateway install-service --singbox /nonexistent >/tmp/launchd-bad-install.log 2>&1; then
  fail "bad sing-box path install unexpectedly succeeded"
else pass "bad sing-box path rejected"; fi
[ ! -e "$PLIST" ] && pass "bad install left no plist" || fail "bad install left plist"
"$BIN" gateway stop >/dev/null 2>&1 || true
"$BIN" gateway install-service --singbox "${PROXYBOT_SINGBOX_BIN:-/tmp/singbox-src/sing-box}" || { fail "install-service"; exit 1; }
[ -e "$PLIST" ] && pass "plist installed" || fail "plist missing"
sleep 1
is_stopped && pass "install did not autostart" || fail "install autostarted gateway"
launchctl print "system/$LABEL" >/dev/null 2>&1 && pass "launchd job loaded" || fail "launchd job not loaded"

banner "explicit start / repeat"
"$BIN" gateway start || fail "explicit start"
sleep 1
is_running && pass "start reached running" || fail "start not running"
if "$BIN" gateway start >/tmp/launchd-repeat-start.log 2>&1; then
  fail "repeated start unexpectedly succeeded"
else pass "repeated start rejected"; fi
COUNT=$(pgrep -f 'proxybot gateway service' | wc -l | tr -d ' ')
[ "$COUNT" = "1" ] && pass "exactly one supervisor" || fail "supervisor count=$COUNT"

banner "crash / no autorestart"
SB_PID=$(python3 -c 'import json;print(json.load(open("/tmp/proxybot-gateway/state.json"))["singbox_pid"])' 2>/dev/null || true)
if [ -n "$SB_PID" ]; then
  kill -9 "$SB_PID" || true
  sleep 2
  has_exited && pass "sing-box kill -> exited" || fail "sing-box kill not exited"
  sleep 3
  is_running && fail "launchd auto-restarted after crash" || pass "no automatic restart after crash"
  "$BIN" gateway start || fail "explicit restart"
  sleep 1
  is_running && pass "explicit restart after crash" || fail "explicit restart failed"
else
  fail "could not read sing-box pid"
fi

banner "stop / no restart"
"$BIN" gateway stop || fail "stop"
sleep 2
is_stopped && pass "stop -> stopped" || fail "stop not stopped"
[ "$(sysctl -n net.inet.ip.forwarding 2>/dev/null)" = "0" ] && pass "forwarding 0" || fail "forwarding not 0"
if ifconfig utun6 >/dev/null 2>&1; then fail "utun6 remains"; else pass "utun6 removed"; fi
sleep 3
is_stopped && pass "no restart after stop" || fail "state changed after stop"

banner "uninstall"
"$BIN" gateway uninstall-service || fail "uninstall-service"
[ ! -e "$PLIST" ] && pass "plist removed" || fail "plist remains"
launchctl print "system/$LABEL" >/dev/null 2>&1 && fail "launchd job still loaded" || pass "launchd job unloaded"
sleep 1
is_stopped && pass "stopped after uninstall" || fail "not stopped after uninstall"
[ "$(sysctl -n net.inet.ip.forwarding 2>/dev/null)" = "0" ] && pass "forwarding clean after uninstall" || fail "forwarding dirty"
if ifconfig utun6 >/dev/null 2>&1; then fail "utun6 remains after uninstall"; else pass "no utun6 after uninstall"; fi
curl -sS -m 8 -o /dev/null -w 'mac=%{http_code}\n' https://example.com 2>&1 | grep -q 'mac=200' && pass "Mac internet" || fail "Mac internet"

echo; echo "TEST COMPLETE PASS=$PASS FAIL=$FAIL"
[ "$FAIL" -eq 0 ]
