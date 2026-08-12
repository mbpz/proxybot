# ProxyBot Investigation Workbench Convergence Design

**Date:** 2026-08-12  
**Status:** Approved design  
**Stable baseline:** `c492759`  
**Scope:** Product positioning, session/evidence model, investigation workflow, desktop technology convergence, and verifiable release discipline

## 1. Decision

ProxyBot is a macOS MITM investigation workbench for developers debugging iOS and Android test devices.

Its primary user journey is:

```text
Setup -> Capture Session -> Investigate -> Modify/Reproduce -> Redacted Share
```

ProxyBot will not position itself as a general packet analyzer, a security suite, or an AI generation platform. Packet capture breadth, automated analysis, SSL bypass, specification generation, and deployment helpers may remain useful capabilities, but they must not compete with the core investigation journey.

This design adopts a vertical convergence strategy. Each delivery batch must improve an end-to-end user outcome while reducing competing Interfaces. It does not authorize a wholesale rewrite.

## 2. Current problems

### 2.1 Product positioning drift

The written product thesis is sound, but the shipped information architecture overstates maturity:

- Core `Inspect` does not yet expose the complete request, response, timing, protocol, and attribution evidence already available in Rust.
- Core `Modify` currently supports Forward or Drop; request editing is not connected to the runtime.
- Replay and Composer are disconnected from the selected Captured Request.
- HAR export copies sensitive headers and bodies without default redaction.
- DNS, Alerts, Graph, and Topology appear as equal Capture destinations even though they are evidence or projections within an investigation.
- Labs routes are hidden from primary navigation but are not controlled by a common opt-in mechanism.
- A release workflow exists, but hosted CI, signing, notarization, uploaded-asset verification, and physical-device evidence do not yet form a verified release chain.

Product claims must describe verified user outcomes, not the existence of backend commands or unfinished pages.

### 2.2 Competing technical Interfaces

Rust, Tauri, React, and TypeScript are appropriate for the product. The inconsistency comes from multiple Interfaces for the same capability:

- raw Tauri `invoke` and `listen`;
- `safeInvoke`, which converts failures into `null`;
- generated Rust-first DTOs through `DesktopContract`;
- hand-written TypeScript mirrors for Request, Device, Graph, Topology, Replay, and related data;
- multiple meanings of `session_id` across capture, AI generation, and instrumentation;
- independent filtering, selection, loading, and failure behavior in Requests, DNS, Alerts, Graph, and Topology.

The target architecture makes each Seam explicit, gives it one authoritative Interface, and preserves multiple data projections only when they serve genuinely different needs.

### 2.3 Stable-baseline constraint

The uncommitted Alerts Desktop Contract migration is not part of stable capability. It must be completed, verified, and committed independently before broad migration. The design document must not absorb or stage those working-tree changes.

## 3. Goals and non-goals

### Goals

1. Make one persistent CaptureSession the shared context for all captured evidence.
2. Separate immutable observed facts from rebuildable inferred Activities and Findings.
3. Provide one Investigation Workspace for complete Request/Response evidence and contextual analysis.
4. Make `DesktopContract` the only production desktop communication Interface.
5. Centralize projection mappings without forcing every use case into one DTO.
6. Make capability maturity enforceable in navigation, routes, commands, settings, and release claims.
7. Publish only artifacts whose identity, contents, signing, notarization, installation, upgrade, and physical-device path are independently verifiable.

### Non-goals

- Replacing Tauri/React with AppKit or SwiftUI.
- Copying Tracexy's packet five-tuple model, PCAP schema, fixed DNS correlation window, privileged Helper, or macOS multi-window Implementation.
- Adding general VPN, full packet analysis, or broad security scanning to Core.
- Combining every persistence, analysis, wire, and view DTO into one universal type.
- Rewriting all existing pages before one vertical user journey works.
- Treating AI, SSL bypass, Graph, Topology, or code generation as release blockers for Core.

## 4. Product model and capability levels

### 4.1 Product promise

The public promise is limited to capabilities that complete the core journey:

- verifiable device and certificate setup;
- persistent capture sessions;
- full request and response inspection;
- evidence-backed device and application attribution;
- routing rules;
- safe request modification and reproduction;
- redacted export and copying.

