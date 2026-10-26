# P2 acceptance: UDP/QUIC policy

## Scope

- Default: UDP/443 and QUIC remain **direct**; they are not MITM and do not
  create HTTP request rows.
- Optional: `--block-quic` adds a sing-box route-layer reject for UDP/443, to
  test whether browsers/apps fall back to TCP/TLS and whether that fallback is
  still decrypted.
- DNS remains direct in this phase; no DNS policy change.
- No pf rule is added for QUIC; policy lives in the sing-box config.

## Commands

```bash
# Default direct policy (writes <data>/singbox-tun-mitm.json and quic-policy=direct)
proxybot singbox-config

# Optional fallback test policy (quic-policy=block)
proxybot singbox-config --block-quic

# Then run the gateway
sudo sing-box run -c "$HOME/Library/Application Support/ProxyBot/singbox-tun-mitm.json"
sudo sh minimal/pf-tun-test.sh install utun6
```

ProxyBot's Setup page reads `<data>/quic-policy` and displays `UDP / QUIC:
direct` or `blocked (fallback test)`.

## Phone evidence (2026-10-26)

- Default direct: Bilibili loaded; Setup showed `UDP / QUIC: direct`.
- Block mode: Setup showed `blocked (fallback test)`; Bilibili loaded again,
  proving TCP/TLS fallback for this browser; Bilibili HTTPS requests were
  captured by ProxyBot.
- sing-box log: `router: pre-match[3] network=udp port=443 => reject` matched
  the phone's QUIC attempts; no UDP rows were fabricated.
- TCP/80, DNS (`223.5.5.5:53`) and the Mac's own internet remained working.
- Cleanup/restore confirmed: ProxyBot and sing-box stopped, `utun6` gone, pf
  anchors flushed, Mac HTTPS 200, iPhone back on automatic networking.
  (`net.inet.ip.forwarding` returned to its previously saved value of 1; with no
  TUN/route rules this is inert, but `sudo sysctl -w net.inet.ip.forwarding=0`
  can be used if a fully pristine state is desired.)

## Acceptance checklist

| # | Check | Pass condition |
| --- | --- | --- |
| 1 | Default policy | no `--block-quic`; UDP/443 forwarding unchanged |
| 2 | Block policy takes effect | gateway stops forwarding UDP/443 normally |
| 3 | TCP fallback | browser may switch to TCP/TLS; record if it does |
| 4 | Fallback HTTPS MITM | if fallback works, HTTPS still appears in ProxyBot |
| 5 | No collateral damage | DNS, TCP/80, TCP/443 and Mac internet unaffected |
| 6 | Clean restore | **PASS** — no processes/utun/pf route; iPhone recovered |

Important: a page failure after blocking QUIC is not automatically a bug; a
successful fallback is not guaranteed for every app. Record "rule effective"
and "client fallback" separately.

## Request/UI semantics

- Request list only contains captured HTTP(S) requests; no fake rows for UDP.
- Gateway-origin requests keep `source_type=gateway`, `client_ip=NULL`,
  `peer_ip=127.0.0.1`.
- Setup status shows the QUIC policy and that UDP is not decrypted.
