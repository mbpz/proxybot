# ProxyBot repository guide

The active application is `minimal/`: a Rust CLI with an embedded local web UI.
The explicit HTTP proxy works on macOS and Windows x64. Gateway Beta, pf,
launchd, passive sniffing, and the maintenance commands are macOS only.

## Source map

- `minimal/src/main.rs`: CLI and server composition.
- `minimal/src/platform.rs`: operating system paths, LAN address, browser launch.
- `minimal/src/capture.rs`, `ca.rs`, `store.rs`, `rules.rs`: capture, CA, persistence, classification.
- `minimal/src/web.rs` and `minimal/web/`: HTTP API and embedded UI.
- `minimal/src/gateway*.rs`, `launchd.rs`, `sniff.rs`, `tun_probe.rs`: macOS-only paths.
- `.github/workflows/minimal-ci.yml`: macOS and Windows checks.
- `.github/workflows/minimal-release.yml`: manual macOS tarball and Windows ZIP release.

## Local checks

```bash
cargo test --locked --manifest-path minimal/Cargo.toml
cargo build --release --locked --manifest-path minimal/Cargo.toml
sh minimal/package.sh                 # macOS
./minimal/package.ps1                 # Windows PowerShell
```

Do not commit generated binaries, captured traffic, local CAs, private keys,
or downloaded dependencies. Documentation must distinguish platform support
and local verification from published releases. Historical design records under
`docs/history/` and accepted ADRs describe past decisions, not current code.
