# Batch 3 Evidence Investigation Workspace Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver one stable Investigation Workspace that loads complete persisted Captured Request evidence without mixing analysis-only or lossy DTOs into the UI.

**Architecture:** A deep `investigation` Module owns query wire types and Captured Request projections. Lists use a lightweight DTO; selection calls a complete detail command. React owns workspace interaction state behind a storage Interface with browser and memory Adapters, while every desktop operation crosses `DesktopContract`.

**Tech Stack:** Rust, rusqlite, generated Desktop Contract, React 19, TypeScript, CSS grid, localStorage Adapter, Vitest, Playwright

## Global Constraints

- Begin only after every new fact has a persistent CaptureSession from Batch 2.
- `CapturedRequestRecord` remains internal to the Persistence Adapter.
- `CapturedRequestAnalysis` remains the immutable input for analysis Implementations.
- Do not use `NormalizedRecord` as a traffic-list fallback or fill missing host/application fields with empty values.
- Unsupported or unrecorded evidence is explicit; it is never rendered as a successful empty result.
- The Context Dock initially shows factual provenance and explicit unavailable states; inferred Activities arrive in Batch 5.
- Every shell command starts with `rtk`; stage exact paths only.

---

### Task 1: Define InvestigationQuery and complete wire projections

**Files:**
- Create: `src-tauri/src/investigation/mod.rs`
- Create: `src-tauri/src/investigation/query.rs`
- Create: `src-tauri/src/investigation/projection.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/db/captured_requests.rs`
- Modify: `src-tauri/src/desktop_contract.rs`
- Modify: `src/generated/desktop-contract.ts` through generation

**Interfaces:**
- Produces: `SessionScope::{Exact(String), Many(Vec<String>)}`
- Produces: `InvestigationQuery { session_scope, start_ms, end_ms, device_ids, applications, protocols, expression, text, noise_expressions, order, page, page_size }`
- Produces: `CapturedRequestListItem { id, capture_session_id, captured_at, method, scheme, host, path, status, duration_ms, response_size, application, device_id, is_websocket }`
- Produces: `EvidenceBody { encoding: "utf8" | "base64", content, size }`
- Produces: `EvidenceAvailability::{Available, NotRecorded, Unsupported(String)}`
- Produces: `CapturedRequestDetail { request, response, timing, websocket, protocol, binary, applied_rules, attribution }`

- [ ] **Step 1: Write projection preservation tests**

Build one `CapturedRequestRecord` containing response headers/body, binary request bytes, negative duration, WebSocket identity, gRPC/GraphQL metadata source bodies, device/app/client/upstream values, and Session ID. Assert List omits bodies but preserves identity; Detail represents invalid UTF-8 as base64 and preserves response fields.

```rust
assert_eq!(detail.request.body.encoding, BodyEncoding::Base64);
assert_eq!(detail.response.status, Some(201));
assert_eq!(detail.attribution.application.as_deref(), Some("sample-app"));
assert_eq!(detail.timing.duration_ms, 0);
```

- [ ] **Step 2: Run and confirm the projection Module is absent**

Run: `rtk cargo test -p proxybot --lib investigation::projection`

Expected: FAIL because the types/mappings are undefined.

- [ ] **Step 3: Implement pure mappings and explicit availability**

Implement `CapturedRequestListItem::from_record` and `CapturedRequestDetail::from_record(record, frames)`. Use exact bytes; never call `String::from_utf8_lossy` for the Inspector. Mark DNS/TLS sub-timings and applied-rule history `NotRecorded` until a producer supplies them. Existing total duration remains Available and clamps invalid negatives to zero.

- [ ] **Step 4: Compile InvestigationQuery into persistence queries**

Reject empty `Many`, reversed time windows, and page sizes outside `1..=500`. Reuse the existing Filter DSL compiler for `expression`; apply `noise_expressions` as a final negated view projection. The Session scope is required and has no `Any` default.

- [ ] **Step 5: Generate and verify wire types**

```bash
rtk pnpm contract:generate
rtk pnpm contract:check
rtk cargo test -p proxybot --lib investigation
rtk cargo test -p proxybot --lib db::captured_requests
rtk git add src-tauri/src/investigation src-tauri/src/lib.rs src-tauri/src/db/captured_requests.rs src-tauri/src/desktop_contract.rs src/generated/desktop-contract.ts
rtk git commit -m "feat: define investigation request projections"
```