### 4.2 Primary destinations

The stable primary destinations remain:

1. **Capture** — the Investigation Workspace.
2. **Setup** — certificate, device, and proxy readiness.
3. **Rules** — routing and mutation rules.
4. **Replay** — replay history and batch operations.
5. **Settings** — application configuration and capability opt-in.

Capture no longer exposes Requests, DNS, Alerts, Graph, and Topology as equal tabs. Their responsibilities move into the Investigation Workspace:

- DNS Observation becomes evidence.
- Alerts become Findings linked to source records.
- Graph and Topology become a Relationships projection.
- single-request replay and Composer entry points live beside selected evidence.

### 4.3 Capability Gate

A single `CapabilityGate` Module classifies and controls capabilities:

| Level | Capabilities | Availability rule |
| --- | --- | --- |
| Core | CaptureSession, Inspector, onboarding, attribution, Routing Rules, modify/reproduce, redacted share | Enabled by default; covered by release acceptance |
| Advanced | `pf`, DNS Server/Upstream, advanced TLS rules, MCP, Rhai/extensions, Mobile Dashboard, batch Replay | Explicit opt-in; prerequisites and risk shown before enablement |
| Labs | anomaly analysis, advanced Relationships, SSL bypass, AI, spec/mock/scaffold/deploy | Explicit opt-in; experimental label; excluded from Core maturity claims |

The gate controls navigation, route resolution, Desktop Contract availability, settings, telemetry labels, and documentation claims. A direct URL or raw command cannot bypass it. Disabled capabilities return a structured `unavailable` error rather than silently rendering an empty view.

## 5. CaptureSession and evidence model

### 5.1 CaptureSession

`CaptureSession` is a persistent investigation identity, not merely the runtime start/stop state.

Required facts include:

- durable UUID and user-visible name;
- start and end timestamps;
- `starting`, `running`, `stopping`, `completed`, or `failed` status;
- selected device and application scope;
- proxy, certificate, network, and enabled-capability snapshot;
- initial filter and capture configuration;
- record and byte statistics;
- failure evidence;
- application version, database schema version, and capture format version.

Invariants:

- At most one local MITM CaptureSession is active.
- Starting first persists a `starting` session. Runtime success transitions it to `running`; startup failure transitions it to `failed` without deleting diagnostic evidence.
- Stop first prevents new work, drains accepted capture events, then closes the session as `completed` or `failed`.
- Captured Requests, DNS Observations, WebSocket Frames, Findings, rule applications, replay lineage, and capture failures belong to exactly one CaptureSession.
- Legacy records without a session are migrated into an explicit legacy-import session; normal queries never use an implicit `Any` scope.
- Cross-session investigation requires an explicit multi-session selection.

Other domains stop borrowing this name. AI work uses `SpecGenerationRun`; Frida work uses `InstrumentationSession`.

### 5.2 Observed evidence

Observed evidence records what ProxyBot directly captured or executed:

- client connection and TLS tunnel;
- HTTP request/response transaction;
- WebSocket upgrade and frame;
- DNS Observation;
- device/application attribution observation;
- Routing Rule match and mutation result;
- Replay parent-child relation;
- capture or decode failure.

Observed evidence is append-only for investigation purposes. Corrections are new records or metadata revisions with provenance, not destructive replacement.

### 5.3 Activity projection

`Activity` groups evidence into a human-meaningful action such as login, token refresh, asset loading, or a WebSocket exchange. It is an inferred, versioned projection and can be deleted and rebuilt without changing observed evidence.

Each Activity contains:

- stable semantic ID;
- CaptureSession ID and anchor record;
- member evidence references;
- display name and optional user override;
- evidence kind and reason for each association;
- time window;
- confidence tier;
- competing candidates and contested state;
- projection algorithm version.

Unknown data remains first-class `ungrouped` evidence. It is not collected into a misleading `Unknown` Activity. The UI always provides `View raw requests` and `Ungroup`. User overrides are stored separately from generated projection output.

The default Activity ID derives from CaptureSession, anchor record, and projection version so refreshes preserve selection and disclosure state. Algorithm changes may deliberately create a new projection version while retaining a trace to the prior result.

