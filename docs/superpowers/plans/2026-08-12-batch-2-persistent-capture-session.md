# Batch 2 Persistent CaptureSession Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn CaptureSession into the durable identity and lifecycle for every newly captured fact, independent of Spec Generation and instrumentation sessions.

**Architecture:** A new `capture_session` Module owns lifecycle state, SQLite persistence, recovery, and the active Session identity. The MITM Runtime Adapter receives the active identity when it starts and attaches it to emitted evidence. Desktop commands expose Rust-first Session DTOs; React renders the authoritative DTO rather than reconstructing a boolean state.

**Tech Stack:** Rust, Tauri 2, rusqlite/SQLite migrations, serde, UUID, React Context, generated Desktop Contract, packaged desktop acceptance

## Global Constraints

- Begin only after Batch 1's single Desktop Contract Seam is green.
- At most one local MITM CaptureSession is active.
- Starting persists `starting` before the listener binds; startup failure persists `failed` and its reason.
- Stopping prevents new work, drains accepted Capture Events, then persists `completed` or `failed`.
- New Captured Requests, DNS Observations, WebSocket Frames, Alerts, rule outcomes, replay lineage, and capture failures have exactly one CaptureSession identity.
- Do not use a fixed DNS correlation algorithm or create packet five-tuple sessions.
- Every shell command starts with `rtk`; stage exact paths only.

---

### Task 1: Define the CaptureSession domain and sharpen repository language