### Task 2: Replace the mixed traffic page with list/detail commands

**Files:**
- Modify: `src-tauri/src/investigation/mod.rs`
- Modify: `src-tauri/src/normalize.rs`
- Modify: `src-tauri/src/proxy/commands.rs`
- Modify: `src-tauri/src/desktop_contract.rs`
- Modify: `src/desktop/contract.ts`
- Modify: `src/components/traffic/model.ts`
- Modify: `src/components/traffic/model.test.ts`
- Modify: `src/generated/desktop-contract.ts` through generation

**Interfaces:**
- `get_captured_request_page({ query: InvestigationQuery }) -> CapturedRequestPage`
- `get_captured_request_detail({ captureSessionId: string, requestId: string }) -> CapturedRequestDetail`
- `CapturedRequestPage { records, total, page, page_size, has_more }`

- [ ] **Step 1: Add list/detail command tests**

Assert the page returns exactly one list projection per persisted row and no `normalized_records`. Assert detail rejects a Request belonging to another Session with a not-found error so cross-session IDs cannot leak evidence.

- [ ] **Step 2: Run and confirm current `get_traffic_page` shape fails**

Run: `rtk cargo test -p proxybot --lib investigation`

Expected: FAIL because the commands do not exist and current page includes duplicate normalized rows.

- [ ] **Step 3: Add commands and remove UI use of NormalizedRecord**

Keep `NormalizedRecord` only in the AI/analysis Module. Remove `normalized_records` from the traffic wire page and delete `normalizedRecordToListItem`. The list ViewModel maps only generated `CapturedRequestListItem` and changes no facts.

- [ ] **Step 4: Add strict result validators**

Browser Contract tests reject detail results that omit response headers/body availability or Session identity. A `NotRecorded` value is valid; a missing field is not.

- [ ] **Step 5: Verify and commit the query Seam**

```bash
rtk pnpm contract:generate
rtk cargo test -p proxybot --lib investigation
rtk cargo test -p proxybot --lib normalize
rtk cargo test -p proxybot --lib proxy::commands
rtk pnpm exec vitest run src/components/traffic/model.test.ts src/desktop/contract.test.ts
rtk pnpm typecheck
rtk git add src-tauri/src/investigation src-tauri/src/normalize.rs src-tauri/src/proxy/commands.rs src-tauri/src/desktop_contract.rs src/desktop/contract.ts src/components/traffic/model.ts src/components/traffic/model.test.ts src/generated/desktop-contract.ts
rtk git commit -m "refactor: split captured request list and detail"
```

### Task 3: Persist Investigation Workspace interaction state

**Files:**
- Create: `src/features/investigation/workspace-state.ts`
- Create: `src/features/investigation/workspace-state.test.ts`
- Create: `src/features/investigation/InvestigationWorkspaceProvider.tsx`
- Modify: `CONTEXT.md`

**Interfaces:**
- Produces: `FocusSet { id, name, query: InvestigationQuery }`
- Produces: `InvestigationWorkspaceState { sessionIds, selectedKind, selectedId, grouping, sort, expression, text, focusSetId, focusSets, noiseExpressions, inspector, contextDock, expandedIds }`
- Produces: `WorkspaceStateAdapter { load(key): WorkspaceState | null; save(key, state): void }`
- Produces: `LocalStorageWorkspaceStateAdapter` and `MemoryWorkspaceStateAdapter`

- [ ] **Step 1: Add migration, manual-close, and stale-ID tests**

Test schema version 1 serialization. Unknown keys are ignored; invalid panel sizes reset to defaults; a selected ID is preserved across live updates if still present. Assert `selectEvidence` opens the Inspector only when `inspector.manuallyClosed === false`. Save a Focus Set and assert restoring it reinstates the full InvestigationQuery, not only its text expression. Add a Noise Control rule and assert it affects the view query while the stored evidence count remains unchanged.

- [ ] **Step 2: Run and confirm state Module is absent**

Run: `rtk pnpm exec vitest run src/features/investigation/workspace-state.test.ts`

Expected: FAIL because the Adapter and reducer are undefined.

- [ ] **Step 3: Implement a pure reducer and two Adapters**

Use action types:

