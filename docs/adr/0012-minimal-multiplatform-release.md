# ADR-0012: Minimal CLI is the sole active product and ships Windows x64

## Status

Accepted (2026-11-02)

## Context

`minimal/` is the active CLI and local web UI. The root Tauri/React workspace
was retained as legacy source with a separate CI and release pipeline. That
made the repository's build, version and platform story ambiguous. The explicit
HTTP proxy does not require macOS system integration; Gateway Beta does.

## Decision

- Keep `minimal/` as the sole active application workspace. Remove the old
  Tauri/React source, its toolchain configuration and its release pipeline.
- Build and test the explicit proxy on macOS arm64, macOS x64 and Windows x64.
  Package Windows as a ZIP with SHA-256; retain macOS tarballs.
- Compile pf, launchd, Gateway Beta, BPF sniffing and TUN probing on macOS only.
  Windows exposes explicit proxy capture, the local web UI, the device CA page
  and rules commands. It uses `%LOCALAPPDATA%\ProxyBot` for user data.
- Keep historical ADRs and `docs/history/` as records of past decisions.

## Consequences

The active build and release path is smaller. Windows runtime behavior still
requires Windows CI and device acceptance before a release support claim.
Windows self-update, self-uninstall and Gateway Beta are outside this decision.

## Evidence

- `minimal/Cargo.toml`, `minimal/src/main.rs`, `minimal/src/platform.rs`
- `.github/workflows/minimal-ci.yml`, `.github/workflows/minimal-release.yml`
- `minimal/package.ps1`, `minimal/README.md`
- `cargo test --locked --manifest-path minimal/Cargo.toml`

Supersedes the active-workspace and release-contract portions of ADR-0001,
ADR-0004 and ADR-0006.
