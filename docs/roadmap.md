# ProxyBot product roadmap

**Status:** active product-convergence plan

**Last reviewed:** 2026-08-04

**Primary user:** a mobile application developer debugging a test device from a Mac

This document is the product and delivery source of truth. Historical plans under
`docs/sdd/` and `docs/superpowers/` explain earlier experiments; they do not prove
that a feature is supported or shipped.

## Product thesis

ProxyBot should be the shortest path from “my test device is behaving strangely”
to a trustworthy Captured Request that a developer can inspect, change, replay,
and export.

The supported workflow is deliberately narrow:

1. Start one Capture Session on macOS.
2. Connect an iOS or Android test device by explicit proxy.
3. Install and trust the local CA when HTTPS inspection is required.
4. Verify one known HTTP request, then one known HTTPS request.
5. Find a Captured Request by device, application, host, method, or status.
6. Inspect, modify, replay, or export it.
7. Stop capture and restore the device's network settings.

The first successful decrypted request matters more than the number of pages,
protocols, generators, or experimental capture modes.

## Product contract

### Core

Core is the supported path and must remain visible, documented, and covered by a
real desktop acceptance test.

| Area | User outcome | Target surface |
| --- | --- | --- |
| Capture | Start, stop, recover, and understand capture state | Persistent Capture Session control |
| Setup | Connect a device, install the CA, verify traffic, and clean up | One guided Device Onboarding flow |
| Inspect | Search and understand Captured Requests | Traffic workspace with contextual DNS and Application Attribution |
| Modify | Apply a Routing Rule or breakpoint safely | Rules and inline breakpoint controls |
| Reproduce | Send a request again or compose a new one | Replay and Composer |
| Share | Export a useful, redacted artifact | HAR and request-code export |
| Configure | Change ports, storage, TLS, and updates | Settings |

### Advanced

Advanced capabilities may be documented after the Core workflow is stable:

- macOS `pf` transparent routing and the built-in DNS server
- MCP stdio Adapter
- Runtime Extension Pipeline and Rhai scripts
- mobile dashboard
- protocol decoding and traffic analysis

Advanced features must not be required for first success.

### Labs

Labs are experiments, not product promises. They should be hidden behind an
explicit opt-in and excluded from the default navigation and release claims:

- Android SSL-bypass and APK patching
- AI analysis
- spec, mock, scaffold, and deployment generation
- overlapping Graph and Topology views until they become one coherent analysis
  workflow

A Labs feature graduates only when it has an owner, a supported user journey, a
failure model, documentation, and a test through its real Adapter.

## Why convergence is required

The current repository contains valuable deep Modules, but the public product
surface does not reflect them consistently:

- The React application still exposes 14 equal-weight navigation items. The
  mounted Layout now provides a shared Capture Session control, but the broader
  first-use journey remains fragmented.
- Device QR setup is separated from the certificate distribution lifecycle and
  depends on hidden ordering across pages.
- The former TUN Implementation could create an interface but had no packet
  forwarding path into the MITM Runtime; it and its unused dependency were
  removed instead of being shipped as a misleading mode.
- No iOS VPN peer or CLI exists in the composition root, so the public product
  and supported documentation do not claim that transport.
- Browser E2E tests use a mock Tauri Adapter; they cannot prove that a packaged
  app starts, captures traffic, or serves a certificate.
- Package metadata, update checking, MCP metadata, tags, and the latest GitHub
  release do not share one version source.
- Release automation hand-builds and ad-hoc-signs an application ZIP instead of
  producing the same signed and notarized Tauri bundle users are asked to run.

These are product-trust problems, not missing-feature problems.

## Lessons from comparable projects

ProxyBot should borrow interaction principles, not copy competitors' breadth.

