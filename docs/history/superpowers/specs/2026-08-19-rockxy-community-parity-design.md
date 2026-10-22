# ProxyBot Rockxy Community Behavioral Parity Design

**Date:** 2026-08-19
**Status:** Approved direction; implementation requires milestone plans
**ProxyBot baseline:** `a3596c777335eaca5c323a445b76a4e127704bf2`
**Rockxy reference snapshot:** `RockxyApp/Rockxy@6a676d631820b577cf3a651c78d856733a7df995`
**Related design:** `docs/superpowers/specs/2026-08-12-investigation-workbench-convergence-design.md`

## 1. Decision

ProxyBot will reach behavioral and user-journey parity with the verifiable public
Rockxy Community source edition while keeping the ProxyBot product identity,
MIT license, Rust MITM Runtime, Tauri desktop shell, and React/TypeScript UI.

Parity is clean-room behavioral parity. ProxyBot may study public documentation,
public interfaces, tests, and observable behavior, but it must not copy or adapt
AGPL implementation code, private downstream code, Rockxy trademarks, images,
copy, icons, or other branded assets.

Delivery uses three ordered phases:

1. **A — Core journey depth:** finish the dependable Setup -> Capture Session ->
   Investigate -> Modify/Reproduce -> Redacted Share path.
2. **B — Community feature breadth:** add every remaining verifiable public
   Community capability behind the appropriate Core, Advanced, or Labs gate.
3. **C — Historical hardening:** replay Rockxy's public release history as a
   regression catalogue, close behavioral gaps, and verify release artifacts.

Phase B cannot excuse a missing Phase A exit gate. Phase C cannot become a
pixel-copy exercise or an excuse to import AGPL source.

## 2. Reference and legal boundary

The parity target is the public Community repository at the fixed snapshot above:

- repository: <https://github.com/RockxyApp/Rockxy>
- fixed commit: <https://github.com/RockxyApp/Rockxy/commit/6a676d631820b577cf3a651c78d856733a7df995>
- public source license: AGPL-3.0-or-later
- public README statement: official signed downloads can include non-public
  downstream components and are not represented as reproducible from the public
  repository alone

Consequences:

- Official-DMG-only or paid/private behavior is outside the required parity
  scope unless equivalent behavior is independently specified from a public,
  verifiable source and separately approved.
- README "Future Work" items are not current parity requirements. They may enter
  a later roadmap only after Rockxy ships them publicly or ProxyBot independently
  chooses them.
- A target feature is not considered verified only because a README names it.
  Source, tests, public docs, or an observable Community build path must support
  the claim.
- Every parity task records the Rockxy evidence source and the independently
  authored ProxyBot acceptance criteria.
- Contributors must not paste Rockxy code into issues, tests, fixtures, prompts,
  comments, or commits.

## 3. Definition of parity

### 3.1 Behavioral parity

A capability is behaviorally equivalent when a ProxyBot user can complete the
same developer outcome with equivalent inputs, visible state, safety properties,
failure recovery, persistence, and export semantics. Names, layout, technology,
and implementation details may differ.

### 3.2 Completion evidence

Each capability requires all applicable evidence:

1. a domain Interface and ownership boundary;
2. persistence or explicit ephemeral semantics;
3. generated Desktop Contract commands, events, DTOs, and structured errors;
4. a reachable UI journey with loading, empty, stale, unavailable, and failure
   states;
5. unit and contract tests, including invalid payload and failed-operation tests;
6. BrowserMockAdapter coverage for fast UI behavior;
7. real Tauri or packaged acceptance coverage for every cross-process seam;
8. user documentation, limitations, cleanup, and safety notes;
9. capability-level release evidence when the feature changes networking,
   certificate trust, credentials, captured data, or public artifacts.

A component, command, route, or fixture by itself is not parity.

### 3.3 Capability maturity

