# Gateway/ProxyBot release checklist

Gateway is Beta until every blocking item below passes. This list is the release
gate, not a wish list.

## Documentation and support boundary

- [x] README says explicit proxy is Core; Gateway is Beta.
- [x] Legacy pf gateway marked legacy and hidden from help/UI.
- [x] `docs/status.md` reflects verified vs unverified behavior.
- [x] Gateway guide includes start/stop/recovery and known limitations.
- [x] ADR-0010 remains the boundary record.

## Licensing and dependencies

- [x] sing-box is not inside the release archive unless GPL compliance work is
  complete.
- [x] Exact sing-box version/revision/build tags recorded.
- [x] User-supplied dependency is documented; missing/wrong build fails clearly.
- [x] License/notice files are included for all distributed components.

## Lifecycle gates (blocking)

Lifecycle test matrix: `docs/status/2026-10-lifecycle-reliability.md`. The
current supervisor has no sleep/Wi-Fi event handling; tests must be run before
launchd design is accepted.

- [x] launchd: service lifecycle (install/start/stop/crash/no-restart/uninstall)
  and service-mode sleep/wake/Wi-Fi/manual-stop regression passed.
- [x] Sleep/wake: no stale TUN or pf after wake; state/status accurate.
- [x] Wi-Fi switch/IP change: gateway reports stale/unknown and requires explicit
  recovery (no automatic reconfiguration).
- [x] Recovery failure is visible/retryable; never reports clean stopped.
- [x] Uninstall removes ProxyBot resources and restores system network settings
  (service/temporary-prefix tests).
- [x] Repeated/concurrent start is safe.
- [x] Mac and phone network recovery verified after tested failure paths.

## Gateway data behavior

- [ ] HTTPS MITM works with trusted CA.
- [ ] HTTP/80 works and remains `identity_source=unknown`.
- [ ] UDP/QUIC/DNS are visibly direct, not falsely captured.
- [ ] Known/unknown device identity semantics preserved; `inferred` not shown as
  confirmed.
- [ ] External metadata spoof from LAN does not establish identity.
- [ ] Per-device routing tested with at least two devices; same-domain concurrent
  requests do not cross.
- [ ] Static metadata/local loopback spoof limitations documented.

## Build/release

- [x] Version bumped in the one source of truth; changelog updated.
- [x] Fresh-prefix install and fresh-data-directory smoke test pass.
- [ ] Upgrade from 0.1.11 and a truly clean external Mac remain untested.
- [x] Release archive checksum and local install script verified.
- [ ] CI gate still to run; local `cargo test`/release build pass.
- [x] Release notes/changelog label Gateway Beta and list limits.
