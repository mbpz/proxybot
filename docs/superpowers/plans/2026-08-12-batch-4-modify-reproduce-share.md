# Batch 4 Modify, Reproduce, and Redacted Share Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete the Core journey by making edits and reproductions traceable children of immutable evidence and making sharing redacted by default.

**Architecture:** Mutation and reproduction use explicit command DTOs rather than mutable `InterceptedRequest` snapshots. A new lineage table records parent evidence, operation, before/after change, result, and error. Redaction is a pure policy Module applied before HAR serialization or clipboard output; raw export is a gated Advanced action delivered later by CapabilityGate.

**Tech Stack:** Rust, rusqlite, reqwest, Tauri Desktop Contract, React/TypeScript, Vitest, packaged desktop acceptance

## Global Constraints

- Begin only after Batch 3 exposes complete Captured Request Detail from one Session.
- Never mutate or delete the original observed Captured Request.
- Method, URL, headers, and body are editable; protected transport-derived fields are not.
- A local mock replay is labelled mock comparison and is never presented as upstream reproduction.
- Redaction is default-on for export and copy; raw export requires an explicit Advanced path.
- Every operation records Session and parent Request identity.
- Every shell command starts with `rtk`; stage exact paths only.

---

### Task 1: Define operation lineage and persistence

**Files:**
- Create: `src-tauri/src/investigation/operations.rs`
- Modify: `src-tauri/src/investigation/mod.rs`
- Modify: `src-tauri/src/db.rs`
- Modify: `CONTEXT.md`

**Interfaces:**
- Produces: `InvestigationOperationKind::{BreakpointMutation, Replay, ComposerSend, RedactedExport, RawExport}`
- Produces: `InvestigationOperationStatus::{Pending, Succeeded, Failed, Cancelled}`
- Produces: `NewInvestigationOperation { capture_session_id, parent_request_id, kind, request_before, request_after }`
- Produces: `DbState::begin_investigation_operation(input) -> InvestigationOperation`
- Produces: `DbState::finish_investigation_operation(id, status, response, error) -> InvestigationOperation`

- [ ] **Step 1: Add immutability and transition tests**

Insert a parent Captured Request, begin a Replay, complete it, and assert the parent row bytes are unchanged. Reject an operation whose parent belongs to another Session and reject `Succeeded -> Failed`.

- [ ] **Step 2: Run and confirm the Module/table are absent**

Run: `rtk cargo test -p proxybot --lib investigation::operations`

Expected: FAIL because the operation types and schema do not exist.

- [ ] **Step 3: Add migration and transactional methods**

Create `investigation_operations` with UUID ID, Session ID, parent Request ID, kind, status, before/after JSON, response JSON, error, created/finished timestamps, and foreign keys. Only `Pending -> Succeeded | Failed | Cancelled` is valid.

- [ ] **Step 4: Add domain language and commit**

Define **Investigation Operation** and **Replay Lineage** in `CONTEXT.md`. Then run:

```bash
rtk cargo test -p proxybot --lib investigation::operations
rtk git add CONTEXT.md src-tauri/src/investigation/operations.rs src-tauri/src/investigation/mod.rs src-tauri/src/db.rs
rtk git commit -m "feat: persist investigation operation lineage"
```

### Task 2: Connect safe edit-and-forward

**Files:**
- Modify: `src-tauri/src/commands/breakpoint.rs`
- Modify: `src-tauri/src/state.rs`
- Modify: `proxybot-core/src/types.rs`
- Modify: `proxybot-core/src/proxy_engine.rs`
- Modify: `src-tauri/src/desktop_contract.rs`
- Modify: `src/desktop/contract.ts`
- Modify: `src/components/breakpoint/BreakpointPanel.tsx`
- Modify: `src/test/BreakpointPanel.test.tsx`
- Modify: `src/generated/desktop-contract.ts` through generation

