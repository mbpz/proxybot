# ADR-0009: Gateway device identity and metadata

- **Status:** Accepted
- **Date:** 2026-10-26
- **Phase:** P4.1
- **Scope:** Gateway identity model, trusted boundary, metadata handoff
- **Out of scope:** device-management UI, discovery, authentication

## Context

The gateway path is `macOS TUN → sing-box → HTTP outbound/transparent adapter →
Hudsucker → ProxyBot`. Requests currently record:

```text
source_type = gateway
client_ip   = NULL
peer_ip     = 127.0.0.1
```

Hudsucker sees a loopback connection created by sing-box, not the phone's
connection. The original source IP exists at the TUN ingress but is not yet
carried to Hudsucker. Treating `peer_ip` as the device IP, or guessing identity
from `User-Agent`, `Host`, `X-Forwarded-For` or `X-Device-ID`, is incorrect.

## Decision

### 1. Identity fields

Requests gain two fields in addition to the existing provenance fields:

| Field | Type | Null | Meaning |
| --- | --- | --- | --- |
| `source_type` | enum | no | `local_proxy` or `gateway` |
| `client_ip` | IP | yes | original device IP only when trustworthily associated |
| `peer_ip` | IP | yes | actual Hudsucker TCP peer (`127.0.0.1` for gateway) |
| `device_id` | string | yes | internal device identity when known |
| `connection_id` | string | yes | gateway ingress/connection correlation key |
| `identity_source` | enum | no | `tun_source_ip` / `device_port` / `unknown` |
| `confidence` | enum | no | `confirmed` / `inferred` / `unknown` |

Rules:

- `client_ip = NULL` and `device_id = NULL` are valid states.
- `peer_ip` must never be copied into `client_ip`.
- `identity_source = unknown` requires `confidence = unknown`.
- Unknown identity must not be filled by guessing.

### 2. Trusted boundary

Only ProxyBot-controlled local entry points may establish identity:

- TUN ingress source information passed by a controlled adapter;
- a controlled local metadata handoff;
- a controlled, explicitly mapped local listener/port.

The following are untrusted: `User-Agent`, `Host`, client-supplied
`X-Forwarded-For`/`X-Device-ID`, the loopback peer address, and time-window
correlation. Metadata headers are accepted only from loopback/controlled
adapter connections, are stripped before forwarding, and override any external
header of the same name.

### 3. P4.2 prototype

Use an explicit **adapter metadata handoff**, not an inferred port mapping:

1. sing-box routes by TUN source IP to a per-device outbound/adapter;
2. the adapter adds a signed/controlled metadata envelope on CONNECT:
   `connection_id`, `client_ip`, `device_id` (optional), `identity_source`,
   `confidence`;
3. Hudsucker records this metadata for the connection and attaches it to the
   requests inside that CONNECT tunnel; the metadata mechanism is transport,
   while the evidence type is `identity_source=tun_source_ip`;
4. missing/conflicting metadata degrades to `client_ip=NULL`,
   `device_id=NULL`, `confidence=unknown`.

Per-device HTTP outbound headers are acceptable for the prototype because
sing-box's config is locally trusted and the outbound connects over loopback.
The static per-device header is an `inferred` identity: it proves the configured
route mapping, not a per-connection observation. `confirmed` is reserved for a
future adapter that proves the ingress connection identity per CONNECT.
HTTP/80 needs a per-device adapter/listener (or is deferred) because the single
transparent adapter cannot know the originating device.

### 4. Port mapping is not the long-term design

Port mapping is not accepted as a general identity mechanism. It can only work
if the receiving side can prove which device-owned listener received the
connection. The current single Hudsucker listener sees only an ephemeral source
port and cannot prove that mapping.

If a port prototype is ever run, it must use per-device listeners and prove
ownership from the listening socket, not from an ephemeral client port.

## Technical finding: sing-box HTTP outbound reuse

Repository inspection of the pinned sing-box source:

- `protocol/http/outbound.go` creates one `sHTTP.Client` and calls
  `DialContext` per connection;
- `sing/http/client.go` dials the proxy server and writes a fresh `CONNECT` for
  each `DialContext`; there is no proxy-connection pool in this layer.

So a single CONNECT tunnel does not silently hop between devices. However this
does **not** make port mapping observable: Hudsucker still sees only
`127.0.0.1:<ephemeral>` and cannot tell which device-specific outbound port was
used. The reuse question is therefore not the blocker; the missing trusted
association is.

HTTP keep-alive inside one CONNECT may carry multiple requests, but they remain
on one device's connection and must share one metadata association.

## Consequences

Easier:

- identity is explicit and testable;
- unknown identity is visible instead of guessed;
- P4.2 can validate one device without changing TUN/MITM core.

Harder:

- the adapter must authenticate/limit local metadata;
- HTTP/80 needs a per-device adapter path or remains unassociated;
- connection lifetime and metadata cleanup need tests.

## Evidence

- `minimal/src/singbox_config.rs` generates per-device-capable outbounds.
- `minimal/src/capture.rs` stores `source_type`, `client_ip`, `peer_ip`,
  `connection_id`, `identity_source`, `confidence` and strips `X-ProxyBot-*`
  before forwarding.
- P4.2 raw CONNECT capture and local loopback identity result:
  `docs/status/2026-10-26-p4.2-single-device-metadata.md`.
- sing-box v1.14.4 source: `protocol/http/outbound.go`,
  `sing/protocol/http/client.go`.
