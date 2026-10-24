#!/usr/bin/env sh
# Gateway stage 1: verify same-interface IPv4 forwarding + NAT only.
# No rdr, no transparent proxy, no application involved.
#
#   sudo sh minimal/pf-nat-test.sh install
#   sudo sh minimal/pf-nat-test.sh restore
#
# Tests the most uncertain assumption from the gateway plan: can macOS forward
# and NAT traffic from a LAN device back out the same Wi-Fi interface?
set -eu
IFACE="${PROXYBOT_IFACE:-en1}"
CIDR="${PROXYBOT_CIDR:-192.168.3.0/24}"
ANCHOR="com.apple/proxybot-nat"
CONF="/tmp/proxybot-nat.conf"
STATE="/tmp/proxybot-nat-forwarding"

usage() { echo "usage: sudo sh $0 install|restore" >&2; exit 2; }
[ $# -eq 1 ] || usage

case "$1" in
  install)
    cat > "$CONF" <<RULES
nat on ${IFACE} inet from ${CIDR} to any -> (${IFACE})
pass in quick on ${IFACE} inet from ${CIDR} to any keep state
pass out quick on ${IFACE} inet from any to any keep state
RULES
    pfctl -n -f "$CONF"
    PREV="$(sysctl -n net.inet.ip.forwarding)"
    echo "$PREV" > "$STATE"
    sudo pfctl -a "$ANCHOR" -F all 2>/dev/null || true
    sudo pfctl -a "$ANCHOR" -f "$CONF"
    sudo sysctl -w net.inet.ip.forwarding=1
    sudo pfctl -e >/dev/null 2>&1 || true
    echo "NAT-only rules loaded (anchor=$ANCHOR iface=$IFACE cidr=$CIDR)"
    echo "Previously: net.inet.ip.forwarding=$PREV"
    echo
    echo "iPhone: manual IP 192.168.3.20, router $(ipconfig getifaddr "$IFACE" 2>/dev/null || echo '<Mac IP>'), DNS 223.5.5.5, HTTP proxy OFF, cellular OFF."
    echo "Test http://captive.apple.com/hotspot-detect.html then https://example.com."
    echo "Restore afterwards: sudo sh $0 restore"
    ;;
  restore)
    sudo pfctl -a "$ANCHOR" -F all 2>/dev/null || true
    if [ -f "$STATE" ]; then
      PREV="$(cat "$STATE")"
      sudo sysctl -w net.inet.ip.forwarding="$PREV" >/dev/null
      rm -f "$STATE"
    else
      sudo sysctl -w net.inet.ip.forwarding=0 >/dev/null
    fi
    rm -f "$CONF"
    echo "NAT-only rules removed; IP forwarding restored"
    ;;
  *) usage ;;
esac