### 5.4 ProxyBot-specific correlation

Correlation prioritizes stronger MITM facts over packet heuristics:

1. exact request/response and WebSocket lineage;
2. client connection and TLS tunnel identity;
3. device and application attribution;
4. rule or Replay parent identity;
5. DNS provenance and temporal proximity;
6. weaker name, endpoint, and time similarity.

There is no universal fixed DNS window. Correlation policies are versioned and tested against shared hosting, CDN reuse, redirects, retries, multiplexing, and missing attribution.

## 6. Investigation Workspace

### 6.1 Persisted workspace state

An `InvestigationWorkspace` Module persists:

- CaptureSession scope;
- selected Activity and evidence record;
- grouping mode and sort;
- search and filters;
- saved Focus Set;
- Noise Control rules;
- Inspector and Context Dock visibility, size, and selected section;
- expanded rows and navigation history.

No selection gives the center list the available space. The first selection opens the Inspector. If the user manually closes it, later selections do not force it open. Live updates cannot discard the current selection when its stable ID still exists.

### 6.2 Layout responsibilities

- **Session/Focus area:** sessions, devices, applications, Focus Sets, and Noise Control.
- **Center list:** Activities or raw Requests under one shared query.
- **Evidence Inspector:** what the selected record actually contains.
- **Context Dock:** how the selected record relates to other evidence and inferred conclusions.

This is a macOS-native investigation workflow in behavior—stable selection, resizable panels, keyboard navigation, progressive detail, and state restoration—while retaining the Tauri/React Implementation.

### 6.3 Evidence Inspector

The Inspector loads a complete detail DTO only after selection. Its sections are:

- Request method, URL, headers, cookies, and body;
- Response status, headers, body, and timing;
- DNS, connection, TLS, first-byte, and completion timeline;
- WebSocket handshake, frames, direction, and payload;
- verified gRPC, GraphQL, and future protocol decodes;
- binary/hex representation with decode-range linkage;
- applied rules, before/after mutation, and result.

Evidence-adjacent actions are:

- Edit and Forward;
- Replay;
- Open in Composer;
- Export Redacted;
- Copy with Redaction.

Unsupported decoding is shown as unsupported or unavailable, never as an empty successful result.

### 6.4 Context Dock

The Context Dock contains explanatory and relational projections:

- device and application attribution provenance;
- DNS and TLS provenance;
- Activity evidence, confidence, and competing candidates;
- Findings linked to source records;
- historical baseline and related Requests;
- Relationships among application, host, request, and dependency;
- override, ungroup, and raw-evidence escape hatches.

Every inferred statement links back to observed evidence. A projection failure does not hide or rewrite Request/Response facts.

### 6.5 Focus and Noise Control

The existing Filter DSL becomes the single query language. A Focus Set is a named, persisted query. Noise Control adds view-only exclusion terms to the query and statistics projection; it never deletes SQLite evidence or silently changes export completeness.

## 7. Unified technical architecture

### 7.1 Desktop Contract Seam

The Rust command and event registry is the sole source of truth for desktop communication. Generated artifacts contain TypeScript argument, result, event, and error DTOs.

`DesktopContract.call` and `DesktopContract.subscribe` provide:

- Rust-to-TypeScript parameter-name mapping;
- runtime payload validation;
- structured errors and retry classification;
- cancellation and stale-response protection;
- event lifecycle management.

Only two production/test Adapters satisfy this Interface:

1. the Tauri Adapter at the application composition root;
2. the Browser Mock Adapter used by Vitest and browser E2E.

Production Modules may not import raw Tauri `invoke`, `listen`, or `safeInvoke`. A repository check enforces the rule. Packaged acceptance tests cover the real Tauri serialization and lifecycle that a browser Adapter cannot prove.

### 7.2 Error Interface

`DesktopError` distinguishes at least:

- `validation`;
- `unavailable`;
- `permission`;
- `conflict`;
- `timeout`;
- `cancelled`;
- `persistence`;
- `internal`.

Each error carries a stable code, safe message, diagnostic context, and retryability. Converting an error into `null`, `[]`, or `No findings` is forbidden.

