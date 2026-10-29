# ProxyBot minimal changelog

## [0.1.13] — 2026-10-29

Patch release for the self-update download path.

### Fixed

- `proxybot update` now passes `curl --output`; the 0.1.12 build wrote the archive
  to stdout and treated the output path as a URL. Update now verifies the
  checksum and replaces the binary correctly.

### Notes

- 0.1.12 already published the Gateway Beta/maintenance feature set to GitHub.
- 0.1.13 is the recommended latest release for external acceptance.

## [0.1.12] — 2026-10-29

Gateway Beta and maintenance release; next small version after GitHub v0.1.11.

### Added

- Gateway Beta: TUN + sing-box + Hudsucker path with TLS SNI rewrite, HTTP/80
  transparent-adapter handling, optional UDP/443 QUIC blocking, supervisor,
  launchd service, device identity metadata and per-device source-IP routing.
- CLI `uninstall` (prompts for data/verified CA, `--purge --yes`, Keychain
  handling) and `update` (release download, SHA-256 verification).
- Full `proxybot list`; focused top-level help; source/unknown identity UI.
- Release archives include LICENSE, THIRD_PARTY.md and uninstall.sh; no sing-box.

### Changed

- Explicit HTTP proxy remains Core; Gateway is Beta.
- Legacy pf `rdr` gateway hidden/unsupported.
- sing-box remains a user-supplied GPL-3.0 dependency.
- Product boundary docs, ADRs and lifecycle test scripts.

## [0.1.11] — 2026-10-24

- Passive sniff, TUN probe, HTTP proxy capture, WebSocket UI, SQLite history,
  CA setup and local web UI.
