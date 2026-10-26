# Gateway strategy revision (2026-10-26)

## Review conclusions

1. **Drop `pf rdr` as the transparent-proxy mechanism.** Apple does not expose PF
   as a supported third-party product API. Stage 1 proved that same-interface
   IPv4 forwarding + NAT works on this Mac (NAT counters and bidirectional
   tcpdump traffic), but Stage 2 proved that pf `rdr` matched packets (80: 78
   packets/5 states, 443: 80 packets/16 states) without ever delivering a
   connection to the local listener, for both loopback and LAN-IP targets.
2. **Network Extension is the long-term candidate, not an assumed solution.**
   `NETransparentProxyProvider` handles connection flows (TCP/UDP), not arbitrary
   IP forwarding. The first step is a minimal probe that proves it can see the
   forwarded iPhone traffic on the target macOS, before building an App,
   signing/entitlement flow and MITM integration.
3. **TUN means "IP packets", not "user-space TCP stack + MITM".**
   Reuse a mature user-space forwarding engine where possible; first prove that
   forwarded packets can enter a TUN interface and return correctly. Do not start
   by implementing a TCP stack.
4. **Separate the capability layers.** A gateway forwards/NATs; a passive
   sniffer observes; a proxy/MITM processes selected flows. They must be
   validated independently. The iPhone is a separate LAN device, so "Mac's own
   traffic" tests do not prove the forwarded-device path.

## Decision

| Direction | Decision | Reason |
| --- | --- | --- |
| Network Extension | candidate; probe first | may or may not see forwarded iPhone flows |
| TUN + user-space engine | mid-term verification | verify IP packet ingress/return before integration |
| BPF passive sniff | implement now | low risk, immediately useful visibility |
| Explicit HTTP proxy | keep as supported | already verified with iPhone Safari/WKWebView/JSON |

## Order of work

1. `proxybot sniff` (implemented and verified 2026-10-26): interface, device
   filter, DNS, TLS SNI, flow/connection statistics; terminal + optional JSONL
   log. Real runs on en1: 298 packets / 45 flows unfiltered; with
   `--device 192.168.3.190` it captured DNS queries (example.com, google.com,
   calendar.google.com, apple-dns, qq/wechat CDNs) and TLS SNI (ecosia.org,
   bing.com, example.com, google, apple, finderbdy/findercsysw.video.qq.com,
   mmbiz.qpic.cn), plus QUIC/UDP flows. Acceptance for C passed.
2. Gateway forwarding was validated (Stage 1). Keep it as a prerequisite check,
   not a product feature.
3. TUN minimal prototype: ingress is proven (`docs/status/2026-10-26-tun-probe-plan.md`).
   Engine evaluation: `docs/status/2026-10-26-tun-engine-evaluation.md`;
   tun2socks (MIT) is the chosen candidate for the TCP/UDP closed loop, with
   sing-box (GPL-3.0) as a reference/alternative. **Direct TUN/TCP/UDP closed
   loop passed** with tun2socks `direct://`; next is the HTTP-proxy/MITM outlet.
4. Evaluate Network Extension only if productisation requires system lifecycle
   and supported interception.
5. Keep explicit HTTP proxy as the stable fallback; experimental features must
   not affect it.

## Acceptance boundaries

- Sniff cannot decrypt HTTPS; ECH/DoH can hide SNI/DNS.
- Sniff only sees packets that reach the capture interface.
- Gateway/NE/TUN work must not regress the HTTP proxy path.
- `--gateway` remains experimental and warning-gated until a supported mechanism
  passes end-to-end validation.
