# ProxyBot minimal changelog

## [0.2.0] — 2026-10-28

Gateway Beta and lifecycle release.

### Added

- Gateway Beta: TUN + sing-box + Hudsucker path with TLS SNI rewrite, HTTP/80
  transparent-adapter handling, and optional UDP/443 QUIC blocking.
- Gateway supervisor: start/stop/status, startup lock, exit code/signal/reason,
  stale/orphan cleanup, recovery-failure visibility, log rotation, retry.
- launchd service integration: explicit install/start/stop/uninstall,
  `RunAtLoad`/`KeepAlive` disabled, no automatic restart; service-mode
  sleep/wake/Wi-Fi regression.
- Device identity metadata on CONNECT, `identity_source`/`confidence`, source-IP
  routed per-device outbounds, and a source/unknown filter UI with pagination.
- Lifecycle diagnostics (`gateway status` health/network) and one-command
  lifecycle/launchd test scripts.

### Changed

- Gateway documented as Beta; explicit HTTP proxy remains the supported Core
  mode.
- Legacy pf `rdr` gateway hidden and marked unsupported.
- sing-box is a user-supplied GPL-3.0 dependency; it is not bundled.
- Product status/boundary docs and ADR-0010/ADR-0011 added.

## [0.1.11] — 2026-10-24

- Passive sniff, TUN probe, HTTP proxy capture, WebSocket UI, SQLite history,
  CA setup and local web UI.
