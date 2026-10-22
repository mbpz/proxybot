# Batch 5 Activity and Context Projections Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add evidence-backed Activities, Findings, and Relationships as versioned rebuildable projections inside the Investigation Workspace.

**Architecture:** Observed evidence remains authoritative. Projection builders consume explicit Session-scoped evidence snapshots and write replaceable versioned results. Stable semantic IDs preserve UI state; confidence, competing candidates, and source references make inference auditable. Existing DNS, Alerts, Graph, Topology, and Auth Implementations become inputs or Adapters instead of competing product destinations.

**Tech Stack:** Rust analysis Modules, rusqlite, generated Desktop Contract, React/TypeScript, Vitest, Playwright

## Global Constraints

- Begin only after complete observed evidence and Core actions are proven in Batch 4.
- Activities, Findings, and Relationships are projections; deleting/rebuilding them never modifies observed evidence.
- Uncertain records remain `ungrouped`; do not create an `Unknown` Activity or missing-property bucket.
- Every inferred claim references source evidence and exposes confidence and competing candidates.
- Do not copy Tracexy's fixed DNS window or packet Session algorithms.
- Every shell command starts with `rtk`; stage exact paths only.

---

### Task 1: Define versioned projection persistence

**Files:**
- Create: `src-tauri/src/investigation/projections/mod.rs`
- Create: `src-tauri/src/investigation/projections/model.rs`
- Create: `src-tauri/src/investigation/projections/persistence.rs`
- Modify: `src-tauri/src/investigation/mod.rs`
- Modify: `src-tauri/src/db.rs`
- Modify: `CONTEXT.md`

**Interfaces:**
- Produces: `ProjectionKind::{Activity, Finding, Relationship}`
- Produces: `ProjectionRun { id, capture_session_id, kind, algorithm_version, source_high_watermark, status, started_at, completed_at, error }`
- Produces: `EvidenceRef { kind, record_id, captured_at, reason }`
- Produces: `ConfidenceTier::{Observed, High, Medium, Low}`
- Produces: `DbState::replace_projection(run, records) -> Result<(), String>`

- [ ] **Step 1: Add atomic replacement and source-preservation tests**

Seed observed Requests, build projection version 1, replace it with version 2, and assert observed row hashes are unchanged. Simulate a failed replacement transaction and assert version 1 remains active.

- [ ] **Step 2: Run and confirm projection schema is absent**

Run: `rtk cargo test -p proxybot --lib investigation::projections::persistence`

Expected: FAIL because projection tables and methods are undefined.

- [ ] **Step 3: Add run and record tables**

Create `projection_runs`, `activities`, `activity_members`, `findings`, and `relationships`. Every record references its run and CaptureSession. Make a run active only after one transaction inserts all records and marks it Completed; failed runs retain diagnostics but never replace the active run.

- [ ] **Step 4: Add domain language and commit**

Define **Activity**, **Finding**, **Relationship**, **Evidence Reference**, and **Projection Run** in `CONTEXT.md`. Then run:

```bash
rtk cargo test -p proxybot --lib investigation::projections
rtk git add CONTEXT.md src-tauri/src/investigation/projections src-tauri/src/investigation/mod.rs src-tauri/src/db.rs
rtk git commit -m "feat: persist rebuildable investigation projections"
```

### Task 2: Build deterministic Activity projection

**Files:**
- Create: `src-tauri/src/investigation/projections/activity.rs`
- Create: `src-tauri/src/investigation/projections/activity_tests.rs`
- Modify: `src-tauri/src/analysis.rs`

**Interfaces:**
- Produces: `Activity { id, capture_session_id, anchor_request_id, name, member_refs, evidence, confidence, competing_candidates, contested, algorithm_version }`
- Produces: `ActivityProjectionResult { activities, ungrouped_refs }`
- Produces: `build_activity_projection(snapshot, overrides, algorithm_version) -> ActivityProjectionResult`

- [ ] **Step 1: Add deterministic and contested fixtures**

Fixtures cover exact Replay lineage, one WebSocket exchange, same-device/app requests, redirect chain, shared CDN IP with two DNS candidates, missing attribution, and out-of-order input. Assert shuffling input preserves IDs/membership. Shared CDN evidence must be contested or ungrouped, never confidently assigned.

- [ ] **Step 2: Run and confirm builder is absent**

Run: `rtk cargo test -p proxybot --lib investigation::projections::activity_tests`

Expected: FAIL because the builder does not exist.

- [ ] **Step 3: Implement ordered evidence scoring**

Use exact lineage and connection/tunnel identity before device/app attribution, rule/replay parent, DNS provenance, and temporal similarity. Keep policy weights in `ActivityProjectionPolicy { version, max_temporal_gap_ms, weights }`; do not hard-code Tracexy's DNS interval. Stable ID is SHA-256 of CaptureSession ID, anchor Request ID, and algorithm version.