```ts
type WorkspaceAction =
  | { type: "select"; kind: "activity" | "request"; id: string }
  | { type: "closeInspector" }
  | { type: "openInspector" }
  | { type: "setQuery"; expression: string; text: string }
  | { type: "saveFocusSet"; focusSet: FocusSet }
  | { type: "deleteFocusSet"; id: string }
  | { type: "setNoiseExpressions"; expressions: string[] }
  | { type: "resizePanel"; panel: "inspector" | "contextDock"; size: number }
  | { type: "restore"; state: InvestigationWorkspaceState };
```

Persist under `proxybot.investigation.v1.<capture-session-id>`. Debounce writes by 100 ms; flush on unmount. Focus Set IDs are UUIDs created in React; applying one copies its query into active state so later edits do not mutate the saved value.

- [ ] **Step 4: Update the glossary and commit**

Define **Investigation Workspace**, **Evidence Inspector**, **Context Dock**, **Focus Set**, and **Noise Control** in `CONTEXT.md`, using the approved meanings. Then run:

```bash
rtk pnpm exec vitest run src/features/investigation/workspace-state.test.ts
rtk pnpm typecheck
rtk git add CONTEXT.md src/features/investigation
rtk git commit -m "feat: persist investigation workspace state"
```

### Task 4: Build the Session/Focus area and center list

**Files:**
- Create: `src/features/investigation/InvestigationWorkspace.tsx`
- Create: `src/features/investigation/SessionFocusPane.tsx`
- Create: `src/features/investigation/CapturedRequestList.tsx`
- Create: `src/features/investigation/InvestigationWorkspace.test.tsx`
- Modify: `src/features/capture-session/CaptureWorkspace.tsx`
- Modify: `src/components/traffic/TrafficPage.tsx`
- Modify: `src/components/traffic/RequestTable.tsx`
- Modify: `src/main.tsx`
- Modify: `src/components/layout/Navigation.test.tsx`

**Interfaces:**
- Consumes: `list_capture_sessions`, `get_captured_request_page`, workspace provider
- Produces: one Capture destination whose center list preserves selection across page refresh and live events

- [ ] **Step 1: Add navigation and selection tests**

Assert Capture no longer exposes `Requests`, `DNS`, `Alerts`, `Graph`, or `Topology` as equal links. Render two records, select one, emit a page refresh containing the same ID, and assert `aria-selected="true"` remains on it.

- [ ] **Step 2: Run and confirm old Capture tabs fail**

Run:

```bash
rtk pnpm exec vitest run src/components/layout/Navigation.test.tsx src/features/investigation/InvestigationWorkspace.test.tsx
```

Expected: FAIL because `CaptureWorkspace` still renders five context links.

- [ ] **Step 3: Compose the new workspace**

Replace `CaptureWorkspace`'s `ContextNav` with `InvestigationWorkspace`. Session selection is mandatory; if none is active, select the latest completed Session or render `Start a Capture Session` without issuing an all-session query. `TrafficPage` becomes the center-list Implementation or is folded into `CapturedRequestList` and deleted after its tests move. Selecting a Session/Request writes `?session=<capture-session-id>&request=<request-id>`; direct navigation restores only after both IDs pass Contract lookups.

- [ ] **Step 4: Keep live updates scoped**

On `intercepted-request`, update only when `capture_session_id` is in current `sessionIds` and the record matches the current query. Otherwise invalidate the page and show stale state; do not append an unfiltered row.

- [ ] **Step 5: Verify and commit the workspace shell**

```bash
rtk pnpm typecheck
rtk pnpm exec vitest run src/features/investigation/InvestigationWorkspace.test.tsx src/components/layout/Navigation.test.tsx src/test/TrafficPage.test.tsx
rtk pnpm test:e2e
rtk git add src/features/investigation src/features/capture-session/CaptureWorkspace.tsx src/components/traffic src/main.tsx src/components/layout/Navigation.test.tsx e2e
rtk git commit -m "feat: add investigation workspace shell"
```

### Task 5: Deliver the Evidence Inspector

