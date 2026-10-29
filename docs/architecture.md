# Architecture

ProxyBot's active implementation is the minimal CLI + local web UI under
`minimal/`. The Tauri desktop app remains in the repository as legacy work.

## Runtime

```text
device --HTTP proxy--> Mac :8088 ─┐
device --gateway-----> TUN ───────┴─> Hudsucker (MITM)
                                        ├─ SQLite history
                                        ├─ app/source classification
                                        ├─ CA page :8090
                                        └─ web UI 127.0.0.1:8899
```

Components:

- `minimal/src/main.rs` — CLI entry point and HTTP proxy/web composition.
- `minimal/src/capture.rs` — Hudsucker handler, request persistence, identity.
- `minimal/src/transparent.rs` — plain HTTP/80 adapter.
- `minimal/src/gateway.rs`, `gateway_run.rs` — pf/route-to and supervisor.
- `minimal/src/launchd.rs` — explicit LaunchDaemon lifecycle.
- `minimal/src/singbox_config.rs` — TUN/sniff/per-device config generation.
- `minimal/src/store.rs` — SQLite schema and queries.
- `minimal/web/` — embedded Live/Setup/TLS/Rules UI.

## Gateway Beta

```text
device -> pf route-to -> sing-box TUN -> sniff
                                      ├─ TCP/443 -> HTTP CONNECT -> Hudsucker
                                      ├─ TCP/80  -> transparent adapter -> Hudsucker
                                      └─ UDP/DNS -> direct
```

- Supervisor owns sing-box, startup health, exit diagnostics and network
  restoration.
- launchd manages process existence only; it does not auto-start, auto-restart or
  reapply network rules.
- Device identity uses trusted loopback CONNECT metadata; static source routing
  is `confidence=inferred`, not cryptographic device authentication.
- CA fingerprints are used for uninstall-time certificate matching.

## Decisions

- [ADR-0002](adr/0002-explicit-proxy-first.md) — explicit proxy first.
- [ADR-0009](adr/0009-gateway-device-identity.md) — identity model.
- [ADR-0010](adr/0010-gateway-beta-boundary.md) — Gateway Beta boundary.
- [ADR-0011](adr/0011-launchd-gateway-boundary.md) — launchd boundary.