**Interfaces:**
- Produces: `BreakpointMutation { method, url, headers, body }`
- Produces: `resolve_breakpoint({ id, decision: "proceed" | "drop" | "modify", mutation: BreakpointMutation | null }) -> InvestigationOperation | null`
- Produces: original Request evidence plus a child `BreakpointMutation` operation for Modify

- [ ] **Step 1: Write field-safety and UI payload tests**

Assert Modify requires mutation; headers with CR/LF names or values are rejected; URL must be HTTP(S); request target cannot be empty. UI test edits method, URL, one header, and body, previews a diff, then asserts the generated mutation DTO is sent.

- [ ] **Step 2: Run and confirm current read-only editor fails**

```bash
rtk cargo test -p proxybot --lib breakpoint
rtk pnpm exec vitest run src/test/BreakpointPanel.test.tsx
```

Expected: FAIL because fields are read-only and `mutated` is always null.

- [ ] **Step 3: Apply a narrow mutation to the paused runtime request**

Replace sparse `InterceptedRequest` input with `BreakpointMutation`. Build the modified runtime request from the immutable paused snapshot; ignore no fields silently. Persist Pending lineage before sending the decision, then Succeeded when the mutated Request completes or Failed/Cancelled on the corresponding capture event.

- [ ] **Step 4: Make the UI editable with before/after preview**

Protected Host/TLS/connection facts remain read-only. The `Modify and Forward` button is disabled until validation passes and shows `Pending` after submission. Forward and Drop keep their existing explicit actions.

- [ ] **Step 5: Verify and commit edit-and-forward**

```bash
rtk pnpm contract:generate
rtk cargo test -p proxybot-core -p proxybot --locked --no-default-features breakpoint
rtk pnpm exec vitest run src/test/BreakpointPanel.test.tsx src/desktop/contract.test.ts
rtk pnpm typecheck
rtk git add proxybot-core/src/types.rs proxybot-core/src/proxy_engine.rs src-tauri/src/commands/breakpoint.rs src-tauri/src/state.rs src-tauri/src/desktop_contract.rs src/desktop/contract.ts src/generated/desktop-contract.ts src/components/breakpoint/BreakpointPanel.tsx src/test/BreakpointPanel.test.tsx
rtk git commit -m "feat: support traceable edit and forward"
```

### Task 3: Reproduce a selected Captured Request

**Files:**
- Create: `src-tauri/src/investigation/reproduce.rs`
- Modify: `src-tauri/src/replay.rs`
- Modify: `src-tauri/src/commands/compose.rs`
- Modify: `src-tauri/src/desktop_contract.rs`
- Modify: `src/features/investigation/EvidenceInspector.tsx`
- Modify: `src/components/composer/ComposerPage.tsx`
- Modify: `src/components/replay/ReplayPage.tsx`
- Modify: `src/test/ComposerPage.test.tsx`
- Modify: `src/test/ReplayPage.test.tsx`
- Modify: `src/generated/desktop-contract.ts` through generation

**Interfaces:**
- Produces: `ReproductionDraft { parent_request_id, method, url, headers, body }`
- `create_reproduction_draft({ captureSessionId, requestId }) -> ReproductionDraft`
- `send_reproduction({ captureSessionId, draft }) -> ReproductionResult`
- `ReproductionResult { operation_id, response, duration_ms, request_diff, response_diff }`

- [ ] **Step 1: Add correct URL and lineage tests**

For a stored HTTPS Request, assert the draft URL is `https://api.example.com/path`, not scheme-less. Send against a local test server and assert the resulting operation points to the original parent and includes response/timing. Assert private-network validation follows the existing Composer rule and exposes a validation error.

- [ ] **Step 2: Run and confirm current Replay/Composer are disconnected**

```bash
rtk cargo test -p proxybot --lib investigation::reproduce
rtk cargo test -p proxybot --lib replay
rtk cargo test -p proxybot --lib commands::compose
rtk pnpm exec vitest run src/test/ComposerPage.test.tsx src/test/ReplayPage.test.tsx
```

