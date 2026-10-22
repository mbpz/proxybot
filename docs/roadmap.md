# ProxyBot product roadmap

**Status:** active product-convergence plan (post-lightweight convergence)

**Last reviewed:** 2026-10-22

**Primary user:** a mobile application developer debugging a test device from a Mac

This document is the product and delivery source of truth. Historical plans
under `docs/history/sdd/` and `docs/history/superpowers/` explain earlier
experiments; they do not prove that a feature is supported or shipped.

The 1.3.0 line is a lightweight convergence pass. The decisions that removed
the native toolchain and collapsed the default navigation are
recorded in [ADR-0007](adr/0007-lightweight-positioning.md) and
[ADR-0005](adr/0005-product-tiers.md); the pre-lightweight state is archived at
tag `archive/pre-lightweight-2026-10-22`.

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

Core is the supported path and remains visible, documented, and covered by a
real desktop acceptance test. The shell exposes exactly three Product
Destinations: **Capture**, **Setup**, and **Settings**. Request-derived tools
live inside the Capture context instead of competing as peers.

| Area | User outcome | Target surface |
| --- | --- | --- |
| Capture | Start, stop, recover, and understand capture state | Persistent Capture Session control; Capture destination |
| Setup | Connect a device, install the CA, verify traffic, and clean up | One guided Device Onboarding flow; Setup destination |
| Inspect | Search and understand Captured Requests | Traffic workspace with contextual DNS and Application Attribution |
| Modify | Apply a Routing Rule or breakpoint safely | Rules and inline breakpoint controls (Capture context) |
| Reproduce | Send a request again or compose a new one | Replay and Composer (Replay context) |
| Share | Export a useful, redacted artifact | HAR and request-code export; request detail actions |
| Configure | Change ports, storage, TLS, updates, and Labs opt-in | Settings destination |

### Advanced

Advanced capabilities are documented after the Core workflow and are never
required for first success:

- macOS `pf` transparent routing and the built-in DNS server
- MCP stdio Adapter
- Runtime Extension Pipeline and Rhai scripts
- mobile dashboard
- protocol decoding and traffic analysis

### Labs

Labs are experiments, not product promises. They are off by default, hidden from
the default navigation, and excluded from release claims. Users opt in under
**Settings → Labs** (local `localStorage` key `proxybot.labs.enabled`); after
that they are reachable from Settings and the command palette (⌘K). Opening a
Labs route while Labs are off renders a single "Labs are off" page.

Current Labs:

- AI analysis (model/token usage inspection)
- Generate (spec, mock, scaffold) and Deploy generation screens
- Graph and Topology analysis views until they become one coherent analysis
  workflow

A removed capability is not reintroduced without a new ADR, a supported user
journey, a failure model, and a real-adapter test. The 1.3.0 lightweight pass
removed one such experimental toolchain; the record is ADR-0007.

A Labs feature graduates only when it has an owner, a supported user journey, a
failure model, documentation, and a test through its real Adapter.

## Lightweight positioning

ProxyBot competes on a dependable macOS first-capture workflow, not on feature
breadth. The 1.3.0 convergence makes that concrete:

- **No bundled native toolchain.** Building and running ProxyBot requires no
  native runtime, native binaries, or second-stage resource downloads.
  A clean `pnpm install` is the whole dependency story.
- **No downloaded artifacts at build or bundle time.** The Tauri bundle is built
  from the repository; release assets are the signed DMG, its checksum, and an
  SBOM.
- **Small default surface.** Three destinations; request-derived tools stay in
  the Capture context. Labs-only dependencies (mermaid, vis-network, recharts,
  highlight.js) are code-split and lazy-loaded, so the initial shell chunk stays
  small (about 467 kB after the 1.3.0 split, down from roughly 1.9 MB).
- **Smaller binaries and dependency graph.** The 1.3.0 cleanup removed the
  native toolchain stack and unreachable placeholder modules, reducing the direct
  Rust dependency set and the compiled source tree.

Not in scope for the lightweight product: being a browser-level automation or
DevTools replacement, a general-purpose packet analyzer, a VPN, or an
AI-first traffic platform.

## Current status of the convergence

Resolved in 1.3.0:

- The default navigation is the three Core destinations; Rules, Replay, DNS,
  and Alerts moved into the Capture context (Analysis joins it only when Labs
  are enabled), and deep links remain valid.
- Device onboarding is one flow: LAN discovery, temporary CA server, explicit
  proxy, platform instructions, QR, verification, and cleanup live in Setup;
  the CA and registered-device panels are reached from Setup cards.
