# ProxyBot

[![CI](https://github.com/mbpz/proxybot/actions/workflows/ci.yml/badge.svg)](https://github.com/mbpz/proxybot/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Documentation](https://img.shields.io/badge/docs-GitHub%20Pages-blue.svg)](https://mbpz.github.io/proxybot/)

ProxyBot is a **lightweight macOS MITM traffic debugger** for mobile application
developers. An iOS or Android test device points its Wi-Fi HTTP proxy at a Mac
running ProxyBot; the app terminates TLS with a locally generated CA, then lets
you inspect, filter, modify, replay, and export HTTP, HTTPS, and WebSocket
Captured Requests.

It is deliberately narrow: one explicit-proxy workflow, a small supported
surface, and no bundled native toolchains. See
[docs/roadmap.md](docs/roadmap.md) for the product tiers.

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

ProxyBot is built around one dependable workflow:

1. Start a Capture Session on a Mac.
2. Connect a test device with an explicit proxy.
3. Install and trust the local CA when HTTPS inspection is needed.
4. Verify a known request.
5. Inspect, filter, modify, replay, or export the Captured Request.
6. Stop capture and restore the device network settings.

The default shell has three Product Destinations — **Capture**, **Setup**, and
**Settings**. Request-derived tools stay one click away inside the Capture
context: Requests, Rules, Replay, DNS, and Alerts, plus Analysis when Labs are
enabled. Replay and Composer share the Replay context. Existing deep links are
preserved.

The following capabilities support that workflow:

- Rust MITM Runtime with HTTP, HTTPS, and WebSocket Capture Events
- React + TypeScript desktop application delivered by Tauri
- Captured Request history, detail, filtering, breakpoints, replay, and Composer
- Routing Rules for direct, upstream proxy, reject, mapping, and breakpoint
  outcomes
- Request detail actions: replay, add breakpoint rule from request, copy as
  cURL, and export the session HAR
- DNS observations and application-aware traffic attribution
- certificate export and a local device-setup server
- reusable `proxybot-core` crate without GUI dependencies

macOS `pf`, the DNS server, MCP stdio, scripting, the mobile dashboard, and
protocol analysis are **Advanced** capabilities. **Labs** — AI analysis,
Generate, Deploy, and the Graph/Topology analysis views — are off by default.
Enable them under **Settings → Labs** (stored locally as
`proxybot.labs.enabled`); after that they are reachable from Settings and the
command palette (⌘K). Opening a Labs route while Labs are off shows a
"Labs are off" page instead of the experiment.

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

Source builds are self-contained: no second-stage downloads, no bundled native
toolchain, and no extra resource fetching step. The default build already
includes everything the app needs.

Use the persistent Capture Session bar in the main window to start or stop
capture and see lifecycle failures. The ProxyBot menu-bar item provides the same
actions and stays synchronized with the window. Open **Setup** to discover the
Mac's active LAN address, start the temporary CA server, and follow the explicit
proxy and verification steps for iOS or Android; the same page links to the CA
and registered-device panels. Follow the
[getting started guide](docs/getting-started.md) to connect and clean up a
device safely.

Create a local application bundle with:

```bash
pnpm build:tauri
```

A local bundle is unsigned and not notarized. It is useful for development only;
it is not evidence that a signed release exists. See
[docs/releasing.md](docs/releasing.md).

## Traffic path

Explicit proxy is the supported first-capture mode:

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
src-tauri/       composition root, desktop Adapters, persistence, MCP, and macOS integration
src/             React desktop application, generated Desktop Contract, and adapter tests
e2e/             browser-mock Playwright coverage
docs/            current docs, adr/, agents/, parity/, history/, status/, zh/
design/          original Pencil (.pen) design sources (not bundled)
scripts/         development, version, and parity tooling
```

Repository governance lives in [CONTRIBUTING.md](CONTRIBUTING.md),
[SECURITY.md](SECURITY.md), [SUPPORT.md](SUPPORT.md),
[CHANGELOG.md](CHANGELOG.md), [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md), and
[.github/CODEOWNERS](.github/CODEOWNERS). `docs/history/` is archived research,
not a list of supported features; `docs/status/` holds one-off status snapshots.

## Development checks

The local CI command covers version/workflow/parity checks, formatting, Rust
tests and lints, the generated contract, TypeScript checks, UI tests, and the
frontend production build:

```bash
pnpm ci:local
```

Individual checks are also available:

```bash
pnpm version:check
pnpm test:workflow-config
pnpm parity:check
cargo test --workspace --locked --no-default-features
cargo clippy --workspace --all-targets --locked --no-default-features -- -D warnings
cargo fmt --all -- --check
pnpm contract:check
pnpm typecheck
pnpm test:ui
pnpm test:e2e
pnpm build
```

Playwright exercises a BrowserMockAdapter in a browser, not a packaged Tauri
app. Passing it does not by itself prove real certificate distribution or
capture. The isolated packaged-process journey uses the real Tauri composition
root, generated CA, HTTPS MITM Runtime, SQLite persistence, and stop/restart
lifecycle:

```bash
pnpm exec tauri build --bundles app
pnpm test:desktop:acceptance
```

It writes its report inside a temporary workspace and removes that workspace
after validation; it does not use external network services or user data. It
proves the backend journey only — it does not drive visible UI controls or prove
a signed installation.

`package.json` is the canonical product version. Use `pnpm version:set <version>`
to update its checked Rust mirrors, and follow the
[release runbook](docs/releasing.md). Public artifacts must come from the hosted
Tauri bundler workflow; local bundles are not evidence of signing or
notarization.

## Documentation

- [Getting started](docs/getting-started.md) — first capture end to end.
- [Architecture](docs/architecture.md) — runtime composition and adapter seams.
- [Roadmap](docs/roadmap.md) — product tiers and current work.
- [ADRs](docs/adr/README.md) — binding architecture and product decisions.
- [Changelog](CHANGELOG.md) — released and unreleased changes.
- [Support](SUPPORT.md) — supported configuration, version policy, and help.

## Contributing and security

Read [CONTRIBUTING.md](CONTRIBUTING.md) before opening a pull request. Use the
issue forms for reproducible bugs and focused feature requests. Report
vulnerabilities privately as described in [SECURITY.md](SECURITY.md), and follow
the [Code of Conduct](CODE_OF_CONDUCT.md) in all project spaces.

ProxyBot is available under the [MIT License](LICENSE).