Expected: FAIL because there is no selected-request draft or parent lineage.

- [ ] **Step 3: Implement draft and send commands**

Load the parent by Session and Request IDs, produce a full URL, and persist a Pending Replay/ComposerSend operation. Complete it with exact response facts and deterministic request/response diffs. Timeouts and network errors finish the operation as Failed.

- [ ] **Step 4: Wire Inspector actions**

`Replay` sends the unchanged generated draft after confirmation. `Open in Composer` navigates with an operation draft ID, not raw request data in the URL. Composer reloads the authoritative draft and permits explicit editing before send. Replay destination continues to own history and batch mock comparison and labels the latter `Mock comparison`.

- [ ] **Step 5: Verify and commit reproduction**

```bash
rtk pnpm contract:generate
rtk cargo test -p proxybot --locked --no-default-features investigation::reproduce
rtk cargo test -p proxybot --locked --no-default-features replay
rtk cargo test -p proxybot --locked --no-default-features commands::compose
rtk pnpm exec vitest run src/test/ComposerPage.test.tsx src/test/ReplayPage.test.tsx src/features/investigation/EvidenceInspector.test.tsx
rtk pnpm typecheck
rtk git add src-tauri/src/investigation/reproduce.rs src-tauri/src/replay.rs src-tauri/src/commands/compose.rs src-tauri/src/desktop_contract.rs src/features/investigation/EvidenceInspector.tsx src/components/composer src/components/replay src/desktop/contract.ts src/generated/desktop-contract.ts src/test
rtk git commit -m "feat: reproduce selected captured requests"
```

### Task 4: Implement the default redaction policy

