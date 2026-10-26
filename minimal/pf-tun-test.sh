#!/usr/bin/env sh
# TUN probe stage: route forwarded iPhone traffic into a utun interface created
# by `proxybot tun-probe`. No NAT/return path is implemented, so the phone has
# no internet while this is active.
#
#   # terminal 1 (root):
#   sudo proxybot tun-probe --duration 60
#   # terminal 2:
#   sudo sh minimal/pf-tun-test.sh install utunN
#   # test the phone, then:
#   sudo sh minimal/pf-tun-test.sh restore
set -eu
IFACE="${PROXYBOT_IFACE:-en1}"
CIDR="${PROXYBOT_CIDR:-192.168.3.0/24}"
PEER="${PROXYBOT_TUN_PEER:-10.255.0.2}"
ANCHOR="com.apple/proxybot-tun"
CONF="/tmp/proxybot-tun.conf"
STATE="/tmp/proxybot-tun-forwarding"

usage() { echo "usage: sudo sh $0 install <utunN>|restore" >&2; exit 2; }
[ $# -ge 1 ] || usage

case "$1" in
  install)
    TUN="${2:-}"
    [ -n "$TUN" ] || usage
    if ! ifconfig "$TUN" >/dev/null 2>&1; then
      echo "interface $TUN not found; start 'proxybot tun-probe' first (it prints utunN)" >&2
      exit 1
    fi
    cat > "$CONF" <<RULES
# Keep multicast/broadcast on the normal path (mDNS, discovery), then route
# only internet-bound traffic: never steal traffic destined to the Mac itself or
# to other LAN hosts (router DNS/DHCP/ICMP), which would break Mac internet.
pass in quick on ${IFACE} inet from ${CIDR} to 224.0.0.0/4 keep state
pass in quick on ${IFACE} inet from ${CIDR} to 255.255.255.255/32 keep state
pass in quick on ${IFACE} route-to (${TUN} ${PEER}) inet from ${CIDR} to ! ${CIDR} keep state
RULES
    pfctl -n -f "$CONF"
    PREV="$(sysctl -n net.inet.ip.forwarding)"
    echo "$PREV" > "$STATE"
    for anchor in com.apple/proxybot com.apple/proxybot-nat com.apple/proxybot-rdr "$ANCHOR"; do
      sudo pfctl -a "$anchor" -F all 2>/dev/null || true
    done
    sudo pfctl -a "$ANCHOR" -f "$CONF"
    sudo sysctl -w net.inet.ip.forwarding=1
    echo "TUN route-to loaded: ${CIDR} -> (${TUN} ${PEER})"
    echo "WARNING: no return path; the iPhone has no internet until restore."
    echo "Watch the tun-probe terminal for packets. Restore: sudo sh $0 restore"
    ;;
  restore)
    for anchor in com.apple/proxybot com.apple/proxybot-nat com.apple/proxybot-rdr "$ANCHOR"; do
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
    echo "TUN probe rules removed; forwarding restored"
    ;;
  *) usage ;;
esac