**Files:**
- Create: `src-tauri/src/capture_session/mod.rs`
- Create: `src-tauri/src/capture_session/model.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `CONTEXT.md`

**Interfaces:**
- Produces: `CaptureSessionStatus::{Starting, Running, Stopping, Completed, Failed}`
- Produces: `CaptureSession { id, name, started_at, ended_at, status, device_ids, applications, config_snapshot, record_count, byte_count, failure, app_version, schema_version, format_version }`
- Produces: `CaptureSessionStart { name, device_ids, applications }`

- [ ] **Step 1: Write domain serialization tests**

Add this test shape in `model.rs`:

```rust
#[test]
fn capture_session_wire_shape_is_stable() {
    let session = CaptureSession::starting(
        "capture-20260812-01".to_owned(),
        CaptureSessionStart {
            name: "Checkout regression".to_owned(),
            device_ids: vec![7],
            applications: vec!["com.example.shop".to_owned()],
        },
        serde_json::json!({"proxy_port": 9090}),
        "1.3.0".to_owned(),
        10,
    );
    assert_eq!(session.status, CaptureSessionStatus::Starting);
    assert_eq!(session.format_version, 1);
    assert_eq!(session.record_count, 0);
    assert!(session.ended_at.is_none());
}
```

- [ ] **Step 2: Run the test and confirm the Module is absent**

Run: `rtk cargo test -p proxybot --lib capture_session::model::tests::capture_session_wire_shape_is_stable -- --exact`

Expected: FAIL because the Module/types are undefined.

- [ ] **Step 3: Implement the minimal domain types**

Use `desktop_contract_type!` for the enum and DTOs. `CaptureSession::starting` assigns the supplied ID, UTC timestamp, `Starting`, empty counters, `format_version: 1`, and no failure/end timestamp. No persistence or Tauri code belongs in `model.rs`.

- [ ] **Step 4: Update the domain glossary**

Replace the current Capture Session definition with:

```markdown
**CaptureSession**:
A durable, user-visible investigation identity that owns one MITM Runtime lifecycle and the Captured Requests, DNS Observations, WebSocket Frames, Alerts, rule outcomes, replay lineage, and failures observed during it.
_Avoid_: Proxy toggle, running flag, inference session
```

Add `Spec Generation Run` and `Instrumentation Session` as distinct terms. Use `CaptureSession` in code identifiers and “Capture Session” in user copy.

- [ ] **Step 5: Verify and commit the domain slice**

```bash
rtk cargo test -p proxybot --lib capture_session
rtk git add CONTEXT.md src-tauri/src/lib.rs src-tauri/src/capture_session/mod.rs src-tauri/src/capture_session/model.rs
rtk git commit -m "feat: define persistent capture session domain"
```

### Task 2: Persist CaptureSession lifecycle and legacy identity

**Files:**
- Create: `src-tauri/src/capture_session/persistence.rs`
- Modify: `src-tauri/src/capture_session/mod.rs`
- Modify: `src-tauri/src/db.rs`
- Modify: `src-tauri/src/db/captured_requests.rs`
- Test: `src-tauri/src/capture_session/persistence.rs`

**Interfaces:**
- Produces: `DbState::create_capture_session(start, config_snapshot) -> Result<CaptureSession, String>`
- Produces: `DbState::transition_capture_session(id, expected, next, failure) -> Result<CaptureSession, String>`
- Produces: `DbState::capture_session(id) -> Result<Option<CaptureSession>, String>`
- Produces: `DbState::list_capture_sessions(limit) -> Result<Vec<CaptureSession>, String>`
- Produces: `DbState::recover_interrupted_capture_sessions() -> Result<usize, String>`

- [ ] **Step 1: Add migration and transition tests**

Test a fresh in-memory database and a database seeded at schema version 9. Assert migration 10 creates `capture_sessions`, migration 11 creates the legacy-import Session and reassigns every `NULL`/empty `http_requests.session_id`, and this transition is rejected:

```rust
let error = db.transition_capture_session(
    &session.id,
    CaptureSessionStatus::Starting,
    CaptureSessionStatus::Completed,
    None,
).unwrap_err();
assert!(error.contains("Starting -> Completed"));
```

- [ ] **Step 2: Run the migration tests and confirm missing tables/transitions**

Run: `rtk cargo test -p proxybot --lib capture_session::persistence`

Expected: FAIL because migrations and persistence methods do not exist.

- [ ] **Step 3: Add migrations 10 and 11**

Migration 10 creates:

```sql
CREATE TABLE capture_sessions (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  started_at TEXT NOT NULL,
  ended_at TEXT,
  status TEXT NOT NULL CHECK(status IN ('starting','running','stopping','completed','failed')),
  device_ids_json TEXT NOT NULL DEFAULT '[]',
  applications_json TEXT NOT NULL DEFAULT '[]',
  config_snapshot_json TEXT NOT NULL,
  record_count INTEGER NOT NULL DEFAULT 0,
  byte_count INTEGER NOT NULL DEFAULT 0,
  failure TEXT,
  app_version TEXT NOT NULL,
  schema_version INTEGER NOT NULL,
  format_version INTEGER NOT NULL
);
CREATE INDEX idx_capture_sessions_started_at ON capture_sessions(started_at DESC);
```

Migration 11 inserts deterministic ID `legacy-import-v1` when unassigned Captured Requests exist, then updates those rows. Use one transaction and record the migration only after all rows are assigned.

- [ ] **Step 4: Enforce the transition table**

Allow only:

```text
Starting -> Running | Failed
Running  -> Stopping | Failed
Stopping -> Completed | Failed
```

`recover_interrupted_capture_sessions` changes persisted `starting`, `running`, or `stopping` records to `failed`, sets `ended_at`, and records `Application exited before Capture Session completed`.

- [ ] **Step 5: Remove unassigned writes from normal persistence**

Change `NewCapturedRequest.session_id` to `&str`, remove its `Option`, and require every production constructor to supply it. Tests that intentionally model legacy data insert SQL directly before running migration 11.

- [ ] **Step 6: Verify and commit persistence**

```bash
rtk cargo test -p proxybot --lib capture_session
rtk cargo test -p proxybot --lib db::captured_requests
rtk cargo fmt --all -- --check
rtk git add src-tauri/src/capture_session src-tauri/src/db.rs src-tauri/src/db/captured_requests.rs
rtk git commit -m "feat: persist capture session lifecycle"
```

### Task 3: Make lifecycle startup and shutdown transactional

**Files:**
- Create: `src-tauri/src/capture_session/runtime.rs`
- Modify: `src-tauri/src/capture_session/mod.rs`
- Modify: `src-tauri/src/bootstrap.rs`
- Modify: `src-tauri/src/proxy/listener.rs`
- Modify: `src-tauri/src/proxy/runtime_adapter.rs`
- Modify: `src-tauri/src/state.rs`
- Test: `src-tauri/src/capture_session/runtime.rs`

**Interfaces:**
- Produces: `CaptureSessionRuntime::active_id() -> Option<String>`
- Produces: `start_capture_session(app, CaptureSessionStart) -> Result<CaptureSession, String>`
- Produces: `stop_capture_session(app) -> Result<CaptureSession, String>`
- Produces event: `capture-session:changed` with full `CaptureSession`, not `boolean`

- [ ] **Step 1: Add lifecycle failure and idempotence tests**

Use a fake Runtime Adapter whose start can fail and whose stop records drain completion. Assert:

```rust
assert_eq!(failed.status, CaptureSessionStatus::Failed);
assert!(failed.failure.as_deref().unwrap().contains("bind"));
assert!(runtime.active_id().is_none());
```

Assert a second start while Running returns a `conflict`, and repeated stop after completion returns the completed Session without starting another transition.

- [ ] **Step 2: Run and confirm current boolean lifecycle cannot satisfy the tests**

Run: `rtk cargo test -p proxybot --lib capture_session::runtime`

Expected: FAIL because `CaptureSessionRuntime` is absent.

- [ ] **Step 3: Implement the lifecycle coordinator**

`CaptureSessionRuntime` owns `Mutex<Option<String>>` and an operation mutex. Start order is persist Starting, set active ID, bind MITM Runtime, transition Running, emit DTO. On error transition Failed and clear active ID. Stop order is transition Stopping, stop/drain MITM Runtime, transition Completed, clear active ID, emit DTO. If drain fails, transition Failed with the drain error.

- [ ] **Step 4: Remove Spec Generation ownership of capture identity**

Delete `AppState.active_session_id` and its getters/setters. `bridge_capture_events` receives a non-optional CaptureSession ID when spawned and passes it to every persisted record. The Session ID never changes during one runtime instance.

- [ ] **Step 5: Register shared state and commands**

Manage one `Arc<CaptureSessionRuntime>` in `bootstrap.rs`. Replace `start_proxy`, `stop_proxy`, and `get_proxy_status` desktop semantics with `start_capture_session`, `stop_capture_session`, and `get_active_capture_session`; retain thin compatibility wrappers only until the React migration in Task 5 and do not expose both families after that commit.

- [ ] **Step 6: Verify and commit lifecycle**

```bash
rtk cargo test -p proxybot --lib capture_session
rtk cargo test -p proxybot --lib proxy::listener
rtk cargo test -p proxybot --lib proxy::runtime_adapter
rtk git add src-tauri/src/capture_session src-tauri/src/bootstrap.rs src-tauri/src/proxy/listener.rs src-tauri/src/proxy/runtime_adapter.rs src-tauri/src/state.rs
rtk git commit -m "feat: coordinate capture session runtime"
```

### Task 4: Attach DNS, Alert, frame, failure, and lineage evidence

**Files:**
- Modify: `src-tauri/src/db.rs`
- Modify: `src-tauri/src/dns.rs`
- Modify: `src-tauri/src/alerts.rs`
- Modify: `src-tauri/src/db/captured_requests.rs`
- Modify: `src-tauri/src/proxy/runtime_adapter.rs`
- Create: `src-tauri/src/capture_session/evidence.rs`
- Test: `src-tauri/src/capture_session/evidence.rs`

**Interfaces:**
- Produces: `NewCaptureFailure { capture_session_id, request_id, host, error, captured_at }`
- Produces: `DbState::record_capture_failure(input) -> Result<i64, String>`
- Adds required `capture_session_id` to new DNS Observations and Alerts
- WebSocket Session membership is derived and enforced through its Captured Request foreign key

- [ ] **Step 1: Add evidence ownership tests**

Start a Session, persist one Request, one frame, one DNS Observation, one Alert, one Routing Rule outcome, and one Capture Failure. Query each by Session and assert no record can be inserted with a missing or nonexistent Session ID. Assert a frame whose request belongs to another Session is rejected. Deliver a Frame before its Completed parent and assert it is buffered by runtime Request ID, then persisted only after the parent receives its desktop Request ID; stopping with an unresolved frame persists a Capture Failure instead of an orphan.

- [ ] **Step 2: Run and confirm current schema allows missing ownership**

Run: `rtk cargo test -p proxybot --lib capture_session::evidence`

Expected: FAIL because DNS, Alerts, and failures lack Session keys.

- [ ] **Step 3: Add migration 12 and persistence inputs**

Add `capture_session_id TEXT NOT NULL REFERENCES capture_sessions(id)` to new normalized DNS/Alert tables through SQLite table rebuilds, preserving legacy rows under `legacy-import-v1`. Create `capture_failures` with Session ID, request ID, host, error, and timestamp. Create `rule_applications` with Session ID, Request ID, Routing Rule identity, before/after facts, and result. Do not duplicate Session ID in `ws_frames`; enforce membership by joining `request_id` to `http_requests.id`.

- [ ] **Step 4: Thread the active ID through producers**

The DNS server receives the active Session ID from its lifecycle owner. Alert producers require a Session/source record. The Runtime Adapter records the winning Routing Rule and any mutation result. `CaptureEvent::Failed` persists `NewCaptureFailure` before logging. Frames that arrive before Completed are held in a bounded per-runtime map keyed by runtime Request ID; Completed resolves and flushes them, while stop converts unresolved entries to explicit failures. After each insert, update CaptureSession record/byte counters in the same database transaction as the evidence write.

- [ ] **Step 5: Verify and commit evidence ownership**

```bash
rtk cargo test -p proxybot --lib capture_session
rtk cargo test -p proxybot --lib dns
rtk cargo test -p proxybot --lib alerts
rtk cargo test -p proxybot --lib proxy::runtime_adapter
rtk git add src-tauri/src/db.rs src-tauri/src/dns.rs src-tauri/src/alerts.rs src-tauri/src/db/captured_requests.rs src-tauri/src/proxy/runtime_adapter.rs src-tauri/src/capture_session/evidence.rs
rtk git commit -m "feat: scope capture evidence to sessions"
```

### Task 5: Expose the persistent lifecycle through Desktop Contract and React

**Files:**
- Modify: `src-tauri/src/desktop_contract.rs`
- Modify: `src/generated/desktop-contract.ts` through generation
- Modify: `src/desktop/contract.ts`
- Modify: `src/features/capture-session/CaptureSession.tsx`
- Modify: `src/features/capture-session/CaptureSession.test.tsx`
- Modify: `src/test/setup.ts`

**Interfaces:**
- `start_capture_session({ input: CaptureSessionStart }) -> CaptureSession`
- `stop_capture_session({}) -> CaptureSession`
- `get_active_capture_session({}) -> CaptureSession | null`
- `list_capture_sessions({ limit: number }) -> CaptureSession[]`
- Event `capture-session:changed -> CaptureSession`

- [ ] **Step 1: Rewrite lifecycle tests around DTO state**

The provider test must assert the rendered name/status and stable ID:

```tsx
expect(screen.getByText("Checkout regression")).toBeInTheDocument();
expect(screen.getByText("Running")).toBeInTheDocument();
expect(adapter.calls.at(-1)).toMatchObject({
  command: "start_capture_session",
  args: { input: { name: "Checkout regression", deviceIds: [], applications: [] } },
});
```

Add a subscription fixture that emits a Failed Session and assert the failure remains visible and retryable.

- [ ] **Step 2: Run and confirm the boolean provider fails**

Run: `rtk pnpm exec vitest run src/features/capture-session/CaptureSession.test.tsx`

Expected: FAIL because the provider exposes only `running: boolean`.

- [ ] **Step 3: Generate commands/events and update the provider**

State becomes `{ session: CaptureSession | null, initialized, operation, error }`. Start requires a user name defaulted to `Capture YYYY-MM-DD HH:mm`; the generated Session ID always comes from Rust. Reconciliation reloads `get_active_capture_session` and never invents status.

- [ ] **Step 4: Remove compatibility commands from the registry**

After React and tray callers use the new coordinator, remove `start_proxy`, `stop_proxy`, `get_proxy_status`, and boolean `capture-session:changed` from the desktop registry. Internal packaged acceptance may call the coordinator directly.

- [ ] **Step 5: Verify and commit the desktop lifecycle**

```bash
rtk pnpm contract:generate
rtk pnpm contract:check
rtk pnpm exec vitest run src/features/capture-session/CaptureSession.test.tsx src/desktop/contract.test.ts
rtk pnpm typecheck
rtk cargo test -p proxybot --test desktop_contract
rtk git add src-tauri/src/desktop_contract.rs src/generated/desktop-contract.ts src/desktop/contract.ts src/features/capture-session src/test/setup.ts
rtk git commit -m "feat: expose persistent capture sessions"
```

### Task 6: Separate Spec Generation and instrumentation identity

**Files:**
- Modify: `src-tauri/src/commands/specgen.rs`
- Modify: `src-tauri/src/generation.rs`
- Modify: `src-tauri/src/frida/session.rs`
- Modify: `src-tauri/src/frida/mod.rs`
- Modify: `src-tauri/src/commands/ssl_bypass.rs`
- Modify: `src/components/ai/SpecGenPanel.tsx`
- Modify: `src/stores/sslBypassStore.tsx`
- Modify: `CONTEXT.md`
- Modify: `src-tauri/src/desktop_contract.rs`
- Modify: `src/desktop/contract.ts`
- Modify: `src/generated/desktop-contract.ts` through generation
- Modify: `src/test/AiPage.test.tsx`
- Modify: `src/stores/sslBypassStore.test.tsx`

**Interfaces:**
- Produces: `SpecGenerationRun { id, source_capture_session_id, created_at }`
- Produces: `InstrumentationSession { id, device_id, process_id, created_at }`
- Removes: `set_active_session` and `get_active_session`

- [ ] **Step 1: Add a source-identity regression test**

Create two CaptureSessions with records. Start a Spec Generation Run for the first and assert its immutable snapshot cannot change when the UI selects the second. Add a serialization test asserting instrumentation DTO uses `id` and generated TypeScript caller property `instrumentationSessionId`, never capture `sessionId`.

- [ ] **Step 2: Run and confirm global active-session state violates isolation**

Run:

```bash
rtk cargo test -p proxybot --lib specgen
rtk cargo test -p proxybot --lib frida
rtk cargo test -p proxybot --lib generation
```

Expected: FAIL until the explicit source/run identities replace global selection.

- [ ] **Step 3: Replace global selection with explicit run creation**

`create_spec_generation_run(capture_session_id)` snapshots the chosen Session and returns its run ID. Generation commands accept `spec_generation_run_id`. Instrumentation commands accept `instrumentation_session_id`. Add a migration for generated-artifact tables only when a stored column name would otherwise remain ambiguous; preserve values while renaming.

- [ ] **Step 4: Verify and commit the identity split**

```bash
rtk pnpm contract:generate
rtk cargo test -p proxybot --locked --no-default-features specgen
rtk cargo test -p proxybot --locked --no-default-features generation
rtk cargo test -p proxybot --locked --no-default-features frida
rtk pnpm typecheck
rtk pnpm test:ui
rtk git add CONTEXT.md src-tauri/src/commands/specgen.rs src-tauri/src/generation.rs src-tauri/src/frida src-tauri/src/commands/ssl_bypass.rs src/components/ai/SpecGenPanel.tsx src/stores/sslBypassStore.tsx src-tauri/src/desktop_contract.rs src/desktop/contract.ts src/generated/desktop-contract.ts src/test
rtk git commit -m "refactor: separate session identities"
```

### Task 7: Extend packaged acceptance with Session evidence

**Files:**
- Modify: `src-tauri/src/acceptance.rs`
- Modify: `scripts/test_desktop_acceptance.mjs`

**Interfaces:**
- Produces acceptance report schema version 2 with `capture_session { id, status, record_count }`, `captured_request.capture_session_id`, and `recovered_interrupted_sessions`

- [ ] **Step 1: Update the report assertions first**

Require:

```js
assert.equal(report.schema_version, 2);
assert.equal(report.capture_session.status, "Completed");
assert.equal(report.capture_session.record_count, 1);
assert.equal(report.captured_request.capture_session_id, report.capture_session.id);
assert.equal(report.recovered_interrupted_sessions, 0);
```

- [ ] **Step 2: Run against the old report and confirm failure**

Run: `rtk pnpm test:desktop:acceptance`

Expected: FAIL against schema version 1 or missing Session fields.

- [ ] **Step 3: Drive the new coordinator and emit evidence**

The acceptance journey creates a named Session, captures HTTPS, stops it, reloads it from SQLite, verifies Completed and counters, restarts into a distinct second Session, stops it, and reports both identities.

- [ ] **Step 4: Run the full batch gate and commit**

```bash
rtk cargo test --workspace --locked --no-default-features
rtk pnpm ci:local
rtk pnpm exec tauri build --bundles app
rtk pnpm test:desktop:acceptance
rtk git add src-tauri/src/acceptance.rs scripts/test_desktop_acceptance.mjs
rtk git commit -m "test: prove persistent capture sessions"
```