| Level | Meaning | Default exposure |
| --- | --- | --- |
| Core | Required for the primary debugging journey and release acceptance | Enabled and documented |
| Advanced | Complete but optional behavior with extra prerequisites or risk | Explicit opt-in |
| Labs | Verifiable experiment whose limits are visible | Explicit opt-in and experimental label |

Direct routes and raw desktop commands cannot bypass the Capability Gate.

## 4. Product scope

### 4.1 Core outcomes

- Start, observe, stop, recover, persist, reopen, and name Capture Sessions.
- Prepare a Mac, iOS device, Android device, simulator, emulator, browser, CLI,
  or supported runtime and verify one request through the real MITM Runtime.
- Inspect complete request, response, timing, TLS, DNS, attribution, rule, and
  WebSocket evidence.
- Search and filter by request, response, application, device, host, path,
  protocol, status, headers, body, timing, rule outcome, and saved Focus Sets.
- Hide Noise Control matches from the current view without deleting evidence.
- Edit and forward a paused request or response, reproduce a selected request,
  compose a new request, and compare results while preserving lineage.
- Save/import/export sessions and HAR, copy redacted cURL/JSON/raw HTTP, and
  create a privacy-safe issue artifact.
- Apply reliable allow, block, map, breakpoint, modify-header, and network-
  condition behavior.

### 4.2 Advanced outcomes

- Configure selective TLS interception, bypass, custom certificates, system
  proxy automation, upstream HTTP/HTTPS/SOCKS5 proxying, PAC, authentication,
  and bypass lists.
- Inspect WebSocket frames, Protobuf, gRPC, GraphQL, JWT, AI provider traffic,
  Web3 JSON-RPC, and x402-style payment hints.
- Run bounded scripts and plugins over documented request/response hooks.
- Query captured evidence through a token-authenticated, redaction-first,
  read-only MCP Adapter.
- Correlate logs, timing, errors, and performance signals with observed requests.

### 4.3 Labs outcomes

- Use an evidence-grounded Assistant with local analysis, optional Ollama or
  configured providers, explicit Review Data, and read-only handoffs.
- Receive encrypted nearby-device session transfers without replacing the
  current local Capture Session.
- Retain existing ProxyBot analysis, generation, deployment, and SSL-bypass
  experiments only behind their current Labs boundary.

### 4.4 Explicit non-goals

- Replacing Tauri/React with SwiftUI/AppKit.
- Copying Rockxy window chrome, screenshots, names, icons, or text.
- Claiming Pro/private downstream parity.
- Shipping Rockxy's announced but unimplemented team collaboration or evidence
  bundle roadmap merely to satisfy a checklist.
- Making a privileged Helper mandatory for explicit-proxy first success.
- Requiring AI, MCP, system proxy, Frida, `pf`, or nearby transfer for Core.
- Converting ProxyBot into a general packet analyzer, wallet, security scanner,
  or cloud collaboration service.

## 5. Current capability assessment

Status meanings:

- **Present:** a real implementation path exists, but it still participates in
  milestone verification.
- **Partial:** useful pieces exist but the Community-equivalent user outcome is
  incomplete or unverified.
- **Missing:** no coherent product path was found at the ProxyBot baseline.

