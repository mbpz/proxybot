# ProxyBot

[![CI](https://github.com/mbpz/proxybot/actions/workflows/ci.yml/badge.svg)](https://github.com/mbpz/proxybot/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Documentation](https://img.shields.io/badge/docs-GitHub%20Pages-blue.svg)](https://mbpz.github.io/proxybot/)

ProxyBot is an early-stage macOS desktop traffic debugger for mobile application
developers. It helps an iOS or Android test device connect to a local Rust MITM
Runtime, then inspect, filter, modify, replay, and export HTTP, HTTPS, and
WebSocket Captured Requests.

> **Development preview:** interfaces, on-disk data, capture behavior, and
> release packaging may change before the first stable release. Existing GitHub
> release artifacts are previews; the supported contributor path is currently a
> source build.

> **Authorized use only:** TLS interception exposes sensitive traffic and
> changes the device trust model. Use ProxyBot only on devices, applications,
> and networks you own or are explicitly authorized to test. Never share
> captured payloads, private keys, certificates, or access tokens in public
> issues.

## Product focus

ProxyBot is converging on one dependable workflow:

1. Start a Capture Session on a Mac.
2. Connect a test device with an explicit proxy.
3. Install and trust the local CA when HTTPS inspection is needed.
4. Verify a known request.
5. Inspect, filter, modify, replay, or export the Captured Request.
6. Stop capture and restore the device network settings.

The following capabilities support that workflow:

- Rust MITM Runtime with HTTP, HTTPS, and WebSocket Capture Events
- React + TypeScript desktop application delivered by Tauri
- Captured Request history, detail, filtering, breakpoints, replay, and Composer
- Routing Rules for direct, upstream proxy, reject, mapping, and breakpoint
  outcomes
- DNS observations and application-aware traffic attribution
- certificate export and local device-setup server
- HAR and request-code export
- reusable `proxybot-core` crate without GUI dependencies

macOS `pf`, the DNS server, MCP stdio, scripting, the mobile dashboard, and
protocol analysis are Advanced capabilities. TUN/iOS VPN, Android SSL bypass,
AI analysis, and generation/deployment screens are Labs until their complete
user journey is proven. See the [product roadmap](docs/roadmap.md).

Some applications use certificate pinning or platform protections that
intentionally prevent decryption. ProxyBot may still expose DNS or SNI metadata,
but it cannot guarantee payload access.

## Build from source

ProxyBot currently targets macOS. You need:

- Xcode Command Line Tools
- a stable Rust toolchain
- Node.js 20 or newer
- pnpm 10
- the [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/)

```bash
git clone https://github.com/mbpz/proxybot.git
cd proxybot
pnpm install --frozen-lockfile
pnpm tauri dev
```

Use the persistent Capture Session bar in the main window to start or stop
capture and see lifecycle failures. The ProxyBot menu-bar item provides the same
actions and stays synchronized with the window. Open **Setup** to discover the
Mac's active LAN address, start the temporary CA server, and follow the exact
explicit-proxy and verification steps for iOS or Android. Follow the
[getting started guide](docs/getting-started.md) to connect and clean up a device
safely.

The default development build omits the native Frida runtime. Enable live Frida
device and process operations only when working on the Labs SSL-bypass feature:

```bash
pnpm tauri dev --features frida-runtime
```

Create a local release bundle with:

```bash
pnpm build:tauri
```

The bundle step downloads pinned Apktool and Frida Gadget assets for the optional
APK patcher. They are verified against SHA-256 digests in
[`src-tauri/resources/resources.lock`](src-tauri/resources/resources.lock) and
are not stored in Git. Run `pnpm resources:fetch` to prepare an offline bundle,
or `pnpm resources:check` to verify an existing cache without network access.

Release tooling enables the optional `frida-runtime` Cargo feature. The first
such build downloads the matching Frida Core development kit through
`frida-rust`. Core development and CI use `--no-default-features`; live Frida
commands then return an explicit capability error.

## Traffic path

Explicit proxy is the recommended first-capture mode:

```text
Test device --Wi-Fi explicit proxy--> Mac --MITM Runtime--> upstream server
                                         |
                                         +--> Captured Request
```

Advanced `pf` routing and DNS observation add host-level network changes:

```text
Test device --Wi-Fi--> macOS pf --> MITM Runtime --> upstream server
                          |
                          +--> DNS server --> Application Attribution
```

Install and trust the ProxyBot CA only on a test device, and remove it when the
session is finished.

## Repository layout

```text
proxybot-core/   reusable MITM Runtime, certificates, rules, and analysis models
src-tauri/       composition root, desktop Adapters, persistence, and macOS integration
src/             React desktop application and Browser Adapter tests
e2e/             browser-mock Playwright coverage
docs/            current user and architecture documentation
ios/             unsupported historical VPN experiment
scripts/         development and reproducible asset tooling
```

Historical plans under `docs/sdd/` and `docs/superpowers/` are research records,
not a list of supported features.

## Development checks

The local CI command covers formatting, Rust tests and lints, TypeScript checks,
UI tests, and the frontend production build:

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

Playwright currently exercises a BrowserMockAdapter, not a packaged Tauri app.
Passing it does not by itself prove real certificate distribution or capture.

## Contributing and security

Read [CONTRIBUTING.md](CONTRIBUTING.md) before opening a pull request. Use the
issue forms for reproducible bugs and focused feature requests. Report
vulnerabilities privately as described in [SECURITY.md](SECURITY.md), and follow
the [Code of Conduct](CODE_OF_CONDUCT.md) in all project spaces.

ProxyBot is available under the [MIT License](LICENSE). Downloaded Apktool and
Frida assets remain under their respective upstream licenses.