- The unsupported TUN mode and disconnected iOS VPN experiment were removed.
- Package metadata, update checking, MCP metadata, Tauri, Rust versions, and
  release tags derive from one Product Version.
- Release automation uses the Tauri bundler with Developer ID signing,
  notarization, checksums, SBOMs, provenance attestations, and the packaged
  desktop acceptance journey.
- One experimental native toolchain and its resource pipeline were removed
  (ADR-0007).
- Documentation converged: ADRs, CHANGELOG, SUPPORT, CODEOWNERS, archived
  history excluded from the site, and a trimmed repository guide.

Still open:

- The signed, notarized, install-verified release has not been produced. It is
  blocked on provisioning Apple credentials
  ([issue #27](https://github.com/mbpz/proxybot/issues/27)) and on physical
  iOS/Android validation evidence
  ([issue #28](https://github.com/mbpz/proxybot/issues/28)).
- Desktop Contract adoption is partial: 36 of 123 desktop commands are typed.
- Tauri security debt remains: `withGlobalTauri: true` and `"csp": null`.
- The packaged acceptance lane proves the backend journey only; visible UI
  interaction and signed-install evidence remain release gates.
- Browser e2e tests still use the mock adapter.

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

## Rockxy Community compatibility program

The [design](https://github.com/mbpz/proxybot/blob/main/docs/history/superpowers/specs/2026-08-19-rockxy-community-parity-design.md),
[master milestone plan](https://github.com/mbpz/proxybot/blob/main/docs/history/superpowers/plans/2026-08-19-rockxy-community-parity-milestones.md),
[evidence ledger](https://github.com/mbpz/proxybot/blob/main/docs/parity/rockxy-community-evidence.md), and
[machine-readable matrix](https://github.com/mbpz/proxybot/blob/main/docs/parity/rockxy-community-matrix.md) define the Rockxy
Community compatibility program. It is pinned to
`RockxyApp/Rockxy@6a676d631820b577cf3a651c78d856733a7df995` and is strictly a
clean-room, public-Community-only effort.

The matrix is the authority for capability evidence, status, owner, and
acceptance; this roadmap remains the authority for product priority. Feature-count
parity cannot waive ProxyBot's first-success, redaction, security, or release
gates.

## Recommended execution order

Work proceeds in order. A later stage cannot compensate for an unmet earlier
exit gate.

### P0 — Make first capture truthful and repeatable

Status: the local journey is in place; the release evidence gate is still open.

1. **Capture Session Module.**
   - One interface for status, prerequisites, start, stop, failure, and
     recovery, shared by the desktop shell and tray adapter.
   - Completed tracer slice: mounted shell, tray status events, and removal of
     unused layout wrappers.
   - Remaining: move setup prerequisites and failure guidance behind the same
     module, and make the Rust side own the Capture Event bridge and breakpoint
     task.
2. **Device Onboarding Module.**
   - Completed: network discovery, temporary certificate distribution, Setup
     page with explicit-proxy instructions, CA-only iOS delivery, Android trust
     guidance, and removal of duplicate certificate/QR entry points.
   - Remaining: validate the path on physical iOS and Android devices and keep
     the results as release evidence.
3. **Quarantine incomplete network experiments.**
   - Completed: the unsupported TUN mode and the disconnected iOS VPN sample
     were removed from code, settings, and documentation.
4. **One Release/Install/Update source of truth.**
   - Completed: `package.json` owns the Product Version; Tauri, the update UI,
     Rust crates, Cargo.lock, MCP metadata, and tags are checked by one tool;
     the workflow uses the Tauri bundler with signing, notarization, checksums,
     SBOMs, provenance, and generated notes.
   - Remaining: run the hosted workflow successfully for both architectures and
     record install/start evidence; credential provisioning is tracked in
     [issue #27](https://github.com/mbpz/proxybot/issues/27).
5. **One real desktop acceptance journey.**
   - Completed: the packaged executable runs the isolated acceptance journey
     through the real composition root; CI runs it from the unsigned app bundle
     and Release runs it from each verified DMG.
   - Remaining: visible UI interaction and signed-install evidence.

**P0 exit gate**

- A new user can obtain the documented build and see a decrypted test request in
  under five minutes without discovering an undocumented prerequisite.
- The main window shows capture state and a recovery-oriented error message.
- Every published version agrees across metadata, UI, MCP, tag, and artifacts.
- No Core documentation describes a Labs feature as shipped.

### P1 — Make the core debugging loop coherent

1. Default navigation convergence.
   - Completed: three Product Destinations (Capture, Setup, Settings);
     Requests, Rules, Replay, DNS, and Alerts live in the Capture context;
     Analysis joins it only when Labs are enabled; Replay and Composer share
     the Replay context; all deep links remain valid.
2. Make Captured Request persistence the single query seam for history, desktop,
   MCP, and analysis.
3. Unify the filter language and query semantics across Traffic, export, MCP,
   Graph, Topology, and Alerts.
4. Migrate production UI calls to the generated Desktop Contract.
   - Completed: the shallow `safeInvoke` adapter that converted failures into
     `null` is gone; typed contract adoption reached General Settings, DNS,
     Alerts, and the capture session.
   - Remaining: 36 of 123 desktop commands are typed; the rest still use direct
     `invoke` and need typed errors.
5. Add redaction-first HAR export and a reproducible issue-report bundle.

**P1 exit gate**

- The default sidebar has no duplicate destination or experimental product.
- The same filter returns the same Captured Requests in every Adapter.
- Tauri command failures are typed product states rather than silent empty data.
- An issue report can include diagnostics without secrets or captured payloads by
  default.

### P2 — Increase reliability and architectural depth

1. Make the running desktop MITM module own the core runtime, Capture Event
   bridge, breakpoint task, and deterministic shutdown.
2. Keep SQLite implementation details behind focused Captured Request, Alert,
   Device, and Routing Rule interfaces; avoid sharing `Mutex<Connection>` with
   adapters.
3. Keep MCP as a headless adapter over the same domain modules. Remove duplicated
   SQL and either implement a real headless Capture Session or delete the claim.
4. Turn the BrowserMockAdapter suite into fast contract coverage and keep a
   smaller real-Tauri acceptance lane for cross-seam behavior.
5. Define architecture decisions in `docs/adr/` when a change alters a public
   interface, persistence model, security boundary, or release contract.
   - Completed: ADR-0001 through ADR-0008 are accepted and indexed.

**P2 exit gate**

- Stop returns only after all owned tasks and network resources are released.
- Desktop and MCP expose the same persisted facts and terminology.
- Major interfaces have contract tests and one documented owner.
- Security-sensitive configuration has a non-null CSP and minimal Tauri
  capabilities.

### P3 — Grow an open-source project, not a feature catalogue

1. Publish a changelog, support matrix, compatibility policy, and release
   cadence.
   - Completed: `CHANGELOG.md`, `SUPPORT.md`, `CONTRIBUTING.md`, `SECURITY.md`,
     `CODEOWNERS`, and the ADR set exist.
   - Remaining: repository description/topics and a verified release upgrade
     path.
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

Ordered by user impact and leverage. They name the module and seam to deepen;
detailed interfaces belong in an accepted design or ADR.

1. Capture Session lifecycle and shell/tray adapters — partially deepened
2. Device Onboarding and certificate distribution lifecycle — completed for the
   local journey; physical-device evidence pending
3. Removal of the incomplete TUN/iOS VPN product surface — completed
4. Generated Desktop Contract adoption across the React application — in progress
5. MCP as an adapter over shared domain modules — pending
6. Release/Install/Update as one reproducible module — workflow completed,
   hosted evidence pending
7. Desktop MITM ownership of all child tasks — pending
8. Real user journey as an acceptance-test interface — backend journey completed

## Non-goals for the current cycle

- becoming a general-purpose VPN, packet analyzer, or browser-level automation /
  DevTools replacement
- implementing every transport protocol
- competing with security suites on exploit or evasion breadth
- making AI, code generation, or deployment generation the primary product
- reintroducing a removed experimental native toolchain (see ADR-0007)
- adding Windows or Linux before the macOS first-capture path is reliable
- promising interception of certificate-pinned or platform-protected apps
- bundling or downloading native toolchains to unlock a feature

## Roadmap governance

- “Supported” requires a documented journey, real-adapter test, maintained
  owner, and failure/cleanup behavior.
- “Done” requires the exit gate to pass on the release artifact, not only unit
  or browser-mock tests.
- Product additions must identify what leaves the default surface or why the new
  complexity belongs in Core.
- Historical plans may explain a decision but cannot override this roadmap,
  `CONTEXT.md`, current code, or release evidence.
- A Labs capability is never advertised in a release note or the default
  navigation before it graduates.
