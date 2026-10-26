# Phase 2 acceptance record (2026-10-26)

## Accepted statement

> Phase 2: the HTTPS MITM path for a tested domain is verified. TUN ingress,
> domain sniffing, sing-box routing, Hudsucker upstream forwarding and ProxyBot
> request persistence were observed for `example.com`. Other protocols, device
> identity attribution and product reliability are not yet accepted.

This is an engineering acceptance record, not a claim that all traffic from all
apps or all devices can be captured/decrypted.

## Verified

- iPhone traffic enters the macOS TUN path (sing-box TUN inbound).
- `action: sniff` recovers TLS SNI and routes TCP/443 to the HTTP outbound.
- Hudsucker receives the request; after rewriting an IP-based CONNECT request to
  the ClientHello SNI, upstream forwarding and certificate validation succeed.
- ProxyBot records `example.com` requests (`/`, `/s.js`) with 200 responses.
- HTTP/80: Hudsucker's CONNECT handling only special-cases `GET`, so POST and
  other methods fail with `Unknown protocol`. Fixed by routing TCP/80 to
  ProxyBot's transparent adapter (`override_address=127.0.0.1`,
  `override_port=8089`), which rewrites absolute-form requests and marks
  `X-ProxyBot-Source: gateway`. Locally verified GET and POST (200/405), and
  verified on the phone: `captive.apple.com/hotspot-detect.html` over HTTP/80
  was recorded as `source_type=gateway`, `client_ip=NULL`,
  `peer_ip=127.0.0.1`, status 200. Safari sometimes upgrades the same URL to
  HTTPS, which is expected.
- DNS: sing-box logs show phone DNS packet connections to 223.5.5.5; DNS remains
  on the direct route (not MITM/hijacked) and resolution works.
- Provenance marker `X-ProxyBot-Source: gateway` from the sing-box HTTP outbound
  is stored as `source_type=gateway`; the header is stripped before forwarding.
- Existing direct/HTTP-proxy mode keeps `source_type=local_proxy` and the real
  client IP.

## Not verified / known boundaries

- UDP/QUIC/ICMP behavior and policy (currently direct; not MITM); DNS works but
  is not policy-controlled.
- Multi-device identity/concurrency; gateway requests currently have
  `client_ip=null` and `peer_ip=127.0.0.1`.
- Gateway lifecycle: crash recovery, Wi-Fi switch, sleep, one-command stop.
- Certificate trust/security review, sensitive-data handling.
- Packaging/distribution and sing-box GPL-3.0 obligations.

## Data model (request provenance)

| Field | Meaning |
| --- | --- |
| `source_type` | `local_proxy` (direct HTTP proxy) or `gateway` (sing-box/TUN) |
| `client_ip` | original device IP when trustworthy; `NULL` for gateway |
| `peer_ip` | actual socket peer seen by Hudsucker (`127.0.0.1` for gateway) |
| `device_id` | reserved; `NULL` until identity propagation |
| `connection_id` | local connection key for diagnostics (currently peer address) |

Do not forge the device IP into `client_ip`; filtering/statistics should use
`source_type` and `client_ip` separately.

## Reproduction

- sing-box v1.14.4 (GPL-3.0, evaluation only), built with `-tags with_gvisor`.
- Config: `minimal/examples/singbox-tun-mitm.json` (TUN `utun6`, sniff, TCP/80
  and TCP/443 → HTTP outbound 8088, UDP direct, source header).
- PF route-to: `minimal/pf-tun-test.sh install utun6`.
- ProxyBot: current `minimal/target/release/proxybot` build.
- Phone: manual IPv4, gateway = Mac, DNS = 223.5.5.5, HTTP proxy off.

## Phase 3 order

### P0 — data model and acceptance (this change)

- Add `source_type`, `peer_ip`, `device_id`, `connection_id` with additive
  SQLite migration.
- Record gateway provenance from the sing-box `X-ProxyBot-Source` header.
- Fix HTTP/80 by rewriting an IP request authority from the `Host` header.
- Update Setup UI to show gateway beta status instead of only an on/off claim.

### P1 — HTTP/80 and DNS validation

- Phone HTTP/80 test through TUN + sniff + HTTP outbound.
- Check DNS egress/resolution and log correlation.
- Record upstream failures/timeouts explicitly.

### P2 — UDP/QUIC policy

- Implemented `proxybot singbox-config [--block-quic]`: default UDP/443 direct;
  optional sing-box route-layer reject for fallback tests.
- Setup shows the policy; request list never fakes UDP rows.
- Acceptance checklist: `docs/status/2026-10-26-p2-quic-policy.md`.

### P3 — gateway lifecycle

- One-command start/stop/health check.
- Restore routing/forwarding on failure, sleep, Wi-Fi switch.
- Verify Mac never loses its own network.

### P4 — packaging and multi-device

- Decide external sing-box process vs. own adapter; complete GPL review.
- Add device identity propagation, device list and concurrency only after P0–P3.
