# Rockxy Community Behavioral Parity Milestone Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan milestone-by-milestone. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver clean-room behavioral parity with the verifiable public Rockxy Community source edition through an ordered A -> B -> C program while preserving ProxyBot's MIT license and Rust/Tauri/React architecture.

**Architecture:** Deep shared Modules own Capture Sessions, investigation queries and projections, rules, session archives, redaction, setup, upstream proxying, automation, Assistant/MCP access, and release provenance. The generated Desktop Contract is the only React-to-Rust Interface; BrowserMockAdapter provides fast behavior evidence and packaged/macOS/physical-device gates prove real seams.

**Tech Stack:** Rust, Tokio, rustls, rusqlite/SQLite, Tauri 2, React 19, TypeScript, pnpm, Vitest, Playwright, macOS Security/Network APIs, GitHub Actions, `gh`

**Spec:** `docs/superpowers/specs/2026-08-19-rockxy-community-parity-design.md`

## Global Constraints

- Reference Rockxy snapshot: `RockxyApp/Rockxy@6a676d631820b577cf3a651c78d856733a7df995`.
- ProxyBot baseline: `a3596c777335eaca5c323a445b76a4e127704bf2`.
- Reproduce public Community behavior only; do not copy AGPL code, tests, fixtures, copy, icons, images, or private/Pro behavior.
- Keep ProxyBot's MIT license, product identity, Rust MITM Runtime, Tauri desktop shell, and React/TypeScript UI.
- Execute Phase A before Phase B and Phase B before Phase C. A later phase cannot waive an earlier exit gate.
- Observed evidence is immutable; Activities, Findings, Relationships, protocol summaries, and Assistant output are rebuildable projections.
- Production React code communicates only through `DesktopContract`; raw Tauri `invoke`, raw `listen`, and `safeInvoke` are forbidden outside the Tauri Adapter.
- Failures are structured states and must never become `null`, `[]`, `false`, or a successful empty result.
- Redaction is default-on for export, copy, Assistant, MCP, Gist, and issue artifacts.
- Explicit proxy remains the Core first-success mode; system proxy, `pf`, upstream proxy, MCP, scripting, AI, and nearby transfer are optional.
- Every task follows RED -> GREEN -> refactor, receives an independent review, stages exact paths, and commits separately.
- Every shell command starts with `rtk`.
- A task is complete only after its scoped tests, generated-contract check, diff check, documentation, and named real-Adapter gate pass.

---

## How to use this master checklist

This document is the program decomposition and progress ledger. Each milestone is
an independently rejectable and releasable outcome. Before executing a milestone:

- create or refresh one file-level implementation plan under
  `docs/superpowers/plans/`;
- pin the current ProxyBot base commit and the fixed Rockxy evidence references;
- convert each task below into test-first 2-5 minute steps with exact signatures,
  assertions, commands, and commit boundaries;
- do not execute tasks from two milestones in one working-tree batch.

Existing file-level plans are reused for Milestones 1-5 and 16 where named.

## Program dependency map

```text
Phase A
M0 -> M1 -> M2 -> M3 -> M4 -> M5 -> M6

Phase B
M6 -> M7 -> M8 -> M9
M6 -> M10 -> M11
M6 -> M12 -> M13
M3 -> M14

Phase C
M7..M14 -> M15 -> M16 -> M17
```

## Program scoreboard

| Phase | Milestones | Exit outcome | Status |
| --- | --- | --- | --- |
| A — Core depth | M0-M6 | Setup -> Capture -> Investigate -> Modify/Reproduce -> Redacted Share is durable and verified | In progress (M0 complete) |
| B — Community breadth | M7-M14 | Every public Community capability is Present, gated, documented, and testable | Not started |
| C — Historical hardening | M15-M17 | high-volume UX, security/release, historical regressions, and final parity evidence pass | Not started |

---

# Phase A — Core journey depth

## Milestone 0: Clean-room baseline and parity ledger

**Outcome:** One auditable source of truth states what is in scope, what ProxyBot
already has, what evidence proves it, and which milestone owns each gap.

**Files:**

- Modify: `docs/roadmap.md`
- Create: `docs/parity/rockxy-community-evidence.md`
- Create: `docs/parity/rockxy-community-matrix.md`
- Create: `scripts/check_parity_matrix.mjs`
- Modify: `package.json`
- Test: `scripts/check_parity_matrix.test.mjs`

**Checklist:**

- [x] **M0.1 Freeze the reference contract.** Record the Rockxy commit, public Community license, public/private artifact boundary, capture date, and exact evidence URLs; reject floating `main` links in the matrix.
- [x] **M0.2 Define evidence grades.** Encode `documented`, `source-backed`, `test-backed`, `observable-build`, and `release-proven` grades and require at least source-backed or observable-build evidence for parity scope.
- [x] **M0.3 Inventory every Community capability.** Include capture, filtering, Focus/Noise, workspaces, Assistant, MCP, setup, certificates, proxy/rules, Compose/Compare, sessions/export, scripting, protocols, logs, nearby transfer, updates, security, accessibility, and performance.
- [x] **M0.4 Map ProxyBot status.** Classify each item as Present, Partial, Missing, Out-of-scope private, or Future-not-shipped with local file/test evidence and owning milestone.
- [x] **M0.5 Make matrix completeness executable.** Add `rtk pnpm parity:check`; fail on duplicate IDs, missing owner, missing acceptance criteria, floating reference, or unsupported completion claim.
- [x] **M0.6 Establish the clean-room contribution rule.** Add contributor guidance forbidding copied Rockxy source/tests/assets and requiring independently authored fixtures from protocols or observable behavior.
- [x] **M0.7 Verify the baseline.** Run `rtk pnpm parity:check`, `rtk pnpm version:check`, `rtk pnpm contract:check`, `rtk git diff --check`, and confirm the worktree contains documentation/tooling changes only.

**Exit gate:** Every scoped capability has a stable ID, evidence grade, ProxyBot
status, owning milestone, independent acceptance statement, and machine-checked row.

---

## Milestone 1: Single Desktop Contract seam

**Existing executable plan:**
`docs/superpowers/plans/2026-08-12-batch-1-desktop-contract-convergence.md`

**Outcome:** All production React calls use generated Rust-first commands, events,
DTOs, runtime validation, structured errors, cancellation, and one Tauri Adapter.