**Files:**
- Create: `src-tauri/src/redaction/mod.rs`
- Create: `src-tauri/src/redaction/policy.rs`
- Create: `src-tauri/src/redaction/tests.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `CONTEXT.md`

**Interfaces:**
- Produces: `RedactionPolicy { version, header_names, query_names, body_field_names, custom_patterns }`
- Produces: `RedactionReport { policy_version, replaced_headers, replaced_query_values, replaced_body_fields, unsupported_bodies, warnings }`
- Produces: `redact_exchange(detail, policy) -> Result<(RedactedExchange, RedactionReport), RedactionError>`

- [ ] **Step 1: Add hostile fixture tests**

Fixtures cover mixed-case Authorization, duplicate Cookie/Set-Cookie, query tokens, nested JSON arrays/objects, form data, encoded values, binary bodies, malformed JSON with a JSON content type, and custom regex. Assert values become `[REDACTED]` without removing structural keys.

- [ ] **Step 2: Run and confirm the Module is absent**

Run: `rtk cargo test -p proxybot --lib redaction`

Expected: FAIL because policy and redactor do not exist.

- [ ] **Step 3: Implement pure redaction with fail-closed warnings**

Default sensitive names include `authorization`, `proxy-authorization`, `cookie`, `set-cookie`, `x-api-key`, `api_key`, `token`, `access_token`, `refresh_token`, `password`, `secret`, and `session`. Unsupported binary content is omitted from the redacted artifact and recorded in `unsupported_bodies`; it is never copied raw.

- [ ] **Step 4: Document exact share semantics and commit**

Define **Redaction Policy** and **Redacted Share** in `CONTEXT.md`. Then run:

```bash
rtk cargo test -p proxybot --lib redaction
rtk git add CONTEXT.md src-tauri/src/lib.rs src-tauri/src/redaction
rtk git commit -m "feat: add default redaction policy"
```

### Task 5: Apply redaction before HAR and copy output

**Files:**
- Modify: `src-tauri/src/har.rs`
- Create: `src-tauri/src/investigation/share.rs`
- Modify: `src-tauri/src/desktop_contract.rs`
- Create: `src/features/investigation/RedactedShareDialog.tsx`
- Create: `src/features/investigation/RedactedShareDialog.test.tsx`
- Modify: `src/features/investigation/EvidenceInspector.tsx`
- Modify: `src/generated/desktop-contract.ts` through generation

**Interfaces:**
- `preview_redacted_share({ query, format }) -> RedactedSharePreview`
- `write_redacted_share({ previewId, outputPath }) -> RedactedShareResult`
- `copy_redacted_request({ captureSessionId, requestId }) -> { text, report }`
- `RedactedSharePreview { preview_id, session_ids, record_count, report, expires_at }`

- [ ] **Step 1: Add export-before-write tests**

Assert a HAR preview contains no secret fixture and reports replacements. Assert write rejects an expired or mismatched preview ID. Assert raw `export_har` is not used by the Core dialog.

- [ ] **Step 2: Run and confirm raw HAR leaks fixtures**

```bash
rtk cargo test -p proxybot --lib har
rtk cargo test -p proxybot --lib investigation::share
rtk pnpm exec vitest run src/features/investigation/RedactedShareDialog.test.tsx
```

Expected: FAIL because existing HAR clones raw headers/bodies and no preview exists.

- [ ] **Step 3: Redact before serialization and bind preview to scope**

Query records by explicit InvestigationQuery, redact each exchange, serialize HAR from redacted values, and hash the Session IDs, record IDs, query, policy version, and generated bytes into `preview_id`. Keep previews in memory for 10 minutes; write uses the exact bytes previewed.

- [ ] **Step 4: Build the share dialog**

Show Session scope, count, policy version, replacement totals, unsupported bodies, and warnings. Primary action is `Export Redacted`; copy returns already-redacted text. No raw toggle exists in Core.

- [ ] **Step 5: Verify and commit safe sharing**

```bash
rtk pnpm contract:generate
rtk cargo test -p proxybot --locked --no-default-features redaction
rtk cargo test -p proxybot --locked --no-default-features har
rtk cargo test -p proxybot --locked --no-default-features investigation::share
rtk pnpm exec vitest run src/features/investigation/RedactedShareDialog.test.tsx src/features/investigation/EvidenceInspector.test.tsx
rtk pnpm typecheck
rtk git add src-tauri/src/har.rs src-tauri/src/investigation/share.rs src-tauri/src/desktop_contract.rs src/generated/desktop-contract.ts src/features/investigation
rtk git commit -m "feat: export redacted investigation evidence"
```

### Task 6: Prove the completed Core journey

**Files:**
- Modify: `src-tauri/src/acceptance.rs`
- Modify: `scripts/test_desktop_acceptance.mjs`
- Modify: `e2e/navigation.spec.ts`
- Create: `e2e/investigation-core.spec.ts`

**Interfaces:**
- Produces acceptance report schema version 3 with mutation/reproduction lineage and redaction counts

- [ ] **Step 1: Add failing report and browser journey assertions**

Require the packaged report to prove original Request hash unchanged, one child reproduction operation, and zero occurrences of the planted secret in exported bytes. Browser E2E starts at a Session, selects a Request, opens Response, opens Composer from evidence, and previews a redacted share.

- [ ] **Step 2: Run against the previous acceptance report**

```bash
rtk pnpm test:e2e
rtk pnpm test:desktop:acceptance
```

Expected: FAIL until the new journey/report is implemented and a current app bundle exists.

- [ ] **Step 3: Extend the isolated packaged journey**

Use a local HTTP test origin, reproduce the captured Request without external network, plant a secret header/body, export to the isolated workspace, and record SHA-256 values and redaction report in the acceptance JSON.

- [ ] **Step 4: Run the full batch gate and commit**

```bash
rtk pnpm ci:local
rtk pnpm test:e2e
rtk pnpm exec tauri build --bundles app
rtk pnpm test:desktop:acceptance
rtk git add src-tauri/src/acceptance.rs scripts/test_desktop_acceptance.mjs e2e/navigation.spec.ts e2e/investigation-core.spec.ts
rtk git commit -m "test: prove the core investigation journey"
```
