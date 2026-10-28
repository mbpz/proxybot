#!/bin/sh
# Release-candidate smoke test for a clean Mac. Uses only the archive installer
# path and a temporary prefix/data directory; does not touch system CA trust or
# Gateway/launchd state.
#
#   sh minimal/rc-smoke-test.sh [archive.tar.gz]
#
# Default archive: minimal/dist/proxybot-min-$(uname -m mapped).tar.gz
set -u

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
ARCHIVE="${1:-}"
if [ -z "$ARCHIVE" ]; then
  case "$(uname -m)" in
    arm64) TARGET=aarch64-apple-darwin ;;
    x86_64) TARGET=x86_64-apple-darwin ;;
    *) echo "unsupported arch"; exit 2 ;;
  esac
  ARCHIVE="$ROOT/minimal/dist/proxybot-min-${TARGET}.tar.gz"
fi
[ -f "$ARCHIVE" ] || { echo "archive not found: $ARCHIVE" >&2; exit 2; }
[ "$(uname -s)" = "Darwin" ] || { echo "macOS required" >&2; exit 2; }

TMP="$(mktemp -d /tmp/proxybot-rc.XXXXXX)"
PREFIX="$TMP/prefix"
DATA="$TMP/data"
PORT=18088; TPORT=18089; WPORT=18899; CAPORT=18090
BIN="$PREFIX/proxybot"
PID=""
KEEP_TMP="${KEEP_TMP:-0}"
PASS=0; FAIL=0
pass(){ PASS=$((PASS+1)); echo "PASS: $*"; }
fail(){ FAIL=$((FAIL+1)); echo "FAIL: $*"; }
cleanup(){
  [ -n "$PID" ] && kill "$PID" >/dev/null 2>&1 || true
  [ -n "$PID" ] && wait "$PID" >/dev/null 2>&1 || true
  if [ "$KEEP_TMP" != "1" ]; then rm -rf "$TMP"; fi
}
trap cleanup EXIT INT TERM

echo "macOS:   $(sw_vers -productVersion 2>/dev/null || echo unknown) ($(uname -m))"
echo "archive: $ARCHIVE"
echo "tmp:     $TMP"

SHA_FILE="$ARCHIVE.sha256"
[ -f "$SHA_FILE" ] || { fail "checksum file missing: $SHA_FILE"; exit 1; }
EXPECTED="$(cut -d ' ' -f1 "$SHA_FILE")"
ACTUAL="$(shasum -a 256 "$ARCHIVE" | cut -d ' ' -f1)"
[ -n "$EXPECTED" ] || { fail "checksum file is empty"; exit 1; }
[ "$EXPECTED" = "$ACTUAL" ] || { fail "checksum mismatch (expected $EXPECTED actual $ACTUAL)"; exit 1; }
pass "checksum file verified ($ACTUAL)"

tar -tzf "$ARCHIVE" | grep -qx proxybot && pass "archive has proxybot" || fail "archive missing proxybot"
tar -tzf "$ARCHIVE" | grep -qx LICENSE && pass "archive has LICENSE" || fail "archive missing LICENSE"
tar -tzf "$ARCHIVE" | grep -qx THIRD_PARTY.md && pass "archive has THIRD_PARTY" || fail "archive missing THIRD_PARTY"
tar -tzf "$ARCHIVE" | grep -qx uninstall.sh && pass "archive has uninstall.sh" || fail "archive missing uninstall.sh"
if tar -tzf "$ARCHIVE" | grep -qi sing-box; then fail "archive contains sing-box"; else pass "no sing-box in archive"; fi

if [ -f "$SCRIPT_DIR/minimal/install.sh" ]; then
  INSTALL_SH="$SCRIPT_DIR/minimal/install.sh"
elif [ -f "$SCRIPT_DIR/install.sh" ]; then
  INSTALL_SH="$SCRIPT_DIR/install.sh"