**Checklist:**

- [ ] **M1.1 Classify the entire Tauri command registry.** Make migrated plus pending names equal the composition-root registry and print actionable drift.
- [ ] **M1.2 Correct Graph and DAG wire semantics.** Preserve distinct Rust-first DTOs and remove unsafe TypeScript assertions that conflate them.
- [ ] **M1.3 Generate structured desktop errors.** Cover validation, unavailable, permission, conflict, timeout, cancelled, persistence, network, protocol, security, and internal classes.
- [ ] **M1.4 Add cancellation and stale-response protection.** Selection changes and unmounts must supersede in-flight calls without replacing current evidence.
- [ ] **M1.5 Migrate Core and Advanced callers.** Move Capture, Setup, Rules, Replay, Settings, DNS, TLS, device, network-condition, and export surfaces.
- [ ] **M1.6 Migrate Labs callers.** Move AI, generation, deployment, SSL bypass, Graph, Topology, and remaining deep links without promoting their maturity.
- [ ] **M1.7 Enforce the seam.** Repository test permits raw Tauri imports only in `src/desktop/contract.ts`; delete `src/utils/safeInvoke.ts`.
- [ ] **M1.8 Prove real serialization.** Add invalid payload, failed mutation, cancellation, event-disposal, BrowserMockAdapter, and packaged Tauri contract tests.

**Exit gate:** Contract pending count is zero; forbidden-import check, contract
generation, UI tests, E2E, and packaged contract smoke all pass.

---

## Milestone 2: Persistent CaptureSession and evidence identity

**Existing executable plan:**
`docs/superpowers/plans/2026-08-12-batch-2-persistent-capture-session.md`

**Outcome:** A durable CaptureSession owns the lifecycle and identity of every
observed record while other domains stop overloading `session_id`.

**Checklist:**

- [ ] **M2.1 Define the CaptureSession domain.** Add stable UUID, name, timestamps, lifecycle status, scope, config snapshot, counters, failure, app/schema/format versions.
- [ ] **M2.2 Persist lifecycle and legacy migration.** Create explicit legacy-import sessions; normal queries never silently use an unscoped session.
- [ ] **M2.3 Make start transactional.** Persist `starting`, bind the listener, publish `running`, or retain a failed session with diagnostic evidence.
- [ ] **M2.4 Make stop completion-safe.** Stop new work, drain accepted events, release tasks/listeners, then persist `completed` or `failed`; repeat stop is idempotent.
- [ ] **M2.5 Attach every observed fact.** Require session identity for requests, DNS observations, WebSocket frames, Alerts/Findings inputs, rule applications, failures, and operation lineage.
- [ ] **M2.6 Separate domain identities.** Rename generation and Frida identities to SpecGenerationRun and InstrumentationSession and migrate stored data safely.
- [ ] **M2.7 Expose authoritative lifecycle DTOs.** Window and tray render the same persisted status, failures, recovery actions, statistics, and active scope.
- [ ] **M2.8 Extend packaged acceptance.** Assert start -> HTTPS request -> persistence -> stop -> restart -> reopen session with stable evidence IDs.

**Exit gate:** At most one local runtime is active; every new observed record has
exactly one CaptureSession; crash/restart leaves recoverable facts.

---

## Milestone 3: Investigation Workspace, Focus Sets, and Noise Control

**Existing executable plan:**
`docs/superpowers/plans/2026-08-12-batch-3-evidence-investigation-workspace.md`

**Outcome:** One persisted workspace provides stable session navigation, complete
request evidence, reusable queries, and independent Inspector/Context panels.

**Checklist:**

- [ ] **M3.1 Define `InvestigationQuery`.** Require session scope and support time, device, application, protocol, expression, text, Focus Set, Noise rules, sort, and pagination.
- [ ] **M3.2 Split list and detail projections.** Keep lightweight list DTOs and load exact request/response/binary/timing/frame/rule/attribution detail only after selection.
- [ ] **M3.3 Preserve evidence fidelity.** Use UTF-8 or base64 bodies without lossy decoding; represent NotRecorded, Unsupported, and Available explicitly.
- [ ] **M3.4 Persist workspace state.** Store query, grouping, sort, selection, panel sizes/tabs, expanded rows, history, CaptureSession scope, and manual panel visibility.
- [ ] **M3.5 Add Focus Sets.** Save named Filter DSL expressions with application/domain/path include/exclude semantics and make them available in every workspace.
- [ ] **M3.6 Add Noise Control.** Persist workspace-scoped view exclusions, retain muted values even when absent, and prove capture/export source evidence is not deleted.
- [ ] **M3.7 Build stable center list behavior.** Preserve selection and scroll under live updates, virtualize rows, show honest empty-state causes, and never invent an Unknown bucket.
- [ ] **M3.8 Deliver the Evidence Inspector.** Show request, response, cookies, timing, TLS/DNS, frames, protocol decode, binary/hex, rules, and explicit availability.
- [ ] **M3.9 Deliver the factual Context Dock.** Show attribution, DNS/TLS provenance, related factual records, rule outcomes, and isolated failure states before inferred projections arrive.
- [ ] **M3.10 Verify workspace restore.** Vitest and Playwright cover create/rename/switch, query restore, manual Inspector close, live selection stability, Focus/Noise, retry, and narrow layouts.

**Exit gate:** A selected persisted Captured Request remains stable through live
updates and reload and exposes all recorded evidence without leaving Capture.

---

## Milestone 4: Modify, reproduce, Compose, and redacted share

**Existing executable plan:**
`docs/superpowers/plans/2026-08-12-batch-4-modify-reproduce-share.md`

**Outcome:** User actions create immutable child evidence; exports and copies are
safe by default and operate from the selected request.

**Checklist:**

