# ProxyBot

[![CI](https://github.com/mbpz/proxybot/actions/workflows/ci.yml/badge.svg)](https://github.com/mbpz/proxybot/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Documentation](https://img.shields.io/badge/docs-GitHub%20Pages-blue.svg)](https://mbpz.github.io/proxybot/)

ProxyBot is a macOS desktop proxy for inspecting HTTP, HTTPS, and WebSocket traffic from devices on the same network. It combines a Rust proxy core, a Tauri desktop application, DNS correlation, and application-aware traffic classification.

> **Project status:** ProxyBot is under active development. Interfaces, on-disk data, and capture behavior may change before a stable release.

> **Authorized use only:** TLS interception exposes sensitive traffic and changes the device trust model. Use ProxyBot only on devices, applications, and networks you own or are explicitly authorized to test. Never share captured payloads, private keys, certificates, or access tokens in public issues.

## Capabilities

- Transparent macOS routing through `pf`, plus explicit proxy workflows
- HTTP/HTTPS interception with a locally generated certificate authority
- WebSocket capture, replay, breakpoints, and request/response rules
- DNS logging and traffic classification by host, SNI, and known application domains
- React + TypeScript desktop interface delivered by Tauri
- Reusable `proxybot-core` Rust crate without GUI dependencies
- MCP stdio mode, mobile dashboard, filters, export, and protocol decoders
- Optional Android SSL-bypass and APK-patching tools

Some applications use certificate pinning or platform protections that intentionally prevent decryption. ProxyBot can still expose DNS/SNI metadata in many of those cases, but it cannot guarantee payload access.

## Build from source

ProxyBot currently targets macOS. You need:

- Xcode Command Line Tools
- A stable Rust toolchain
- Node.js 20 or newer
- pnpm 10
- The [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/)

```bash
git clone https://github.com/mbpz/proxybot.git
cd proxybot
pnpm install --frozen-lockfile
pnpm tauri dev
```

The default development build omits the native Frida runtime. Enable live
Frida device and process operations when needed with:

```bash
pnpm tauri dev --features frida-runtime
```

Create a release bundle with:

```bash
pnpm build:tauri
```

The bundle step downloads pinned Apktool and Frida Gadget assets for the optional APK patcher. They are verified against SHA-256 digests in [`src-tauri/resources/resources.lock`](src-tauri/resources/resources.lock) and are not stored in Git. Run `pnpm resources:fetch` explicitly to prepare an offline bundle, or `pnpm resources:check` to verify an existing cache without network access.

Release tooling explicitly enables the optional `frida-runtime` Cargo feature. The first such build downloads the matching Frida Core development kit through `frida-rust`. Core development and CI use `--no-default-features`, which keeps the same IPC commands but returns a clear capability error for live Frida operations and can build fully offline after Cargo dependencies are cached.

## How traffic reaches ProxyBot

```text
Phone ──Wi-Fi──> Mac (pf / explicit proxy) ──> ProxyBot ──> upstream server
                         │                       │
                         └── built-in DNS ───────┘
                             correlation
```

On first use, ProxyBot creates a local CA. Install and trust that CA only on a test device. Configure the device to use the Mac as its proxy/gateway and DNS server, then start capture from the desktop application. See the [getting started guide](docs/getting-started.md) for the full setup and cleanup procedure.

## Repository layout

```text
proxybot-core/   reusable proxy, certificate, classification, and spec modules
src-tauri/       single desktop bootstrap, Tauri commands, platform integration, and storage
src/             React application and browser-side tests
e2e/             Playwright end-to-end coverage
docs/            MkDocs sources and architecture notes
ios/             experimental iOS packet-tunnel code
scripts/         development and reproducible asset tooling
```

## Development checks

The local CI command covers formatting, Rust tests and lints, TypeScript checks, UI tests, and the frontend production build:

```bash
pnpm ci:local
```

Individual checks are also available:

```bash
cargo test --workspace --locked --no-default-features
cargo clippy --workspace --all-targets --locked --no-default-features -- -D warnings
cargo fmt --all -- --check
pnpm typecheck
pnpm test:ui
pnpm test:e2e
pnpm build
```

## Contributing and security

Read [CONTRIBUTING.md](CONTRIBUTING.md) before opening a pull request. Use the issue forms for reproducible bugs and focused feature requests. Report vulnerabilities privately as described in [SECURITY.md](SECURITY.md), and follow the [Code of Conduct](CODE_OF_CONDUCT.md) in all project spaces.

ProxyBot is available under the [MIT License](LICENSE). Downloaded Apktool and Frida assets remain under their respective upstream licenses.