React query state is explicit: `idle`, `loading`, `ready`, `empty`, `stale`, or `error`. Panel failures are isolated: Context failure does not clear evidence; Inspector failure does not clear the list. Selection changes cancel or supersede prior work.

### 7.3 Captured Request projections

Projection types remain distinct where their requirements differ:

- `CapturedRequestRecord` — Persistence Adapter internal representation;
- `CapturedRequestAnalysis` — immutable analysis facts;
- `CapturedRequestListItem` — lightweight list wire DTO;
- `CapturedRequestDetail` — complete Inspector wire DTO;
- React view models — local to their owning view.

A deep `CapturedRequestProjection` Module owns all Record-to-Analysis/List/Detail/Wire mappings and invariants. Tests cover response preservation, binary bodies, timestamps, duration, nullability, invalid encodings, gRPC/GraphQL metadata, WebSocket identity, and attribution provenance.

Device, DNS, Graph, Topology, Replay, and instrumentation wire DTOs are also Rust-first generated. A caller cannot assert that Graph and DAG payloads are equivalent. Separate algorithm Implementations may map through Adapters into an explicit common investigation projection only when their semantics match.

### 7.4 Investigation query and projections

All investigation views consume one `InvestigationQuery` Interface:

```text
InvestigationQuery
  sessionScope
  timeWindow
  deviceIds
  applications
  protocols
  text
  focusSet
  noiseRules
```

The query produces independently loadable projections:

- `ActivityProjection` — evidence grouping, confidence, and competing candidates;
- `FindingProjection` — anomaly or security findings with source references;
- `RelationshipProjection` — request, application, host, and dependency relations;
- list, detail, and statistics projections.

Projection version and freshness are visible. Derived results are rebuildable; source evidence is not.

### 7.5 Locality and Leverage

The target deep Modules are:

- `DesktopContract`: callers learn one call/event/error Interface while serialization and Adapter complexity stays local.
- `CaptureSession`: callers learn one lifecycle and scope Interface while runtime, persistence, recovery, and statistics stay local.
- `CapturedRequestProjection`: callers select an appropriate projection while mapping and invariants stay local.
- `InvestigationWorkspace`: views share selection, query, panel, and restoration behavior without duplicating it.
- `CapabilityGate`: product maturity policy is enforced once across UI, command, and release surfaces.
- `BuildFlavor`: local, CI, and release invoke one authoritative build definition.

Each Seam has at least two meaningful Adapters or consumers, so the Interface is a real test surface rather than hypothetical indirection.

## 8. Modify, reproduce, and share

### 8.1 Modify

Core `Modify` is not complete until the user can edit allowed method, URL, headers, and body fields, preview the difference, and forward the mutated request through the real runtime Adapter.

Safety requirements:

- preserve the original observed evidence;
- store the mutation as a child operation with actor, time, and before/after difference;
- validate protected or protocol-dependent fields;
- show whether forwarding succeeded, timed out, or was cancelled;
- cover the real mutation payload with Contract and packaged acceptance tests.

Until then, product text says Forward/Drop rather than edit-and-forward.

### 8.2 Reproduce

The selected Captured Request is the source of truth for single-request reproduction:

- Replay runs an unchanged or explicitly edited child request and records lineage.
- Open in Composer pre-populates the verified method, URL, headers, and body.
- The Replay destination owns history, comparison, and batch operations.

Replay results display request differences, response differences, timing, and errors. Local mock replay must not be represented as a successful upstream reproduction.

### 8.3 Redacted Share

Redaction is default-on for export and copying. The policy covers Authorization, Cookie/Set-Cookie, common token and secret headers, query parameters, JSON/form fields, and configured custom patterns.

Before writing an artifact, ProxyBot shows:

- selected Session and scope;
- included record count;
- applied redaction policy version;
- removed or replaced field summary;
- warnings for unknown binary or unsupported content.

Raw export is an explicit Advanced action with a separate warning and confirmation. Redaction tests use hostile nested payloads, mixed casing, duplicate headers, encoded secrets, binary bodies, and partial decode failures.

## 9. Verifiable release discipline

### 9.1 Release states