- [ ] **M4.1 Persist operation lineage.** Store parent request/session, operation kind/status, before/after request, result response, error, actor, and timestamps with valid transitions.
- [ ] **M4.2 Connect request edit-and-forward.** Edit method, HTTP(S) URL, headers, and body; reject CR/LF injection and protected fields; preview diff before forwarding.
- [ ] **M4.3 Connect response breakpoints.** Edit status, headers, and body with phase-safe validation and store both original response and mutation outcome.
- [ ] **M4.4 Reproduce the selected request.** Replay unchanged or edited evidence through the real upstream Adapter, record lineage, and distinguish mock comparison from real replay.
- [ ] **M4.5 Open selected evidence in Composer.** Populate verified method/URL/query/headers/body, keep history, create fresh requests explicitly, and record sends as child operations.
- [ ] **M4.6 Implement one redaction policy.** Cover Authorization, Cookie, Set-Cookie, proxy credentials, common secret headers, query parameters, JSON/form keys, and configured patterns.
- [ ] **M4.7 Apply redaction to all share paths.** HAR, cURL, JSON, raw HTTP, clipboard, issue artifact, MCP, Assistant, Gist, and logs consume the same policy and show a preview.
- [ ] **M4.8 Prove the Core journey.** Packaged acceptance captures, inspects, modifies/replays, verifies parent immutability, exports redacted evidence, restarts, and reopens lineage.

**Exit gate:** No UI path claims edit/replay/share unless the real operation and
its result are persisted and sensitive defaults are removed.

---

## Milestone 5: Evidence-backed Activities, Findings, and Relationships

**Existing executable plan:**
`docs/superpowers/plans/2026-08-12-batch-5-activity-context-projections.md`

**Outcome:** Derived explanations become auditable, rebuildable projections inside
the workspace rather than competing top-level destinations.

**Checklist:**

- [ ] **M5.1 Persist versioned projection runs.** Atomically replace active Activity, Finding, and Relationship outputs without changing observed record hashes.
- [ ] **M5.2 Build deterministic Activities.** Prefer exact request/frame/replay/connection/application facts, preserve stable semantic IDs, and leave uncertain evidence ungrouped.
- [ ] **M5.3 Expose evidence and uncertainty.** Every association records reason, source, time, confidence, competing candidates, contested state, algorithm version, and user override.
- [ ] **M5.4 Convert Alerts to Findings.** Link anomaly/privacy/auth/cert findings to source records; preserve acknowledgement separately from evidence.
- [ ] **M5.5 Adapt Graph/Topology/Auth to Relationships.** Share normalized facts while keeping algorithms distinct; remove equal-weight Capture destinations.
- [ ] **M5.6 Add projection controls.** Rebuild, show freshness/version, ungroup, override, reveal raw evidence, and isolate projection failure from Inspector facts.
- [ ] **M5.7 Integrate Context Dock views.** Present Activities, Findings, baselines, related requests, and Relationships with source navigation.
- [ ] **M5.8 Test adversarial correlation.** Cover CDN/shared IP, redirects, retries, missing attribution, out-of-order events, cross-device DNS, and algorithm upgrades.

**Exit gate:** Every inference has source evidence and uncertainty; deleting all
projection tables leaves observed requests fully usable.

---

## Milestone 6: Session archive, import/export, Compare, and productivity state

**Outcome:** Investigation state can be saved, reopened, imported, compared, and
shared across tools without losing identity or secrets.

**Files:**

- Create: `src-tauri/src/session_archive/{mod.rs,format.rs,import.rs,export.rs,migration.rs}`
- Create: `src-tauri/src/investigation/compare.rs`
- Modify: `src-tauri/src/har.rs`
- Modify: `src-tauri/src/db.rs`
- Create: `src/features/session-library/`
- Create: `src/features/compare/`
- Modify: `src/features/capture-session/`
- Modify: `src/components/traffic/RequestTable.tsx`
- Modify: `src-tauri/src/desktop_contract.rs`

**Checklist:**

- [ ] **M6.1 Specify a ProxyBot session archive.** Version a ZIP-based manifest with session metadata, evidence, frames, notes, workspace state, projection provenance, body entries, checksums, and redaction status.
- [ ] **M6.2 Add safe archive export.** Stream large bodies, normalize entry names, reject path traversal, cap counts/sizes, write atomically, and support redacted-default plus explicitly gated raw mode.
- [ ] **M6.3 Add import preview and migration.** Validate magic/version/checksums/limits before persistence, show counts/warnings/redactions, import transactionally, and preserve source provenance.
- [ ] **M6.4 Complete HAR import/export.** Parse HAR 1.2 requests/responses/timings/binary encodings, preview malformed/unsupported fields, scope exports, and never merge secrets silently.
- [ ] **M6.5 Add copy/export formats.** Produce redacted cURL, JSON, raw HTTP, HAR, and native session outputs from one selected/scope-aware export Interface.
- [ ] **M6.6 Persist notes and saved state.** Add request notes, pinned/saved requests, favorite applications/domains, highlight metadata, and migrations tied to stable evidence IDs.
- [ ] **M6.7 Add custom header columns.** Persist request/response header source separately, match case-insensitively, update live without reloading detail, and cap column count/width.
- [ ] **M6.8 Deliver generic Compare.** Compare two captured requests, operation results, or pasted payloads across status, headers, query, JSON tree, text, and binary hash with source navigation.
- [ ] **M6.9 Deliver multi-tab workspaces.** Share one live CaptureSession while each tab owns query, sort, selection, Inspector, layout, name, order, and optional detached-window placement.
- [ ] **M6.10 Verify round trips.** Export/import/re-export produces stable semantic evidence; malicious archives fail atomically; Playwright restores tabs, notes, columns, Compare inputs, and selection.

**Exit gate:** A session round trip and HAR round trip preserve supported facts,
redaction status, stable IDs/provenance, and workspace recovery.

---

# Phase B — Community feature breadth

## Milestone 7: Complete rule and traffic-modification parity

**Outcome:** Static rules and interactive breakpoints cover Community-equivalent
allow, block, map, header, and network-condition workflows with audited outcomes.

**Files:**

- Modify: `proxybot-core/src/{types.rs,rules_engine.rs,proxy_engine.rs}`
- Create: `proxybot-core/src/rules/{allow.rs,block.rs,map_local.rs,map_remote.rs,modify_headers.rs}`
- Modify: `src-tauri/src/{rules.rs,runtime_extensions.rs}`
- Create: `src/features/rules/`
- Modify: `src/components/rules/`
- Modify: `src-tauri/src/desktop_contract.rs`

**Checklist:**

