# Gateway/ProxyBot release checklist

Gateway is Beta until every blocking item below passes. This list is the release
gate, not a wish list.

## Documentation and support boundary

- [ ] README says explicit proxy is Core; Gateway is Beta.
- [ ] Legacy pf gateway marked legacy and hidden from help/UI.
- [ ] `docs/status.md` reflects verified vs unverified behavior.
- [ ] Gateway guide includes start/stop/recovery and known limitations.
- [ ] ADR-0010 remains the boundary record.

## Licensing and dependencies

- [ ] sing-box is not inside the release archive unless GPL compliance work is
  complete.
- [ ] Exact sing-box version/revision/build tags recorded.
- [ ] User-supplied dependency is documented; missing/wrong build fails clearly.
- [ ] License/notice files are included for all distributed components.

## Lifecycle gates (blocking)

Lifecycle test matrix: `docs/status/2026-10-lifecycle-reliability.md`. The
current supervisor has no sleep/Wi-Fi event handling; tests must be run before
launchd design is accepted.

- [x] launchd: service lifecycle (install/start/stop/crash/no-restart/uninstall)
  and service-mode sleep/wake/Wi-Fi/manual-stop regression passed.
- [ ] Sleep/wake: no stale TUN or pf after wake; state/status accurate.
- [ ] Wi-Fi switch/IP change: gateway recovers, reconfigures or reports failure.
- [ ] Recovery failure is visible/retryable; never reports clean stopped.
- [ ] Uninstall removes ProxyBot resources and restores system network settings.
- [ ] Repeated/concurrent start is safe.
- [ ] Mac and phone network recovery verified after every failure path.

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

- [ ] Version bumped in the one source of truth; changelog updated.
- [ ] Fresh-environment install (no dev caches or /tmp sing-box) succeeds.
- [ ] Upgrade and uninstall paths tested.
- [ ] Release archive checksums and install script verified.
- [ ] `cargo test`, release build and any CI gate pass.
- [ ] Release notes explicitly label Gateway Beta and list limits.