- [ ] **Step 4: Preserve explicit overrides**

Overrides are `{ anchor_request_id, name?, force_ungrouped?, member_request_ids? }` stored outside projection output. Apply them after automatic grouping and retain them across rebuilds by anchor identity.

- [ ] **Step 5: Verify and commit Activity**

```bash
rtk cargo test -p proxybot --lib investigation::projections::activity_tests
rtk cargo test -p proxybot --lib analysis
rtk git add src-tauri/src/investigation/projections/activity.rs src-tauri/src/investigation/projections/activity_tests.rs src-tauri/src/analysis.rs
rtk git commit -m "feat: infer evidence backed activities"
```

### Task 3: Convert Alerts into source-linked Findings

**Files:**
- Create: `src-tauri/src/investigation/projections/finding.rs`
- Modify: `src-tauri/src/alerts.rs`
- Modify: `src-tauri/src/anomaly.rs`
- Modify: `src-tauri/src/db.rs`
- Test: inline tests in `src-tauri/src/investigation/projections/finding.rs`, `src-tauri/src/alerts.rs`, and `src-tauri/src/anomaly.rs`

**Interfaces:**
- Produces: `Finding { id, capture_session_id, type, severity, summary, source_refs, evidence, detector_version, acknowledged }`
- Produces: `scan_captured_request({ capture_session_id, request_id }) -> FindingProjectionResult`
- Retires: context-free `scan_request_anomalies(device_id, host, ip, req_body, resp_body)` desktop command

- [ ] **Step 1: Add source-reference and no-false-empty tests**

Scan a real stored Request containing a privacy fixture and assert every Finding references its Request ID. Make the detector fail and assert the result is `Failed` with error, not `Completed` with an empty finding list.

- [ ] **Step 2: Run and confirm current live-input command lacks source identity**

Run:

```bash
rtk cargo test -p proxybot --lib investigation::projections::finding
rtk cargo test -p proxybot --lib anomaly
rtk cargo test -p proxybot --lib alerts
```

Expected: FAIL because current command accepts free-form/empty facts and Alerts lack source refs.

- [ ] **Step 3: Adapt detectors to CapturedRequestAnalysis snapshots**

Delete `live_anomaly_input` after all callers use stored `CapturedRequestAnalysis`. Publish Findings through one projection run; retain Alert acknowledgement through an Adapter during migration, then expose acknowledgement on Findings.

- [ ] **Step 4: Verify and commit Findings**

```bash
rtk cargo test -p proxybot --lib investigation::projections::finding
rtk cargo test -p proxybot --lib anomaly
rtk cargo test -p proxybot --lib alerts
rtk git add src-tauri/src/investigation/projections/finding.rs src-tauri/src/alerts.rs src-tauri/src/anomaly.rs src-tauri/src/db.rs
rtk git commit -m "refactor: link findings to captured evidence"
```

### Task 4: Adapt Graph, Topology, and Auth into Relationships

**Files:**
- Create: `src-tauri/src/investigation/projections/relationship.rs`
- Modify: `src-tauri/src/commands/graph.rs`
- Modify: `src-tauri/src/dag.rs`
- Modify: `src-tauri/src/topology/builder.rs`
- Modify: `src-tauri/src/state_machine.rs`
- Test: inline tests in `src-tauri/src/investigation/projections/relationship.rs`, `src-tauri/src/commands/graph.rs`, `src-tauri/src/dag.rs`, `src-tauri/src/topology/tests.rs`, and `src-tauri/src/state_machine.rs`

**Interfaces:**
- Produces: `RelationshipNode { id, kind: Request | Application | Host | Device | AuthState, label, evidence_refs }`
- Produces: `RelationshipEdge { id, from, to, kind, evidence_refs, confidence }`
- Produces: `build_relationship_projection(snapshot, views) -> RelationshipProjectionResult`

- [ ] **Step 1: Add semantic Adapter tests**

Assert request referer edges, token DAG edges, device/application topology, and Auth transitions map to distinct `kind` values and preserve source evidence. Reject an Adapter that maps a token edge to a referer relation merely because endpoints match.

- [ ] **Step 2: Run and confirm no common semantic projection exists**

Run: `rtk cargo test -p proxybot --lib investigation::projections::relationship`

Expected: FAIL because Graph/Topology/Auth outputs have unrelated DTOs.

- [ ] **Step 3: Implement explicit Adapters**

Keep independent algorithms. Add `FromRequestDependencyGraph`, `FromTokenDependencyDag`, `FromTopologyGraph`, and `FromAuthStateMachine` Adapters that produce Relationship nodes/edges with tagged semantics. Duplicate equivalent nodes only when their evidence identity differs.

- [ ] **Step 4: Verify and commit Relationships**