- [ ] **M7.1 Normalize rule domain types.** Give allow, block, map local, map remote, breakpoint, modify headers, and network conditions explicit Rust-first inputs, priority, enabled state, validation, and audit output.
- [ ] **M7.2 Deliver allow/block lists.** Support host/path patterns, wildcard/regex validation, import/export, toggle, quick-create from selected request, and clear block response semantics.
- [ ] **M7.3 Deepen Map Local.** Serve a file, directory tree, or captured response snapshot; resolve symlinks, prevent traversal, cap size, infer MIME, and preserve original evidence.
- [ ] **M7.4 Deepen Map Remote.** Rewrite scheme/host/port/path/query with explicit Host-header policy, loop prevention, TLS/SNI correctness, and before/after audit evidence.
- [ ] **M7.5 Add Modify Headers.** Add/remove/replace request or response headers with URL scope, case-insensitive matching, CORS/auth/cache presets, CR/LF rejection, and deterministic action order.
- [ ] **M7.6 Finish request/response breakpoints.** Persist reusable templates/rules, queue safely, edit all allowed fields, handle cancel/drop/timeout, and prove restart persistence.
- [ ] **M7.7 Finish Network Conditions.** Provide 3G/EDGE/LTE/WiFi/very-bad/custom profiles, latency/bandwidth/loss semantics, one active effect per request, cancellation, and timing evidence.
- [ ] **M7.8 Add Rules workspace consistency.** Use one editor shell, search/filter, validation summary, quick-create navigation, matched-rule source links, and explicit policy conflicts.
- [ ] **M7.9 Prove HTTP/HTTPS/WebSocket behavior.** Integration fixtures verify every action across clear HTTP and decrypted HTTPS; unsupported WebSocket mutation is explicit.

**Exit gate:** Each rule has a persisted definition, deterministic runtime effect,
visible matched outcome, reversible toggle, and real MITM integration test.

---

## Milestone 8: Setup Hub, certificate lifecycle, and system proxy automation

**Outcome:** Supported clients can be configured and verified; certificate and
system-network changes have explicit ownership, diagnostics, and recovery.

**Files:**

- Modify: `src-tauri/src/commands/device_setup.rs`
- Create: `src-tauri/src/setup/{catalog.rs,snippets.rs,probe.rs,readiness.rs}`
- Create: `src-tauri/src/certificate/{store.rs,key_protection.rs,custom.rs,trust.rs}`
- Create: `src-tauri/src/system_proxy/{mod.rs,backup.rs,restore.rs,authorization.rs}`
- Modify: `src-tauri/src/bootstrap.rs`
- Replace/extend: `src/features/device-onboarding/`
- Create: `src/features/setup-hub/`
- Modify: `src/components/certs/`

**Checklist:**

- [ ] **M8.1 Define a target catalogue.** Cover cURL, Python, Node.js, Go, Rust, Java, Ruby, browsers, Postman-class clients, Docker, Electron, iOS, Android, simulators/emulators, Flutter, and React Native with honest support levels.
- [ ] **M8.2 Generate target-specific snippets.** Derive proxy host/port and CA path from authoritative readiness state; escape shell/language values and never claim process/device attribution from a generic probe.
- [ ] **M8.3 Add deterministic readiness probes.** Start a local HTTP/HTTPS target, run the selected client when supported, watch for the exact captured marker, classify proxy/trust/timeout failures, and clean up all child processes.
- [ ] **M8.4 Harden root CA lifecycle.** Generate strong root/leaf keys, protect private material using a macOS credential/key Adapter, cache host certificates safely, expose trust state, rotate/reset, and preserve migration/recovery evidence.
- [ ] **M8.5 Add custom certificates.** Import certificate/key pairs with format, key-match, validity, chain, hostname, permission, and duplicate checks; select by host without exposing private keys to React.
- [ ] **M8.6 Deepen selective TLS behavior.** Persist allow/deny/wildcard rules, show decrypt/bypass/passthrough provenance, classify confirmed pinning separately from transient TLS errors, and expire auto-bypass decisions.
- [ ] **M8.7 Add system proxy ownership.** Detect active services, save original HTTP/HTTPS/SOCKS/PAC settings with restrictive permissions, apply ProxyBot settings, expose owner/session identity, and never block explicit proxy startup on failure.
- [ ] **M8.8 Add crash-safe restore.** Restore on normal stop, process termination, stale ownership, and next launch; reject restoring another owner's newer settings; provide an emergency cleanup command.
- [ ] **M8.9 Choose and secure the authorization boundary.** Use a minimal macOS authorization/helper Adapter only for privileged operations, authenticate caller identity, validate inputs, version the protocol, and fail closed without adopting Rockxy source.
- [ ] **M8.10 Prove setup on real targets.** Automated local clients plus recorded physical iOS and Android runs verify HTTP, HTTPS, cleanup, certificate removal guidance, and recovery after an interrupted session.

**Exit gate:** A new user can configure a supported target, see one known decrypted
request, diagnose failure, and restore proxy/trust state without hidden ordering.

---

## Milestone 9: Upstream proxy, PAC, authentication, and bypass

**Outcome:** ProxyBot can route outbound requests through direct, HTTP, HTTPS, or
SOCKS5 upstreams selected statically or by PAC without leaking credentials.

**Files:**

- Create: `proxybot-core/src/upstream/{mod.rs,config.rs,resolver.rs,connector.rs}`
- Create: `src-tauri/src/upstream_proxy/{mod.rs,pac.rs,credentials.rs,test.rs}`
- Modify: `proxybot-core/src/proxy_engine.rs`
- Modify: `src-tauri/src/proxy/listener.rs`
- Create: `src/features/upstream-proxy/`
- Modify: `src-tauri/src/desktop_contract.rs`

**Checklist:**

