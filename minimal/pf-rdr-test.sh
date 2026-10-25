#!/usr/bin/env sh
# Gateway stage 2: NAT (already verified in stage 1) + pf rdr into the
# transparent adapter. Requires a running `proxybot start` (no --gateway).
#
#   sudo sh minimal/pf-rdr-test.sh install
#   sudo sh minimal/pf-rdr-test.sh restore
set -eu
IFACE="${PROXYBOT_IFACE:-en1}"
CIDR="${PROXYBOT_CIDR:-192.168.3.0/24}"
PORT="${PROXYBOT_TRANSPARENT_PORT:-8089}"
RDR_IP="${PROXYBOT_RDR_IP:-127.0.0.1}"
ANCHOR="com.apple/proxybot-rdr"
CONF="/tmp/proxybot-rdr.conf"
STATE="/tmp/proxybot-rdr-forwarding"

usage() { echo "usage: sudo sh $0 install|restore" >&2; exit 2; }
[ $# -eq 1 ] || usage

case "$1" in
  install)
    if ! lsof -nP -iTCP:"$PORT" -sTCP:LISTEN >/dev/null 2>&1; then
      echo "no listener on 127.0.0.1:${PORT}; start proxybot (without --gateway) first" >&2
      exit 1
    fi
    cat > "$CONF" <<RULES
rdr pass on ${IFACE} inet proto tcp from any to any port {80, 443} -> ${RDR_IP} port ${PORT}
nat on ${IFACE} inet from ${CIDR} to any -> (${IFACE})
pass in quick on lo0 inet from any to any keep state
pass in quick on ${IFACE} inet from ${CIDR} to any keep state
pass out quick on ${IFACE} inet from any to any keep state
RULES
    pfctl -n -f "$CONF"
    PREV="$(sysctl -n net.inet.ip.forwarding)"
    echo "$PREV" > "$STATE"
    # Avoid duplicate rules from earlier stages / the app's own anchor.
    for anchor in com.apple/proxybot com.apple/proxybot-nat "$ANCHOR"; do
      sudo pfctl -a "$anchor" -F all 2>/dev/null || true
    done
    sudo pfctl -a "$ANCHOR" -f "$CONF"
    sudo sysctl -w net.inet.ip.forwarding=1
    sudo pfctl -e >/dev/null 2>&1 || true
    echo "Stage 2 rules loaded (anchor=$ANCHOR rdr=${RDR_IP}:${PORT})"
    echo "iPhone: manual IP 192.168.3.20, router $(ipconfig getifaddr "$IFACE" 2>/dev/null || echo '<Mac IP>'), DNS 223.5.5.5, proxy OFF."
    echo "Test http://captive.apple.com/hotspot-detect.html first (no CA needed)."
    echo "Restore: sudo sh $0 restore"
    ;;
  restore)
    for anchor in com.apple/proxybot com.apple/proxybot-nat "$ANCHOR"; do
      sudo pfctl -a "$anchor" -F all 2>/dev/null || true
    done
    if [ -f "$STATE" ]; then
      PREV="$(cat "$STATE")"
      sudo sysctl -w net.inet.ip.forwarding="$PREV" >/dev/null
      rm -f "$STATE"
    else
      sudo sysctl -w net.inet.ip.forwarding=0 >/dev/null
    fi
    rm -f "$CONF"
    echo "gateway test rules removed; IP forwarding restored"
    ;;
  *) usage ;;
esac