- **Development Build:** local or PR checks passed; not distributable proof.
- **Release Candidate:** signed, notarized artifact uploaded to a Draft Release; asset re-verification is pending.
- **Verified Release:** the uploaded asset was independently downloaded and passed identity, installation, upgrade, updater, and physical-device gates.

Only the third state is described as a formal release in README, the updater, and GitHub Releases.

### 9.2 BuildFlavor Module

One `BuildFlavor` Module defines:

| Flavor | Purpose | Required content |
| --- | --- | --- |
| `core-ci` | fast correctness gate | Contract, Rust, TypeScript, unit and integration tests |
| `desktop-smoke` | release-like unsigned package | production Tauri configuration and bundle resources |
| `release` | distributable candidate | complete resources, release features, signing, notarization, updater config |

Local scripts, CI, and release workflows call these definitions rather than recreate their command lists. pnpm has one version authority through `packageManager` and Corepack. Obsolete Yew/wasm build paths are removed after verifying they have no supported consumer.

### 9.3 PR and tag gates

Pull requests run:

- Desktop Contract generation consistency and bypass prohibition;
- Rust formatting, linting, tests, and database migration tests;
- TypeScript checks, Vitest, and Browser Mock Adapter tests;
- wire serialization and projection invariant tests;
- CaptureSession lifecycle and recovery tests;
- unsigned `desktop-smoke` packaging;
- retained reports and safe diagnostics.

Tag builds add:

- offline verification of all locked bundle resources;
- the complete release feature set, including the declared Frida mode;
- Developer ID signing and notarization;
- DMG, checksum, SBOM, provenance, and release manifest;
- packaged acceptance and updater compatibility checks.

### 9.4 Uploaded-asset verification

Release assets first enter a Draft Release. CI then downloads the exact GitHub asset and verifies:

- asset length and SHA-256;
- `codesign --verify --deep --strict`;
- `spctl` assessment;
- `stapler validate`;
- mount, install, first launch, and database migration;
- upgrade from the previous verified version;
- updater download and install;
- core capture-path smoke.

Physical-device gates record one supported iOS and one supported Android first HTTPS capture. The evidence records operator, device model, OS version, commit, artifact SHA-256, and results for certificate setup, proxy setup, Request/Response inspection, and Session shutdown. A failed or missing gate leaves the Release in Draft.

### 9.5 Release manifest

The machine-readable manifest records:

- git SHA, product version, target, and build time;
- Rust, Node, pnpm, and Tauri versions;
- BuildFlavor, enabled features, and database schema;
- `resources.lock` digest and Frida/apktool versions;
- artifact length and SHA-256;
- SBOM and provenance references;
- signing Team, notary request ID, and staple result;
- CI run, packaged acceptance, and physical-device evidence;
- updater channel and minimum compatible version.

The same checksum, identity, version, and provenance rules apply to Frida, apktool, sidecars, and other bundled resources. ProxyBot does not introduce a privileged Helper merely to resemble Tracexy.

## 10. Verification strategy

### 10.1 Module tests

- CaptureSession transition table, crash recovery, drain ordering, and legacy import.
- Activity determinism, stable identity, contested attribution, ungrouped evidence, and override preservation.
- CapturedRequestProjection information preservation and invalid-data handling.
- Redaction policy adversarial fixtures.
- BuildFlavor and release manifest completeness.

### 10.2 Interface tests

- Generated Desktop Contract matches every registered production command and event.
- Tauri and Browser Mock Adapters satisfy identical success and failure fixtures.
- Graph/DAG payloads cannot pass through an incorrect DTO.
- CapabilityGate rejects disabled navigation, direct routes, and commands consistently.
- Query projections return the same scope semantics for Requests, Activities, Findings, Relationships, and statistics.

### 10.3 Workflow tests

- Start Session, capture HTTPS, inspect full Request/Response, stop, reopen, and retain selection/query state.
- Select a request, edit and forward, observe immutable original plus child mutation result.
- Open a request in Composer, Replay it, and compare lineage and responses.
- Export a Session and prove default redaction.
- Fail Inspector or Context loading independently and verify evidence remains accessible.
- Disable a Labs capability and verify its route and command fail explicitly.