- [ ] **M9.1 Define validated configuration.** Model Direct, HTTP, HTTPS, SOCKS5, and PAC URL modes with timeouts, DNS policy, bypass patterns, and no credentials in serialized UI state.
- [ ] **M9.2 Add HTTP CONNECT routing.** Authenticate safely, establish tunnels for HTTPS, forward clear HTTP correctly, preserve target Host/SNI, cap headers, and classify 407/timeout/TLS failures.
- [ ] **M9.3 Add HTTPS upstream proxying.** Validate upstream TLS trust and hostname, keep proxy credentials separate from target Authorization, and support CONNECT over TLS.
- [ ] **M9.4 Add SOCKS5.** Support no-auth and username/password negotiation, domain versus local DNS resolution, IPv4/IPv6, response-code mapping, and bounded handshake time.
- [ ] **M9.5 Add PAC resolution.** Fetch only allowed HTTP(S) URLs with size/time limits, cache with expiry, evaluate a constrained PAC runtime, and support DIRECT/PROXY/HTTPS/SOCKS routes in ordered fallback.
- [ ] **M9.6 Add bypass matching.** Normalize host/IP/port, validate wildcard/CIDR/local patterns, explain the matched rule, and ensure bypass applies before credential use.
- [ ] **M9.7 Protect credentials.** Store secrets in a macOS credential Adapter, return only configured/not-configured state, redact logs/errors, and delete credentials on explicit removal.
- [ ] **M9.8 Add Test Connection and UI.** Validate configuration against local deterministic proxy fixtures, show route/failure without changing active capture, and require confirmation before applying.
- [ ] **M9.9 Prove runtime routing.** Integration matrix covers HTTP/HTTPS target over every upstream mode, PAC fallback, bypass, auth failure, DNS behavior, cancellation, and restart persistence.

**Exit gate:** Every outbound request records the selected upstream route and
failure without exposing credentials; direct mode remains unchanged.

---

## Milestone 10: Protocol-aware inspection and custom previewers

**Outcome:** Protocol detection enriches selected evidence on demand without
blocking the capture hot path or pretending heuristic output is authoritative.

**Files:**

- Create: `proxybot-core/src/protocol/{mod.rs,graphql.rs,grpc.rs,protobuf.rs,ai.rs,web3.rs,x402.rs,jwt.rs}`
- Modify: `src-tauri/src/{normalize.rs,protobuf,graphql}`
- Create: `src-tauri/src/protocol_inspection/`
- Create: `src/features/protocol-inspector/`
- Modify: `src/components/traffic/RequestDetail.tsx`
- Modify: `src/components/ws-frames/`

**Checklist:**

- [ ] **M10.1 Define protocol labels and evidence.** Add HTTP, WebSocket, GraphQL, gRPC, Protobuf, AI, Web3 RPC, x402, and Unknown/Unsupported labels with detector version, reasons, confidence, and source bytes.
- [ ] **M10.2 Complete WebSocket inspection.** Persist connection identity and ordered directional frames, support text/binary/ping/pong/close, payload limits, search, hex, and dropped-frame diagnostics.
- [ ] **M10.3 Complete Protobuf/gRPC.** Add bounded heuristic wire trees, optional schema/mapping stores, gRPC frame/trailer handling, compression state, streaming summaries, and explicit decode failures.
- [ ] **M10.4 Complete GraphQL.** Parse operation name/type/variables/top-level fields/errors and graphql-ws messages without executing queries or discarding raw bodies.
- [ ] **M10.5 Add AI traffic inspection.** Detect supported provider/model hints, streaming state, usage, tool calls, retrieval hints, warnings, and unavailable fields from observed payloads only.
- [ ] **M10.6 Add Web3 JSON-RPC inspection.** Cover EVM/Solana-style requests, IDs, methods, batches, errors, chain/transaction hints, provider host, and debug intent without wallet behavior.
- [ ] **M10.7 Add x402 hints.** Surface payment-required and retry evidence from status/headers/body with redaction and no payment execution.
- [ ] **M10.8 Add custom previewer tabs.** Persist enabled/order settings for JSON tree, text, image, HTML-sandbox, JWT, GraphQL, Protobuf, and hex; previewers consume bounded immutable bytes.
- [ ] **M10.9 Integrate protocol filters and highlights.** Add protocol column/filter and Inspector match navigation while keeping URL/method/header rule semantics honest.
- [ ] **M10.10 Build an independent fixture corpus.** Cover malformed/truncated/compressed/binary/streaming/batch cases and assert bounded CPU/memory plus raw-evidence preservation.

**Exit gate:** Protocol summaries are reproducible, source-linked, bounded, and
clearly heuristic or verified; capture latency does not depend on deep decoding.

---

## Milestone 11: Automation runtime, scripting UI, plugins, and exporters

**Outcome:** Dynamic behavior beyond static rules is sandboxed, observable,
bounded, and reversible while using ProxyBot-native Rhai/plugin contracts.

**Files:**

- Modify: `src-tauri/src/scripting/{engine.rs,mod.rs}`
- Modify: `src-tauri/src/plugin/`
- Modify: `src-tauri/src/runtime_extensions.rs`
- Create: `src-tauri/src/automation/{model.rs,store.rs,console.rs,policy.rs}`
- Create: `src/features/automation/`
- Create: `src/features/exporters/`

**Checklist:**

- [ ] **M11.1 Define hook contracts.** Version request, response, connection, and completion contexts; expose only documented values and return explicit pass/modify/block/error outcomes.
- [ ] **M11.2 Enforce execution budgets.** Cap wall time, operations, input/output body size, console entries, and concurrent hooks; timeout/panic fails open or closed according to declared policy and records evidence.
- [ ] **M11.3 Harden the Rhai sandbox.** Deny filesystem/network/process access by default, register a small safe API, validate rewrites, and preserve deterministic script order.
- [ ] **M11.4 Persist scripts and folders.** Store source, enabled state, order, scope, revision, validation result, and last error atomically; support import/export with path and size safety.
- [ ] **M11.5 Deliver the scripting workspace.** Add list/editor/templates/console, inline parse/runtime errors, test-against-fixture, enable confirmation, and source navigation from matched requests.
- [ ] **M11.6 Complete plugin manifests and discovery.** Validate name/version/API/capabilities/checksum, quarantine incompatible plugins, require explicit enablement, and prevent untrusted native loading in default builds.
- [ ] **M11.7 Define Inspector/Exporter plugin seams.** Plugins receive immutable bounded evidence and emit typed preview/export results; failures cannot corrupt capture or storage.
- [ ] **M11.8 Add HAR/OpenAPI/Gist exporters.** Reuse SessionArchive and RedactionPolicy, preview exact payloads, keep GitHub credentials in the credential Adapter, and require explicit publish confirmation.
- [ ] **M11.9 Prove isolation and ordering.** Tests cover timeouts, panics, invalid rewrites, multiple mutations, block precedence, restart loading, malicious manifests, secret redaction, and concurrent traffic.