| Community capability | ProxyBot baseline | Required convergence |
| --- | --- | --- |
| HTTP/HTTPS/WebSocket capture | Present | persistent session identity, richer evidence, release proof |
| Advanced filter/search | Partial | one query model, response/body/process fields, highlights |
| Focus Sets/Noise Control | Missing | persisted shared queries and view-only exclusions |
| Investigation Inspector/Context Dock | Partial | complete detail DTO, factual vs inferred split |
| Multi-tab workspaces | Missing | independent persisted workspace state over shared capture |
| Notes, pinned/saved items, custom columns | Missing | session-safe persistence and fast list projection |
| AI Assistant | Partial backend experiments | grounded investigation workflow, Review Data, providers |
| MCP | Partial stdio server | read-only tool contract, token/redaction, shared facts |
| Developer Setup Hub | Partial device onboarding | target catalogue, snippets, probes, honest attribution |
| Certificate management | Partial | key protection, trust diagnostics, rotate/reset/custom certs |
| TLS selective decrypt/bypass | Present/partial | provenance, auto recovery, contract/UI consistency |
| System proxy automation | Missing | ownership, backup/restore, crash recovery, authorization |
| Block/allow lists | Partial routing rules | explicit product models, persistence, quick actions |
| Map Local/Map Remote | Partial | file/directory/snapshot safety and verified rewrite semantics |
| Request/response breakpoints | Partial | real edit-and-forward, response mutation, persistence |
| Modify headers | Missing as static action | request/response phases, add/remove/replace, presets |
| Network conditions | Partial | end-to-end runtime integration and mutually exclusive policy |
| Compose/Replay | Partial | selected-request entry, lineage, history, result comparison |
| Generic Compare | Missing | request/response/header/JSON/body comparison workspace |
| Sessions and HAR | Partial export/history | durable sessions, import, preview, redaction, migrations |
| cURL/JSON/raw/Gist/OpenAPI export | Partial | shared redaction/export policy and confirmation |
| Scripting/plugins | Partial Rhai/native skeleton | supported sandboxed hooks, UI, storage, errors, quotas |
| Upstream proxy/PAC/SOCKS5 | Missing | resolver, connector, auth storage, test and bypass |
| Protobuf/gRPC/GraphQL | Partial decoders | complete inspectors, mappings, schemas, failure states |
| AI/Web3/x402 inspection | Partial AI only | protocol labels, summaries, evidence and filters |
| Logs/timeline/error/performance | Partial timing only | correlated streams and bounded derived analysis |
| Nearby-device transfer | Missing | encrypted pairing/intake and separate imported workspace |
| Update/release security | Partial | independently verified published asset and upgrade path |

## 6. Target architecture

### 6.1 Preserve the current stack

- `proxybot-core` owns reusable MITM, routing, protocol, redaction, and domain
  rules without Tauri, React, SQLite, or macOS UI dependencies.
- `src-tauri` owns the composition root, SQLite Adapters, macOS integrations,
  credential/key protection, Desktop Contract registry, MCP Adapter, and release
  acceptance entry points.
- `src` owns the React presentation and view-local state only. It communicates
  through `DesktopContract`; raw `invoke`, raw `listen`, and `safeInvoke` are
  removed from production feature code.
- BrowserMockAdapter remains fast contract evidence. Packaged acceptance covers
  real serialization, lifecycle, persistence, and network behavior.

### 6.2 Deep Modules

The parity program deepens these Modules instead of creating feature-page silos:

| Module | Responsibility |
| --- | --- |
| `CaptureSession` | lifecycle, durable scope, recovery, statistics, config snapshot |
| `InvestigationQuery` | one filter/search/focus/noise language and evaluation seam |
| `InvestigationWorkspace` | workspace state, selection, panels, navigation and restore |
| `CapturedRequestProjection` | record/list/detail/analysis/export mappings and invariants |
| `OperationLineage` | mutation, replay, compose and comparison parent/child evidence |
| `RuleEngine` | allow/block/map/breakpoint/header/network-condition policy and audit |
| `SessionArchive` | native session format, HAR import/export, migrations and preview |
| `RedactionPolicy` | export, MCP, Assistant and issue-artifact secret handling |
| `ProtocolInspection` | bounded post-capture detection and protocol-specific projections |
| `SetupReadiness` | target catalogue, snippets, probes, certificates and system state |
| `UpstreamProxy` | direct/HTTP/HTTPS/SOCKS5/PAC resolution and credentials |
| `AutomationRuntime` | scripts/plugins, hooks, budgets, sandbox, errors and console |
| `Assistant` | local analysis, optional providers, Review Data and read-only handoffs |
| `McpAdapter` | authenticated read-only tools over shared domain Interfaces |
| `ReleaseProvenance` | build identity, signing/notary, checksums, SBOM and re-verification |

