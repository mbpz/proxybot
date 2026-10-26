# ADR-0010: Gateway beta boundary and lifecycle

- **Status:** Accepted
- **Date:** 2026-10-26
- **Phase:** product convergence
- **Supersedes:** the experimental pf `rdr` gateway described in early minimal
  notes (not a supported mode)

## Context

ProxyBot's Core mode is the explicit HTTP proxy. A separate Gateway mode now
uses macOS TUN + sing-box + pf route-to, with a supervisor and per-device
source-IP routing. It has passed multi-phase tests but has not yet been validated
for external users, sleep/Wi-Fi changes, or long-running service management.

The old `pf rdr` gateway and `proxybot gateway install/uninstall` commands remain
in the binary for historical cleanup work; they are not the gateway
implementation and must not be presented as a supported path.

## Decision

1. Gateway is a **Beta** capability. Explicit HTTP proxy remains the supported
   default and the only Core capture mode.
2. Gateway requires a user-supplied sing-box v1.14.4 (or compatible) built with
   `with_gvisor`; ProxyBot does not bundle or distribute sing-box in the beta.
3. The supported gateway entry points are:
   - `proxybot gateway start|stop|status`
   - `proxybot singbox-config`
   - `proxybot gateway start --devices <file>` for per-device identity
4. The legacy pf `rdr` path (`proxybot start --gateway`,
   `proxybot gateway install/uninstall`) is **legacy**, hidden from help, and
   only retained for cleanup/diagnostics.
5. Device identity is `inferred` unless a future ingress adapter proves a
   per-connection association. `connection_id` is proxy-side diagnostic only.
6. Gateway must not be described as "all traffic decrypted": UDP/QUIC and DNS
   are direct, HTTP/80 identity is unknown, and certificate-pinned apps can fail.
7. Before a public Gateway release, the release checklist must pass, including
   launchd/sleep/Wi-Fi behavior, clean-environment install, and the sing-box
   licensing decision.

## Consequences

- README/status/UI must say **Gateway Beta**, not "experimental side-router" and
  not "supported full-device capture".
- The old PF instructions are removed or clearly marked legacy.
- Gateway release work includes GPL review and lifecycle tests, not only feature
  work.

## Evidence

- `docs/status.md`
- `docs/gateway-beta.md`
- `docs/release-checklist.md`
- `docs/legal/sing-box.md`
- `docs/status/2026-10-26-p2-quic-policy.md`
- `docs/status/2026-10-26-p3-gateway-lifecycle.md`
- `docs/status/2026-10-26-p4.2-single-device-metadata.md`
- `docs/status/2026-10-26-p4.3-device-routing.md`