**Exit gate:** Scripts/plugins can modify only declared traffic fields within
budgets, surface every failure, and cannot access undeclared system resources.

---

## Milestone 12: Evidence-grounded Assistant

**Outcome:** The Assistant explains selected or workspace evidence locally first,
shares only reviewed/redacted context, and never mutates traffic automatically.

**Files:**

- Create: `src-tauri/src/assistant/{mod.rs,context.rs,budget.rs,grounding.rs,providers.rs,credentials.rs}`
- Rework: `src-tauri/src/ai_pipeline/`
- Create: `src/features/assistant/`
- Modify: `src/features/capture-session/`
- Modify: `src-tauri/src/desktop_contract.rs`

**Checklist:**

- [ ] **M12.1 Define investigation questions and results.** Support explain failure, compare requests, inspect auth, summarize workspace, and prepare bug evidence with source request references.
- [ ] **M12.2 Build local deterministic analysis.** Generate evidence-backed summaries and next checks without a model; missing evidence is stated explicitly.
- [ ] **M12.3 Build bounded context packs.** Select exact request/detail/projection facts, enforce item/token/byte budgets, redact secrets, exclude provider traffic, and record provenance.
- [ ] **M12.4 Add Review Data.** Show the exact outbound provider/model/payload/redactions before every configured-provider send and require per-send approval.
- [ ] **M12.5 Add optional providers.** Support Ollama and independently configured OpenAI-compatible/OpenAI Responses/Anthropic/Gemini-style Adapters with credential protection, streaming, cancellation, timeout, and normalized errors.
- [ ] **M12.6 Ground responses.** Require source references for factual claims, mark unsupported statements, reveal source evidence on click, and preserve selected-request anchors during streaming.
- [ ] **M12.7 Add read-only handoffs.** Assistant may prepare navigation, filter, Compare, Replay, or issue-artifact inputs, but the user must initiate every state-changing action.
- [ ] **M12.8 Add product-help mode.** Answer ProxyBot usage from versioned local docs and workspace counts without attaching captured payloads unless explicitly selected/reviewed.
- [ ] **M12.9 Verify privacy and failure behavior.** Tests cover redaction, context budgets, provider recapture prevention, cancellation, malformed streams, missing credentials, grounding, and zero-request help.

**Exit gate:** No captured content leaves the Mac without visible bounded Review
Data and user approval; every answer distinguishes evidence from suggestion.

---

## Milestone 13: Read-only authenticated MCP parity

**Outcome:** External local AI clients can query the same persisted facts through
a bounded, token-authenticated, redaction-first MCP Adapter.

**Files:**

- Modify: `src-tauri/src/mcp/{mod.rs,server.rs,transport.rs}`
- Create: `src-tauri/src/mcp/{auth.rs,limits.rs,redaction.rs,tools.rs,session.rs}`
- Modify: `src-tauri/src/bootstrap.rs`
- Create: `src/features/mcp-settings/`
- Modify: `docs/mcp-integration.md`

**Checklist:**

- [ ] **M13.1 Define the MCP threat model.** Local-only stdio/session boundary, short-lived handshake token, client/session limits, redaction defaults, audit events, and no state-changing tools.
- [ ] **M13.2 Use shared query Interfaces.** MCP consumes CaptureSession, InvestigationQuery, CapturedRequestProjection, Rule queries, certificate status, and RedactionPolicy instead of duplicate SQL.
- [ ] **M13.3 Publish exactly ten bounded tools.** Define stable tools for product status/capabilities, list sessions, query flows, get redacted flow detail, list rules, explain a matched rule, get certificate status, get setup readiness, list findings, and export redacted cURL.
- [ ] **M13.4 Enforce limits.** Cap rows, body bytes, time range, tool-call rate, concurrent requests, and response size; use cursors and explicit truncation metadata.
- [ ] **M13.5 Authenticate sessions.** Generate/store handshake state with restrictive permissions, reject replay/expired/mismatched tokens, and avoid logging token values.
- [ ] **M13.6 Apply redaction and provenance.** Every tool result declares redaction policy version, omitted fields, source Session/evidence IDs, and freshness.
- [ ] **M13.7 Add settings and client setup.** Expose disabled/running/error state, start/stop, token rotation, copy-safe client configuration, and troubleshooting without exposing captured secrets.
- [ ] **M13.8 Prove desktop/MCP equivalence.** Contract tests query the same seeded database through both Adapters and compare canonical facts, errors, limits, acknowledgements, and redaction.

**Exit gate:** MCP cannot mutate product state, cannot query unbounded data, and
returns the same redacted facts as the desktop for identical queries.

---

## Milestone 14: Logs, timeline, error/performance analysis, and nearby transfer

**Outcome:** Related runtime/application logs and imported mobile sessions enrich
investigations without contaminating observed traffic or replacing active capture.

**Files:**

- Create: `src-tauri/src/logs/{mod.rs,source.rs,store.rs,correlate.rs,filter.rs}`
- Create: `src-tauri/src/performance/{timeline.rs,errors.rs,insights.rs}`
- Create: `src-tauri/src/nearby_transfer/{protocol.rs,pairing.rs,receiver.rs,import.rs}`
- Create: `src/features/logs/`
- Create: `src/features/nearby-transfer/`
- Modify: `src/features/capture-session/`

**Checklist:**

- [ ] **M14.1 Define log evidence.** Model timestamp, level, source, process, stream, message, structured fields, CaptureSession, correlation evidence, and redaction state.
- [ ] **M14.2 Add bounded log sources.** Support explicitly selected process stdout/stderr and permitted macOS log streams with clear authorization, retention, backpressure, and stop semantics.
- [ ] **M14.3 Persist and filter logs.** Batch writes, cap memory, page SQLite, reuse Filter DSL where semantics match, and preserve missing attribution as uncorrelated.
- [ ] **M14.4 Correlate without rewriting facts.** Link logs to requests using process/session/time/request IDs where available; store confidence/reasons and allow unlink.
- [ ] **M14.5 Build timeline/error/performance projections.** Combine recorded DNS/connect/TLS/send/wait/receive, status, size, retries, and related logs; mark unrecorded phases and use evidence-backed thresholds.
- [ ] **M14.6 Define nearby-transfer protocol.** Pair explicitly, authenticate peers, encrypt transport, version messages, cap archive size/count, verify checksums, prevent replay, and expose cancellation/expiry.
- [ ] **M14.7 Import into a dedicated workspace.** Preserve current Mac capture, create an imported CaptureSession with provenance, preview metadata/redactions, and reject malformed transfers atomically.
- [ ] **M14.8 Verify retention and concurrency.** High-rate log and transfer fixtures prove bounded memory, ordered stop, redaction, active-capture isolation, and restart recovery.