| Project | Useful lesson | ProxyBot decision |
| --- | --- | --- |
| [mitmproxy](https://github.com/mitmproxy/mitmproxy) | Clear roles for each interface, a short configure/install/verify loop, and advanced capture modes disclosed progressively | Make explicit proxy the default path; move `pf`, DNS, and other modes behind Advanced |
| [HTTP Toolkit](https://github.com/httptoolkit/httptoolkit) | Setup is organized around selecting and intercepting the relevant client, reducing unrelated traffic | Make Device Onboarding and device-scoped filtering part of the main workflow |
| [Proxelar](https://github.com/emanuele-em/proxelar) | A concise quick start, a certificate install page, a smoke-test request, explicit limitations, and reproducible single-binary distribution | Add a deterministic first-capture check and state limitations next to setup |
| [whistle](https://github.com/avwo/whistle) | A rule-first extension model can grow without expanding the basic workflow | Keep Routing Rules and extensions deep, but do not expose plugin complexity during onboarding |
| [Anything Analyzer](https://github.com/DeepLifeStudio/anything-analyzer) | A unified Session gives capture and analysis a coherent boundary | Use one Capture Session concept; do not adopt an all-source or AI-first scope |

Star counts and feature checklists are intentionally omitted: they age quickly
and reward surface area rather than a reliable user journey.

## Recommended execution order

Work proceeds in order. A later stage cannot compensate for an unmet earlier
exit gate.

### P0 — Make first capture truthful and repeatable

1. **Deepen the Capture Session Module.**
   - Give it one Interface for status, prerequisites, start, stop, failure, and
     recovery.
   - Use the same Module from the desktop shell and tray Adapter.
   - The mounted shell, tray status event, and removal of unused Header, Footer,
     AppHeader, and lifecycle wrappers form the first completed tracer slice.
   - Next, move setup prerequisites and failure guidance behind the same Module.
2. **Deepen the Device Onboarding Module.**
   - Own network discovery, certificate distribution, platform instructions,
     QR generation, verification, and cleanup in one place.
   - Lead with explicit proxy mode; introduce `pf` only after the basic path
     succeeds.
   - The Rust-first preparation contract, temporary server lifecycle, mounted
     Setup page, CA-only iOS delivery, Android trust guidance, and removal of
     duplicate Certificate/QR entry points form the completed tracer slice.
   - Next, validate this path on physical iOS and Android devices and preserve
     the results as release evidence.
3. **Quarantine incomplete network experiments.**
   - Remove TUN and iOS VPN from the default UI, release claims, and supported
     documentation until a real packet-forwarding Adapter exists.
   - The unsupported TUN state, commands, dependency, settings control, shutdown
     branch, and disconnected iOS PacketTunnel sample have been removed.
4. **Establish one Release/Install/Update source of truth.**
   - Derive Rust, Tauri, frontend, MCP, tag, and update metadata from one version.
   - Build with the Tauri bundler; add Developer ID signing, notarization,
     stapling, checksums, SBOM, release notes, and an install/start smoke test.
   - Do not advertise Homebrew until a maintained tap and installation check
     exist.
   - `package.json` now owns product version identity; Tauri and the update UI
     consume it directly, while Rust, MCP, Cargo.lock, and tags are gated by one
     consistency tool.
   - The Release workflow now uses the Tauri bundler, requires Developer ID
     signing and notarization, verifies mounted DMGs, and publishes checksums,
     SPDX SBOMs, provenance attestations, and generated notes.
   - This stage remains open until the hosted workflow and install/start checks
     succeed for both published architectures; credential provisioning and
     evidence are tracked in [issue #27](https://github.com/mbpz/proxybot/issues/27).
5. **Add one real desktop acceptance journey.**
   - Launch the packaged app, prepare the CA, start capture, make a local test
     request, observe the Captured Request, stop, and restart.
   - The packaged executable now has an isolated acceptance Adapter that runs
     this journey through the real Tauri composition root, generated CA, HTTPS
     MITM Runtime, Capture Event persistence, and restart lifecycle, then emits
     a machine-readable report without browser mocks, external network, or user
     data.
   - CI runs it from the unsigned Core app bundle; Release runs the same journey
     from each verified DMG before the single publish job. Visible UI interaction
     and physical signed-install evidence remain release exit gates.

**P0 exit gate**

- A new user can obtain the documented build and see a decrypted test request in
  under five minutes without discovering an undocumented prerequisite.
- The main window shows capture state and a recovery-oriented error message.
- Every published version agrees across metadata, UI, MCP, tag, and artifacts.
- No Core documentation describes a Labs feature as shipped.

### P1 — Make the core debugging loop coherent

1. Reduce the default navigation to Capture, Setup, Rules, Replay/Composer, and
   Settings; place DNS, Alerts, and analysis in the context of a Capture Session.
2. Make Captured Request persistence the single query seam for history, desktop,
   MCP, and analysis.
3. Unify the filter language and query semantics across Traffic, export, MCP,
   Graph, Topology, and Alerts.
4. Migrate production UI calls to the generated Desktop Contract Interface;
   remove the shallow `safeInvoke` Adapter that converts failures into `null`.
5. Add redaction-first HAR export and a reproducible issue-report bundle.

**P1 exit gate**

- The default sidebar has no duplicate destination or experimental product.
- The same filter returns the same Captured Requests in every Adapter.
- Tauri command failures are typed product states rather than silent empty data.
- An issue report can include diagnostics without secrets or captured payloads by
  default.

### P2 — Increase reliability and architectural depth

1. Make the running desktop MITM Module own the core runtime, Capture Event
   bridge, breakpoint task, and deterministic shutdown.
2. Keep SQLite Implementation details behind focused Captured Request, Alert,
   Device, and Routing Rule Interfaces; avoid sharing `Mutex<Connection>` with
   Adapters.
3. Keep MCP as a headless Adapter over the same domain Modules. Remove duplicated
   SQL and either implement a real headless Capture Session or delete the claim.
4. Turn the BrowserMockAdapter suite into fast contract coverage and keep a
   smaller real-Tauri acceptance lane for cross-Seam behavior.
5. Define architecture decisions in `docs/adr/` when a change alters a public
   Interface, persistence model, security boundary, or release contract.

**P2 exit gate**

- Stop returns only after all owned tasks and network resources are released.
- Desktop and MCP expose the same persisted facts and terminology.
- Major Interfaces have contract tests and one documented owner.
- Security-sensitive configuration has a non-null CSP and minimal Tauri
  capabilities.

### P3 — Grow an open-source project, not a feature catalogue

1. Publish a changelog, support matrix, compatibility policy, and release
   cadence.
2. Add screenshots or a short first-capture recording only after the P0 journey
   is stable.
3. Maintain good-first-issue and help-wanted work from the Core roadmap.
4. Measure opt-in, privacy-preserving product outcomes: install success, time to
   first Captured Request, setup failure stage, and crash-free Capture Sessions.
5. Graduate one Labs capability at a time based on demonstrated user demand.

**P3 exit gate**

- A release has notes, checksums, a support statement, and a verified upgrade
  path.
- A contributor can select, build, test, and submit a Core issue using only
  repository documentation.
- Roadmap changes reference user evidence or an accepted architecture decision.

## Architecture deepening candidates

These are ordered by user impact and Leverage. They name the Module and Seam to
deepen; detailed Interfaces belong in an accepted design or ADR.

1. Capture Session lifecycle and shell/tray Adapters
2. Device Onboarding and certificate distribution lifecycle
3. Removal of the incomplete TUN/iOS VPN product surface
4. Generated Desktop Contract adoption across the React application
5. MCP as an Adapter over shared domain Modules
6. Release/Install/Update as one reproducible Module
7. Desktop MITM ownership of all child tasks
8. Real user journey as an acceptance-test Interface

## Non-goals for the current cycle

- becoming a general-purpose VPN or packet analyzer
- implementing every transport protocol
- competing with security suites on exploit or evasion breadth
- making AI, code generation, or deployment generation the primary product
- adding Windows or Linux before the macOS first-capture path is reliable
- promising interception of certificate-pinned or platform-protected apps

## Roadmap governance

- “Supported” requires a documented journey, real-Adapter test, maintained owner,
  and failure/cleanup behavior.
- “Done” requires the exit gate to pass on the release artifact, not only unit or
  browser-mock tests.
- Product additions must identify what leaves the default surface or why the new
  complexity belongs in Core.
- Historical plans may explain a decision but cannot override this roadmap,
  `CONTEXT.md`, current code, or release evidence.