Browser tests prove UI behavior through the shared Adapter. Packaged acceptance proves real Tauri serialization, lifecycle, bundle contents, and system integration. Physical devices prove the final trust path.

## 11. Delivery sequence

This design is intentionally decomposed into independently reviewable vertical batches. Each batch receives its own implementation plan and commit series.

### Batch 0 — Stabilize the current baseline

1. Complete and verify the existing Alerts Desktop Contract migration.
2. Commit it without mixing later investigation work.
3. Fix hosted pnpm setup and make the existing CI gates execute.
4. Correct public maturity language while release evidence remains incomplete.

### Batch 1 — One desktop communication Seam

1. Expand the Rust-first registry to all production commands and events.
2. Remove raw production calls and `safeInvoke` slice by slice.
3. Reuse Browser Mock Adapter fixtures across Vitest and browser E2E.
4. Fix the Graph/DAG wire DTO correctness defect before UI consolidation.

### Batch 2 — Persistent CaptureSession

1. Add schema, lifecycle, recovery, and explicit SessionScope.
2. Attach all new evidence to one Session.
3. Migrate legacy unscoped records explicitly.
4. Rename AI and instrumentation session concepts.

### Batch 3 — Complete evidence inspection

1. Introduce list/detail projections and centralized mappings.
2. Deliver Request, Response, Timeline, WebSocket, Protocol, Binary, and Applied Rules sections.
3. Persist workspace selection, panels, query, and Focus Sets.

### Batch 4 — Complete the core outcome

1. Connect safe edit-and-forward.
2. Connect selected Request to Replay and Composer.
3. Implement default redacted copy and export.
4. Validate the full journey through packaged acceptance.

### Batch 5 — Activity and contextual analysis

1. Add versioned Activity projection, evidence, confidence, and ungrouped behavior.
2. Move DNS into evidence and Alerts into Findings.
3. Merge Graph, Topology, and Auth presentation into Relationships.
4. Introduce the unified Context Dock.

### Batch 6 — Capability and release convergence

1. Enforce CapabilityGate across navigation, routes, Contract, and settings.
2. Unify BuildFlavor, resource locking, release manifest, SBOM, and provenance.
3. Add uploaded-asset verification and updater smoke.
4. Complete recorded iOS and Android physical-device gates before a Verified Release.

Dependencies are deliberate: Batch 1 enables reliable later migration; Batch 2 supplies the investigation identity; Batches 3 and 4 complete the Core journey; Batch 5 adds inference only after facts are trustworthy; Batch 6 prevents experimental capabilities and unverifiable artifacts from diluting the result.

## 12. Acceptance criteria

The convergence is successful when all of the following are true:

1. A developer can connect a supported iOS or Android test device and start a persistent CaptureSession.
2. Every new Request, Response, DNS Observation, WebSocket Frame, rule result, Finding, and failure has an explicit Session identity.
3. The developer can inspect complete Request/Response evidence, attribution provenance, timing, and applicable protocol data without changing pages.
4. The developer can edit or reproduce a selected Request while preserving the immutable original and operation lineage.
5. Sharing is redacted by default and reports its policy and omissions.
6. Activity and Finding inference always links to evidence, exposes confidence and contention, and leaves uncertain data ungrouped.
7. Production React code uses only the generated Desktop Contract; failures never become false empty states.
8. Core, Advanced, and Labs availability is enforced consistently and reflected accurately in product claims.
9. A formal release is a re-downloaded GitHub asset that passed checksum, signing, notarization, installation, upgrade, updater, packaged acceptance, and recorded iOS/Android gates.
10. Existing uncommitted work is preserved and every batch is independently reviewable and revertible.

## 13. Tracexy lessons retained—and rejected

ProxyBot retains three principles from Tracexy:

- observed facts and inferred Activities have different types and lifecycles;
- investigation state and semantic identity remain stable while live data changes;
- release claims are backed by evidence that can be independently repeated against the shipped asset.

ProxyBot explicitly rejects copying Tracexy's packet-oriented data plane, AppKit Implementation, fixed packet/DNS heuristics, or privileged Helper architecture. The adaptation is built around HTTP transactions, WebSockets, mobile-device attribution, rules, replay lineage, Tauri/React, and ProxyBot's actual release supply chain.