elif [ -f "$ROOT/minimal/install.sh" ]; then
  INSTALL_SH="$ROOT/minimal/install.sh"
else
  INSTALL_SH="$ROOT/install.sh"
fi
[ -f "$INSTALL_SH" ] || { fail "install.sh not found (copy it next to this script)"; exit 1; }
PROXYBOT_LOCAL_ARCHIVE="$ARCHIVE" PROXYBOT_PREFIX="$PREFIX" sh "$INSTALL_SH" >"$TMP/install.log" 2>&1 \
  && pass "install.sh" || { fail "install.sh"; cat "$TMP/install.log"; exit 1; }
[ -x "$BIN" ] && pass "installed binary" || fail "installed binary missing"
[ -f "$PREFIX/share/proxybot/LICENSE" ] && pass "installed LICENSE" || fail "installed LICENSE missing"
[ "$("$BIN" --version)" = "proxybot 0.2.0" ] && pass "version 0.2.0" || fail "wrong version: $("$BIN" --version)"

"$BIN" start --no-open --data-dir "$DATA" --proxy-port "$PORT" --transparent-port "$TPORT" --web-port "$WPORT" --ca-port "$CAPORT" >"$TMP/start.log" 2>&1 &
PID=$!
ok=0
for _ in 1 2 3 4 5 6 7 8 9 10; do
  if curl -fsS -m 1 "http://127.0.0.1:$WPORT/api/setup" >/dev/null 2>&1; then ok=1; break; fi
  sleep 1
done
[ "$ok" = "1" ] && pass "fresh data start" || { fail "fresh data start"; tail "$TMP/start.log"; }

HTTP_CODE=""
for _ in 1 2 3; do
  HTTP_CODE="$(curl -sS -m 10 -x "http://127.0.0.1:$PORT" -o /dev/null -w '%{http_code}' http://example.com 2>/dev/null || true)"
  [ "$HTTP_CODE" = "200" ] && break
  sleep 1
done
[ "$HTTP_CODE" = "200" ] && pass "HTTP proxy 200" || fail "HTTP proxy code=$HTTP_CODE"
HTTPS_CODE=""
for _ in 1 2 3; do
  HTTPS_CODE="$(curl -sS -m 15 -x "http://127.0.0.1:$PORT" --cacert "$DATA/ca.pem" -o /dev/null -w '%{http_code}' https://example.com 2>/dev/null || true)"
  [ "$HTTPS_CODE" = "200" ] && break
  sleep 1
done
[ "$HTTPS_CODE" = "200" ] && pass "HTTPS MITM with generated CA" || fail "HTTPS code=$HTTPS_CODE"
sleep 1
curl -sS -m 3 "http://127.0.0.1:$WPORT/api/requests?limit=10" | grep -q '"host":"example.com"' \
  && pass "request persisted" || fail "request not persisted"

kill "$PID" >/dev/null 2>&1 || true
wait "$PID" >/dev/null 2>&1 || true
PID=""
sleep 1
if lsof -nP -iTCP:"$PORT" -sTCP:LISTEN >/dev/null 2>&1; then fail "proxy port still listening"; else pass "proxy stopped"; fi

"$PREFIX/share/proxybot/uninstall.sh" </dev/null >"$TMP/uninstall.log" 2>&1
[ ! -e "$BIN" ] && pass "binary removed" || fail "binary remains"
[ ! -e "$PREFIX/share/proxybot/LICENSE" ] && pass "license removed" || fail "license remains"
[ -d "$DATA" ] && pass "user data kept" || echo "WARN: data dir removed"
echo
echo "SMOKE RESULT: $([ "$FAIL" -eq 0 ] && echo PASS || echo FAIL) (PASS=$PASS FAIL=$FAIL)"
if [ "$KEEP_TMP" = "1" ]; then echo "tmp kept: $TMP"; else echo "tmp cleaned (KEEP_TMP=1 to retain)"; fi
[ "$FAIL" -eq 0 ]
