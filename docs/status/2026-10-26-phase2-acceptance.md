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
- HTTP/80: locally verified through a CONNECT-by-IP + `Host` rewrite
  (200, recorded as `example.com`); phone verification is pending.
- Provenance marker `X-ProxyBot-Source: gateway` from the sing-box HTTP outbound
  is stored as `source_type=gateway`; the header is stripped before forwarding.
- Existing direct/HTTP-proxy mode keeps `source_type=local_proxy` and the real
  client IP.

## Not verified / known boundaries

- Phone HTTP/80 through the full TUN path.
- UDP/DNS/QUIC/ICMP behavior and policy (currently direct; not MITM).
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

- Default direct, visibly "not decrypted".
- Optional QUIC blocking to compare fallback behavior.
- Never represent UDP as captured HTTP.

### P3 — gateway lifecycle

- One-command start/stop/health check.
- Restore routing/forwarding on failure, sleep, Wi-Fi switch.
- Verify Mac never loses its own network.

### P4 — packaging and multi-device

- Decide external sing-box process vs. own adapter; complete GPL review.
- Add device identity propagation, device list and concurrency only after P0–P3.
