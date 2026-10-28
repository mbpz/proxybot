# ProxyBot status

Last updated: 2026-10-26 (minimal CLI v0.2.0)

This is the single status overview. Phase records live under `docs/status/`.

## Capture modes

| Mode | Status | Notes |
| --- | --- | --- |
| Explicit HTTP proxy | **Supported / default** | iPhone Safari/WeChat verified; no admin |
| Gateway (TUN + sing-box) | **Beta** | Not a Core support claim; see limitations |
| Passive sniff | Experimental | Metadata only; no decryption |
| Legacy pf `rdr` gateway | **Legacy / unsupported** | Historical research only; not the gateway implementation |

## Gateway beta verified

- TUN ingress and sing-box sniff/routing.
- HTTPS MITM through Hudsucker, including upstream SNI rewrite.
- HTTP/80 through the transparent adapter; HTTP/80 identity remains unknown.
- UDP/DNS direct; optional UDP/443 reject for QUIC fallback testing.
- Supervisor lifecycle: start/stop/status, startup mutex, exit code/signal/reason,
  stale/orphan cleanup, recovery injection, log rotation.
- Per-device source-IP routing with two devices; same-domain concurrent requests
  did not cross identities.
- Device metadata: `client_ip` and `identity_source=tun_source_ip` with
  `confidence=inferred`; `device_id` remains NULL; `connection_id` is proxy-side.
- Gateway stop restored pf/forwarding and Mac networking in tested runs.
- Source/device UI: source dropdown grouping by `client_ip`, unknown-identity
  filter, identity labels, detail navigation and offset pagination; browser
  interaction verified.
- Lifecycle baseline: sleep/wake, Wi-Fi loss/reconnect, different-Wi-Fi stale
  detection, manual-stop sleep/wake and recovery passed.
- launchd service: process lifecycle (PASS=22) and service-mode sleep/Wi-Fi/
  manual-stop/kill/uninstall regression (PASS=23, all manual checks confirmed)
  passed. Automatic IP-change reconfiguration remains out of scope.

## Not verified / not supported

- launchd, macOS sleep/wake, Wi-Fi switch/IP change.
- More than two devices; multiple devices behind one source IP/NAT.
- Exact TUN ingress-to-request connection correlation.
- Full DNS policy/leak verification.
- Certificate-pinned apps; UDP/QUIC decryption.
- External-user packaging, clean-room install, upgrade/uninstall.
- sing-box bundled distribution and commercial GPL review.

## Known behavior

- HTTPS interception requires installing/trusting the ProxyBot CA; pinned apps can
  fail.
- HTTP/80 is MITM'd but identity is unknown.
- UDP/QUIC and DNS are not MITM'd.
- Gateway requests under static device metadata are `inferred`, not `confirmed`.
- A local process can spoof loopback metadata; stronger process auth is deferred.
- `confidence` is an association-strength label, not device authentication.

## Roadmap gates

Before a public Gateway release:

1. Documentation/Beta boundary (this change).
2. sing-box GPL-3.0 distribution decision (`docs/legal/sing-box.md`).
3. Device list/filter UI (P4.4), preserving unknown/inferred semantics.
4. launchd/sleep/Wi-Fi lifecycle gates.
5. Release/package/clean-environment checklist.
