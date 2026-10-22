# ADR-0002: Explicit proxy is the Core capture mode

- **Status:** Accepted
- **Date:** 2026-10-22
- **Deciders:** ProxyBot maintainers

## Context

ProxyBot can observe mobile traffic in several ways. The lightest and most
predictable is an explicit Wi-Fi HTTP proxy configured on the test device:
configuration and cleanup stay on the device, no host-wide network state
changes, and no elevated privileges are required. The alternative, a macOS `pf`
redirect plus a built-in DNS server, captures traffic the device would not
otherwise send to the proxy, but it mutates host network state, may require
administrator rights, and needs explicit cleanup when capture stops or the app
quits.

A first-time user must reach one decrypted request without discovering an
undocumented prerequisite. Making the invasive mode the default would trade a
reliable onboarding path for broader capture.

## Decision

1. Explicit proxy is the Core and default capture path. It is the only mode
   documented in the first-capture guide and the only one required for first
   success.
2. macOS `pf` and the built-in DNS server are Advanced capabilities. They may
   change host network state, may require elevated privileges, and must provide
   explicit start, failure, and cleanup behavior.
3. Advanced capture modes must never be a prerequisite for Core capture, and
   their failures must not be reported as capture-session failures of the
   explicit-proxy path.
4. Device onboarding prepares the CA and explicit-proxy inputs independently of
   the capture session so either lifecycle can fail or stop without
   misreporting the other.

## Consequences

- Onboarding, docs, and acceptance tests target the explicit-proxy path first.
- `pf` and DNS remain available for users who need them, with clearly documented
  prerequisites and cleanup.
- Certificate installation is a separate step from capture; the guide keeps
  "verify one HTTP request, then one HTTPS request" as the first-success
  sequence.
- The project does not promise interception of traffic that never reaches the
  proxy.

## Evidence

- `docs/architecture.md` ("Network modes")
- `docs/getting-started.md`
- `src-tauri/src/pf.rs` (Advanced `pf` adapter)
- `src-tauri/src/commands/device_setup.rs` (explicit-proxy onboarding)
- `proxybot-core/src/config.rs` (default proxy port 8088)
