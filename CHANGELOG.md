# Changelog

All notable changes to ProxyBot are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project uses semantic versioning for the Product Version declared in
`package.json`. This changelog starts with the unreleased 1.3.0 line; earlier
releases are listed on the GitHub Releases page.

The pre-lightweight state is archived at Git tag
`archive/pre-lightweight-2026-10-22` (see
[ADR-0007](docs/adr/0007-lightweight-positioning.md)).

## [Unreleased] — 1.3.0

The 1.3.0 line is a preview. It is not yet backed by a signed, notarized,
install-verified release; see [SUPPORT.md](SUPPORT.md).

### Added

- Unified Capture Session lifecycle: window and menu-bar controls share one
  start/stop/recovery surface.
- Device Onboarding module: active LAN discovery, temporary CA distribution
  server, explicit-proxy QR/instructions, iOS CA-only download, Android guided
  setup page.
- Generated Desktop Contract: Rust-declared commands and events generate typed
  TypeScript, checked in CI; General Settings, DNS, and Alerts migrated to it,
  with a `BrowserMockAdapter` for UI tests.
- Interactive breakpoint bridge, per-host TLS decryption rules, gzip/deflate/
  brotli body decoding, replay/Composer, HAR export, WebSocket frame inspection,
  extended filter DSL, reverse-proxy mode, Rhai scripting hooks, network
  conditions, and a local mobile dashboard.
- MCP stdio adapter exposing six local tools over persisted data
  (`capture_traffic`, `classify_request`, `apply_rule`, `get_devices`,
  `get_alerts`, `acknowledge_alert`).
- Packaged Desktop Acceptance journey executing the real Tauri composition
  root: CA preparation, decrypted local HTTPS capture, SQLite persistence, stop,
  restart, and cleanup; wired into CI and the release workflow.
- Rockxy Community parity evidence ledger and machine-readable capability
  matrix, with a schema-enforcing check script.
- Documentation governance: `docs/adr/`, this changelog, and `SUPPORT.md`;
  historical records archived under `docs/history/` and excluded from the
  documentation site.

### Changed

- Default navigation converged to three Core destinations — **Capture**,
  **Setup**, and **Settings**. Rules, Replay, Composer, DNS, Alerts, Graph,
  Topology, and remaining Advanced/Labs tools are reached through the Capture
  context; all existing deep-link paths remain valid.
- Product version now comes from `package.json` only. Tauri, the update UI, the
  Rust crates, Cargo.lock, MCP metadata, and the release tag are checked against
  it by one consistency tool.
- Release automation uses the Tauri bundler with Developer ID signing,
  notarization, checksums, SPDX SBOMs, and provenance attestations, gated on an
  exact `v<version>` tag and fail-closed on missing credentials.
- MCP and the desktop consume the same persisted facts; desktop contract
  failures remain typed errors instead of silent empty state.
- Repository/docs slimmed: historical plans and specs moved out of the published
  site, internal agent role templates trimmed, duplicate tooling files removed.

### Removed

- Android SSL bypass: native Frida runtime, Apktool APK patcher, ADB helper,
  bypass UI/store/routes, and e2e coverage. Use upstream Frida tooling outside
  ProxyBot if you need this capability.
- Frida/Apktool bundle resource pipeline: `apktool.jar` and Frida gadget
  downloads, `resources:fetch`/`resources:check`, bundle resource
  configuration, the `frida-runtime` Cargo feature and dependency, and related
  CI/release steps. Source builds no longer require third-party downloads.
- Unreachable placeholder modules and development server stubs with no product
  entry point (CDP server, AI pipeline placeholder, Prometheus metrics server,
  plugin loader/WASM host stubs) and the legacy command surface they exposed.
- Unsupported TUN capture mode and the disconnected iOS VPN experiment.
- Homebrew tap update from CI; the tap is not supported until a maintained tap
  installs a verified release artifact.

### Fixed

- MCP query handling parameterized to close an SQL injection surface.
- Certificate server binds before reporting running, uses the configured port,
  and serves a corrected iOS configuration profile.
- DNS classification skips host resolution for IP-literal targets.
- QR setup SVG is sanitized and stale QR output is cleared on platform switch.
- Topology LIKE-wildcard escaping and `vis-data` import; DAG response edges;
  alert SQL placeholder indices; history loading on startup.
- Desktop contract drift detection and generated-contract check in CI.
- Dev server loads the product version from `package.json`.

## Earlier releases

See [GitHub Releases](https://github.com/mbpz/proxybot/releases) for the
pre-1.3.0 history. The latest published artifact is `v1.2.15` (2026-05-15), a
development preview.