```bash
rtk cargo test -p proxybot --lib investigation::projections::relationship
rtk cargo test -p proxybot --lib commands::graph
rtk cargo test -p proxybot --lib dag
rtk cargo test -p proxybot --lib topology
rtk cargo test -p proxybot --lib state_machine
rtk git add src-tauri/src/investigation/projections/relationship.rs src-tauri/src/commands/graph.rs src-tauri/src/dag.rs src-tauri/src/topology src-tauri/src/state_machine.rs
rtk git commit -m "feat: adapt analysis into relationships"
```

### Task 5: Expose projection queries and controls through Desktop Contract

**Files:**
- Modify: `src-tauri/src/investigation/projections/mod.rs`
- Modify: `src-tauri/src/desktop_contract.rs`
- Modify: `src/desktop/contract.ts`
- Modify: `src/generated/desktop-contract.ts` through generation
- Modify: `src/desktop/contract.test.ts`

**Interfaces:**
- `rebuild_projection({ captureSessionId, kind }) -> ProjectionRun`
- `get_activities({ query }) -> ActivityProjectionPage`
- `get_findings({ query }) -> FindingProjectionPage`
- `get_relationships({ query }) -> RelationshipProjection`
- `get_investigation_baseline({ query }) -> InvestigationBaseline`
- `get_related_requests({ captureSessionId, requestId, limit }) -> CapturedRequestListItem[]`
- `save_activity_override({ captureSessionId, override }) -> Activity`

- [ ] **Step 1: Add strict evidence-link validators**

Contract tests reject an Activity, Finding, or Relationship without Session ID, algorithm/detector version, or evidence references. Validate `confidence` enum and `contested` candidates.

- [ ] **Step 2: Run and confirm commands/validators are absent**

Run: `rtk pnpm exec vitest run src/desktop/contract.test.ts`

Expected: FAIL because projection commands are not generated.

- [ ] **Step 3: Add Session-scoped commands and validators**

Every projection/baseline query consumes the same `InvestigationQuery`. Related Requests additionally require the selected Request and derive candidates by exact lineage, Activity membership, application, and host in that order. Rebuild returns a run immediately only if it is Completed; longer work reports Running via `projection-run:changed` and preserves the prior active projection until success.

- [ ] **Step 4: Generate, verify, and commit**

```bash
rtk pnpm contract:generate
rtk pnpm contract:check
rtk pnpm exec vitest run src/desktop/contract.test.ts
rtk cargo test -p proxybot --test desktop_contract
rtk git add src-tauri/src/investigation/projections/mod.rs src-tauri/src/desktop_contract.rs src/desktop/contract.ts src/generated/desktop-contract.ts src/desktop/contract.test.ts
rtk git commit -m "feat: expose investigation projections"
```

### Task 6: Integrate Activity, Findings, and Relationships into the workspace

**Files:**
- Create: `src/features/investigation/ActivityRequestList.tsx`
- Create: `src/features/investigation/ActivityRequestList.test.tsx`
- Modify: `src/features/investigation/ContextDock.tsx`
- Modify: `src/features/investigation/ContextDock.test.tsx`
- Modify: `src/features/investigation/InvestigationWorkspace.tsx`
- Modify: `src/features/investigation/workspace-state.ts`
- Modify: `src/main.tsx`
- Modify: `src/components/layout/Sidebar.tsx`
- Retire default routes: standalone DNS, Alerts, Graph, Topology routes
- Modify: `e2e/navigation.spec.ts`

**Interfaces:**
- Produces center modes: `Activities` and `Raw Requests`
- Produces Context sections with `View raw requests`, `Ungroup`, confidence, evidence, competing candidates, Findings, and Relationships

- [ ] **Step 1: Add inference auditability tests**

Render a contested Activity and assert confidence, both candidates, evidence reasons, `View raw requests`, and `Ungroup` are visible. Render ungrouped records and assert no `Unknown` group exists. Render historical baseline and related Requests with evidence links. A failed projection shows error/stale while raw Requests remain accessible.

- [ ] **Step 2: Run and confirm the factual-only Dock fails**

```bash
rtk pnpm exec vitest run src/features/investigation/ActivityRequestList.test.tsx src/features/investigation/ContextDock.test.tsx
```

Expected: FAIL because inference UI is absent.

- [ ] **Step 3: Integrate projections without remounting workspace state**

Mode changes reuse the same InvestigationQuery and selection store. Context Dock loads each projection independently. Retire the standalone Capture links/routes from default navigation; preserve opt-in deep routes only until Batch 6 CapabilityGate controls them.

- [ ] **Step 4: Verify and commit the unified investigation UI**

```bash
rtk pnpm typecheck
rtk pnpm test:ui
rtk pnpm test:e2e
rtk pnpm build
rtk git add src/features/investigation src/main.tsx src/components/layout/Sidebar.tsx e2e/navigation.spec.ts
rtk git commit -m "feat: unify contextual investigation analysis"
```
