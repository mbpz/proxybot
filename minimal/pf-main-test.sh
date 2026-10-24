#!/usr/bin/env sh
# EXPERIMENT: load ProxyBot's redirect/NAT rules into the MAIN pf ruleset
# instead of an anchor, preserving macOS's built-in com.apple anchors.
#
#   sudo sh minimal/pf-main-test.sh install
#   sudo sh minimal/pf-main-test.sh restore
#
# install flushes the current main ruleset temporarily. Restore with `restore`
# (loads /etc/pf.conf again) when finished.
set -eu
IFACE="${PROXYBOT_IFACE:-en1}"
CIDR="${PROXYBOT_CIDR:-192.168.3.0/24}"
PORT="${PROXYBOT_TRANSPARENT_PORT:-8089}"
RDR_IP="${PROXYBOT_RDR_IP:-127.0.0.1}"
CONF="/tmp/proxybot-pf-main.conf"

usage() { echo "usage: sudo sh $0 install|restore" >&2; exit 2; }
[ $# -eq 1 ] || usage

case "$1" in
  install)
    cat > "$CONF" <<RULES
scrub-anchor "com.apple/*"
dummynet-anchor "com.apple/*"
nat-anchor "com.apple/*"
rdr-anchor "com.apple/*"

rdr pass on ${IFACE} inet proto tcp from any to any port {80, 443} -> ${RDR_IP} port ${PORT}
nat on ${IFACE} inet from ${CIDR} to any -> (${IFACE})

# Keep our LAN pass rules BEFORE the com.apple anchors so a stale Application
# Firewall dynamic anchor cannot block phone -> Mac traffic.
block drop quick on ${IFACE} inet proto udp from ${CIDR} to any port {80, 443}
pass in quick on lo0 inet from any to any keep state
pass in quick on ${IFACE} inet from ${CIDR} to any keep state
pass out quick on ${IFACE} inet from any to any keep state

anchor "com.apple/*"
load anchor "com.apple" from "/etc/pf.anchors/com.apple"
RULES
    pfctl -n -f "$CONF"
    # Avoid duplicate rules from the app-owned anchor while testing.
    sudo pfctl -a com.apple/proxybot -F all 2>/dev/null || true
    sudo pfctl -f "$CONF"
    sudo sysctl -w net.inet.ip.forwarding=1
    sudo pfctl -e >/dev/null 2>&1 || true
    echo "main pf ruleset loaded from $CONF (iface=$IFACE cidr=$CIDR rdr=$RDR_IP port=$PORT)"
    echo "test the phone now; restore with: sudo sh $0 restore"
    ;;
  restore)
    sudo pfctl -f /etc/pf.conf
    sudo sysctl -w net.inet.ip.forwarding=0
    echo "restored /etc/pf.conf and disabled IP forwarding"
    ;;
  *) usage ;;
esac