**Exit gate:** Logs and imported mobile sessions remain distinguishable evidence
sources, cannot erase local traffic, and expose correlation uncertainty.

---

# Phase C — Historical hardening and verified parity

## Milestone 15: High-volume workspace, accessibility, and native desktop behavior

**Outcome:** Community-equivalent workflows remain stable, keyboard-accessible,
and readable across long sessions, narrow windows, and live updates.

**Files:**

- Modify: `src/components/traffic/RequestTable.tsx`
- Modify: `src/features/investigation/`
- Create: `src/features/keyboard-shortcuts/`
- Modify: `src/index.css`
- Add/modify: `e2e/`
- Add: `scripts/benchmark_investigation.mjs`

**Checklist:**

- [ ] **M15.1 Define performance budgets.** Set measured limits for 100k list items, live append rate, selection latency, filter latency, Inspector load, memory, body preview, and projection rebuild.
- [ ] **M15.2 Virtualize and incrementally update.** Avoid full list/filter/sidebar rebuilds on append, batch events, preserve selection/scroll, and load large bodies on demand.
- [ ] **M15.3 Bound retention and offload bodies.** Make live caps, eviction/persistence policy, disk-body threshold, cleanup, missing-body state, and archive behavior explicit.
- [ ] **M15.4 Complete keyboard workflows.** Provide searchable shortcuts for capture, workspace tabs, filters, request navigation, Inspector, rules, breakpoints, Compose, Compare, and settings without conflicts.
- [ ] **M15.5 Complete accessibility.** Add semantic names/roles, focus order, screen-reader updates, contrast, reduced motion, zoom/text scaling, and non-color status cues.
- [ ] **M15.6 Harden panel/window state.** Restore sizes/placement safely across monitors, clamp invalid state, support narrow layouts, and never hide the only recovery action.
- [ ] **M15.7 Standardize empty/error/status copy.** Explain no capture, filtered-out traffic, disabled capability, permission, stale data, unsupported protocol, and retry/cleanup actions consistently.
- [ ] **M15.8 Add deterministic benchmarks and E2E.** Seed synthetic 100k sessions, enforce budgets with tolerance, exercise rapid live updates, resize, keyboard, accessibility snapshots, and workspace restore.

**Exit gate:** Defined performance and accessibility budgets pass on the supported
macOS baseline without losing evidence, selection, or recovery controls.

---

## Milestone 16: Capability enforcement, security, update, and verified release

**Existing executable plan:**
`docs/superpowers/plans/2026-08-12-batch-6-capability-verified-release.md`

**Outcome:** Product claims match gated capability maturity and only independently
verified uploaded artifacts become public releases.

**Checklist:**

- [ ] **M16.1 Define one capability catalogue.** Exhaustively classify Core, Advanced, and Labs requirements, prerequisites, risks, and default state.
- [ ] **M16.2 Enforce capability decisions.** Apply the same gate to routes, navigation, Desktop Contract commands, settings, docs, acceptance, and release manifest.
- [ ] **M16.3 Harden Tauri security.** Use a non-null CSP, minimal capabilities, path/URL allowlists, no production devtools, and explicit sidecar/helper permissions.
- [ ] **M16.4 Verify secret storage.** Audit CA keys, upstream/provider/GitHub credentials, MCP tokens, archives, logs, and crash diagnostics for permission/redaction/lifecycle policy.
- [ ] **M16.5 Establish one BuildFlavor.** Local, CI, release candidate, and verified release use the same version, features, resource lock, Tauri config, and provenance inputs.
- [ ] **M16.6 Lock third-party resources.** Checksum/version/license-pin Apktool, Frida assets/devkit, sidecars, schemas, and generated web resources; release build cannot download undeclared inputs.
- [ ] **M16.7 Generate release provenance.** Bind git SHA, versions, schema/format, lock digests, SBOM, tests, signing identity/team, notarization request, staple, checksums, and asset lengths.
- [ ] **M16.8 Publish Draft and re-download.** Verify the exact GitHub assets with checksum, codesign, Gatekeeper, stapler, manifest, SBOM, and provenance before any release is marked verified.
- [ ] **M16.9 Verify install and update.** Clean-machine install/start/capture, prior-version migration, signed updater/appcast, rollback-safe failure, and no unreviewed downgrade.
- [ ] **M16.10 Require physical-device evidence.** Record iOS/Android setup, HTTPS capture, cleanup, app/build identifiers, artifact digest, and run URL before publication.

**Exit gate:** A public claim and release badge can be generated only from the
re-downloaded signed/notarized asset whose manifest and acceptance evidence pass.

---

## Milestone 17: Historical release regression catalogue and final parity audit

**Outcome:** Rockxy's public Community release history becomes an independent
regression catalogue, and every scoped capability has current ProxyBot evidence.

**Files:**

- Create: `docs/parity/rockxy-release-regressions.md`
- Create: `tests/parity/`
- Create: `scripts/check_parity_release.mjs`
- Modify: `docs/parity/rockxy-community-matrix.md`
- Modify: `docs/roadmap.md`
- Modify: `CHANGELOG.md`

**Checklist:**