### 6.3 Data flow

```text
client/device/runtime
  -> MITM Runtime
  -> observed Capture Event
  -> CaptureSession-scoped persistence
  -> CapturedRequestProjection
  -> InvestigationQuery
  -> list/detail/workspace
  -> optional rebuildable Finding/Relationship/Protocol projections

selected evidence
  -> mutation/replay/compose operation
  -> immutable child lineage
  -> result evidence
  -> compare/redacted export/Assistant/MCP
```

Observed request/response/frame/DNS/rule/operation records are not overwritten by
derived Activities, Findings, relationships, protocol summaries, or AI output.

### 6.4 Error and recovery contract

Every desktop operation uses structured errors with a stable code, safe message,
retryability, and diagnostic context. At minimum: validation, unavailable,
permission, conflict, timeout, cancelled, persistence, network, protocol,
security, and internal.

Feature UI uses explicit idle, loading, ready, empty, stale, unavailable, and
error states. Panel failures stay isolated. A failed derived projection never
clears observed evidence. Network and certificate mutations publish ownership
and restore status before reporting success.

## 7. Delivery structure

The master milestone checklist is:

`docs/superpowers/plans/2026-08-19-rockxy-community-parity-milestones.md`

It orders all work as A -> B -> C. Each milestone has an exit gate and must be
expanded into its own executable TDD implementation plan immediately before
execution. Existing approved Batch 1-6 plans are reused where they already
provide the required file-level steps.

No two independent milestones are combined merely because they touch the same
files. A later milestone may depend on an earlier Interface but cannot silently
change it without updating this design or an ADR.

## 8. Test strategy

### 8.1 Test layers

1. Rust unit/property tests for parsing, policy, redaction, migration, protocol,
   lifecycle, sandbox, and security invariants.
2. SQLite Adapter tests for migrations, ownership, lineage, retention, and
   cross-Adapter fact equivalence.
3. Desktop Contract generation/check tests for all commands, events, DTOs, and
   errors.
4. React/Vitest tests for all UI states, cancellation, retry, and accessibility.
5. BrowserMockAdapter Playwright tests for stable journeys and workspace state.
6. Packaged Tauri acceptance for real capture, import/export, mutation, replay,
   setup, and restart seams.
7. macOS integration tests for system proxy, trust/key storage, helper or
   authorization boundary, PAC/upstream proxy, crash restore, and update.
8. Hosted release verification against the re-downloaded uploaded asset.
9. Physical iOS and Android evidence for device/certificate workflows.

### 8.2 Parity fixtures

Fixtures are independently authored from public protocol standards and minimal
observable behavior. They cover HTTP errors, redirects, compression, binary
bodies, WebSocket frames, gRPC, GraphQL, Protobuf, AI streams, Web3 batches,
x402 responses, pinned TLS, upstream proxies, PAC, malformed archives, secret
redaction, high-volume sessions, and crash recovery.

Rockxy fixtures or test code are not copied.

## 9. Release and acceptance policy

A milestone is complete only when:

- its scoped tests pass from a clean tree;
- generated artifacts are current;
- the real Adapter gate named by the milestone passes;
- limitations and cleanup behavior are documented;
- a reviewer confirms the diff matches this design;
- the commit contains only the milestone's intended files.

The entire parity program is complete only when the final checklist has no open
required item, the Community feature matrix maps every supported claim to fresh
evidence, and a re-downloaded signed/notarized release asset passes install,
capture, inspect, modify/replay, redacted share, restart, and update acceptance.

## 10. Future target changes

Rockxy development after `6a676d6` does not silently expand this program. A
rebaseline must:

1. record the new Rockxy commit;
2. classify new claims as shipped, partial, private, or future;
3. add only independently specified behavioral requirements;
4. preserve the clean-room evidence log;
5. receive explicit approval before changing the milestone checklist.