**Files:**
- Create: `src/features/investigation/EvidenceInspector.tsx`
- Create: `src/features/investigation/EvidenceInspector.test.tsx`
- Create: `src/features/investigation/evidence/RequestEvidence.tsx`
- Create: `src/features/investigation/evidence/ResponseEvidence.tsx`
- Create: `src/features/investigation/evidence/TimelineEvidence.tsx`
- Create: `src/features/investigation/evidence/WebSocketEvidence.tsx`
- Create: `src/features/investigation/evidence/ProtocolEvidence.tsx`
- Create: `src/features/investigation/evidence/BinaryEvidence.tsx`
- Create: `src/features/investigation/evidence/AppliedRulesEvidence.tsx`
- Reuse: `src/components/traffic/HeadersView.tsx`, `src/components/traffic/BodyView.tsx`, `src/components/ws/HexDump.tsx`
- Retire after migration: `src/components/traffic/RequestDetail.tsx`

**Interfaces:**
- Consumes: `get_captured_request_detail`
- Produces tabs: Request, Response, Timeline, WebSocket, Protocol, Binary, Applied Rules
- Produces explicit `available`, `not recorded`, and `unsupported` render states

- [ ] **Step 1: Add complete evidence and failure-isolation tests**

Use a fixture with request/response headers and bodies, WebSocket frames, GraphQL data, and unavailable TLS timing. Assert Response body and `TLS timing was not recorded` are both visible. Then make detail loading throw and assert the center list remains mounted and selected.

- [ ] **Step 2: Run and confirm the old detail projection fails**

Run: `rtk pnpm exec vitest run src/features/investigation/EvidenceInspector.test.tsx`

Expected: FAIL because `RequestDetail` has only Headers/Body/WebSocket from a lossy list item.

- [ ] **Step 3: Implement cancellable detail loading**

Use an incrementing request token or `AbortController` supported by the Contract. A result updates state only if its token equals the latest selection. Query state is `idle | loading | ready | error`; `empty` is not valid for a known selected ID.

- [ ] **Step 4: Render exact evidence and availability**

Headers preserve duplicates and case in their raw list. Bodies use UTF-8 or base64 according to `EvidenceBody`. Binary view uses raw decoded bytes from the generated DTO. Protocol sections render only verified decode output; unavailable sections display the supplied reason.

- [ ] **Step 5: Verify and commit the Inspector**

```bash
rtk pnpm exec vitest run src/features/investigation/EvidenceInspector.test.tsx src/test/TrafficPage.test.tsx
rtk pnpm typecheck
rtk pnpm test:e2e
rtk git add src/features/investigation src/components/traffic/RequestDetail.tsx src/components/traffic/HeadersView.tsx src/components/traffic/BodyView.tsx src/components/ws/HexDump.tsx e2e
rtk git commit -m "feat: inspect complete captured request evidence"
```

### Task 6: Add the factual Context Dock and resizable layout

**Files:**
- Create: `src/features/investigation/ContextDock.tsx`
- Create: `src/features/investigation/ContextDock.test.tsx`
- Create: `src/features/investigation/ResizableInvestigationLayout.tsx`
- Create: `src/features/investigation/ResizableInvestigationLayout.test.tsx`
- Modify: `src/features/investigation/InvestigationWorkspace.tsx`
- Modify: `src/index.css`

**Interfaces:**
- Produces panels: Attribution, DNS/TLS Provenance, Findings, Baseline, Related, Relationships
- Initial batch behavior: attribution/provenance facts are rendered; inference-only panels state `Available after analysis` without claiming no findings

- [ ] **Step 1: Add panel responsibility tests**

Assert raw Request/Response bodies never appear in Context Dock, attribution evidence does, and a Context command failure renders `Context unavailable` without removing Inspector evidence. Assert closing Inspector persists manual closure and selection does not reopen it.

- [ ] **Step 2: Run and confirm missing layout behavior**

Run:

```bash
rtk pnpm exec vitest run src/features/investigation/ContextDock.test.tsx src/features/investigation/ResizableInvestigationLayout.test.tsx
```

Expected: FAIL because the Dock/layout do not exist.

- [ ] **Step 3: Implement CSS-grid layout and pointer resizing**

Use semantic regions and persisted sizes. Clamp Inspector to `180..600` px and Context Dock to `260..520` px. Pointer cancel restores the last committed size; keyboard resize handles change by 16 px and expose `aria-valuenow`.

- [ ] **Step 4: Verify the full Batch 3 workflow**

```bash
rtk pnpm typecheck
rtk pnpm test:ui
rtk pnpm build
rtk pnpm test:e2e
rtk cargo test --workspace --locked --no-default-features
rtk git add src/features/investigation src/index.css
rtk git commit -m "feat: add investigation context dock"
```