- [ ] **M17.1 Catalogue public release behavior.** Convert relevant Rockxy changelog items into independently worded ProxyBot regression statements grouped by capture, TLS, helper/proxy, storage, rules, workspaces, protocols, privacy, and release.
- [ ] **M17.2 Exclude invalid targets.** Mark private/Pro, branded, source-only implementation details, superseded bugs, and public Future Work as non-requirements with reasons.
- [ ] **M17.3 Build regression fixtures.** Independently author cases for CONNECT framing, TLS pipeline transition, pinned-host fallback, connection leaks/timeouts, crash restore, rule forwarding, workspace selection, import safety, and redaction.
- [ ] **M17.4 Prove migration compatibility.** Exercise every ProxyBot schema/session/archive version, prior release install, rule/settings migration, corrupted state, interrupted update, and rollback-safe recovery.
- [ ] **M17.5 Re-audit every matrix row.** Require current source, test, UI, docs, real-Adapter, and release evidence; downgrade claims whose evidence is fixture-only or stale.
- [ ] **M17.6 Run the full local gate.** Contract, format, Rust tests, Clippy, typecheck, UI, build, E2E, Tauri bundle, packaged desktop acceptance, parity checks, migration tests, and benchmarks pass from a clean tree.
- [ ] **M17.7 Run hosted and artifact gates.** Identify the exact head SHA run with `gh`, verify all required jobs execute real tests, then verify the re-downloaded release candidate and physical-device evidence.
- [ ] **M17.8 Perform independent product review.** A reviewer follows public docs from clean install through setup, capture, investigation, modify/replay, export/import, restart, update, and cleanup without repository knowledge.
- [ ] **M17.9 Close documentation truth gaps.** Align README, roadmap, support matrix, screenshots, changelog, security, release notes, and capability labels with verified outcomes only.
- [ ] **M17.10 Publish the parity report.** Record target snapshot, ProxyBot release digest, passed evidence, intentional behavioral differences, unsupported/private exclusions, and remaining non-parity roadmap items.

**Exit gate:** The master checklist has no open required item; every public parity
claim links to fresh evidence from the exact verified release artifact.

---

# Full feature coverage checklist

This index prevents a milestone from passing while a public Community feature is
unowned. Completion is derived from the owning task and cannot be checked by
documentation alone.

## Capture and investigation

- [ ] HTTP/HTTPS capture, CONNECT, TLS interception, pass-through, and recovery — M2/M8/M17
- [ ] WebSocket connection/frame inspection — M3/M10
- [ ] Complete request/response/timing/TLS/DNS/rule evidence — M3
- [ ] Advanced filter and full-text search — M3
- [ ] Focus Sets and Noise Control — M3
- [ ] Context Dock and bottom Evidence Inspector — M3/M5
- [ ] Notes, pinned/saved requests, favorites, highlights — M6
- [ ] Custom request/response header columns — M6
- [ ] Multi-tab and detached persisted workspaces — M6/M15
- [ ] Activity, Finding, Relationship and baseline projections — M5

## Modification and reproduction

- [ ] Allow and block lists — M7
- [ ] Map Local file/directory/snapshot — M7
- [ ] Map Remote rewrite — M7
- [ ] Request and response breakpoints — M4/M7
- [ ] Modify request/response headers and presets — M7
- [ ] Network condition profiles — M7
- [ ] Selected-request Replay and history — M4
- [ ] Composer edit/send/history — M4
- [ ] Generic request/response/body Compare — M6

## Sessions and sharing

- [ ] Native session save/import/export and migrations — M6
- [ ] HAR import/export — M6
- [ ] Redacted cURL, JSON, raw HTTP, clipboard — M4/M6
- [ ] OpenAPI export and Gist publishing — M11
- [ ] Privacy-safe issue artifact — M4/M6

## Setup, certificates, and routing

- [ ] Developer Setup Hub target catalogue and snippets — M8
- [ ] One-click deterministic readiness probes — M8
- [ ] Root CA trust, rotate/reset, and key protection — M8
- [ ] Custom certificates — M8
- [ ] Selective TLS decrypt/bypass/passthrough — M8
- [ ] System proxy ownership and crash restore — M8
- [ ] HTTP/HTTPS upstream proxy — M9
- [ ] SOCKS5, authentication, PAC, and bypass — M9

## Protocols, automation, and intelligence

- [ ] Protobuf heuristic/schema mapping and gRPC — M10
- [ ] GraphQL HTTP/WebSocket inspection — M10
- [ ] JWT and custom previewers — M10
- [ ] AI provider/model traffic inspection — M10
- [ ] Web3 JSON-RPC inspection — M10
- [ ] x402 payment-flow hints — M10
- [ ] Sandboxed scripting UI and hooks — M11
- [ ] Plugin discovery/validation/Inspector/Exporter seams — M11
- [ ] Evidence-grounded local/provider Assistant — M12
- [ ] Token-authenticated redaction-first read-only MCP — M13
- [ ] Correlated logs, error analysis, timeline, performance insights — M14
- [ ] Encrypted nearby-device session transfer — M14

## Product quality and release

- [ ] High-volume performance and retention budgets — M15
- [ ] Keyboard shortcuts, accessibility, appearance, window restore — M15
- [ ] Core/Advanced/Labs capability enforcement — M16
- [ ] Credential, helper/sidecar, CSP, filesystem, and update security — M8/M9/M11/M13/M16
- [ ] Signed, notarized, stapled, checksummed, SBOM/provenance release — M16
- [ ] Clean install, upgrade, rollback-safe failure, iOS/Android evidence — M16/M17
- [ ] Historical regression catalogue and final Community parity report — M17

---

# Standard milestone verification checklist

Run the applicable subset after every task and the full set at each milestone exit:

- [ ] `rtk pnpm parity:check`
- [ ] `rtk pnpm version:check`
- [ ] `rtk pnpm contract:check`
- [ ] `rtk cargo fmt --all -- --check`
- [ ] `rtk cargo test --workspace --locked --no-default-features`
- [ ] `rtk cargo clippy --workspace --all-targets --locked --no-default-features -- -D warnings`
- [ ] `rtk pnpm typecheck`
- [ ] `rtk pnpm test:ui`
- [ ] `rtk pnpm build`
- [ ] `rtk pnpm test:e2e`
- [ ] `rtk pnpm exec tauri build --bundles app`
- [ ] `rtk pnpm test:desktop:acceptance`
- [ ] milestone-specific macOS/network/security/physical-device gate
- [ ] `rtk git diff --check`
- [ ] `rtk git status --short`
- [ ] independent spec review and code-quality review
- [ ] exact-path commit with the milestone report and no unrelated changes

## Final completion statement

Do not state "Rockxy Community parity complete" until Milestone 17 passes from
the exact re-downloaded verified ProxyBot release asset. Until then, report the
highest completed milestone and name the remaining product, hosted-CI, signing,
network, physical-device, or release-evidence gaps explicitly.
