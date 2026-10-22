# Batch 1 Desktop Contract Convergence Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the generated Desktop Contract the only production React desktop Interface and correct the Graph/DAG wire-shape defect before investigation UI work begins.

**Architecture:** The Rust composition root remains the canonical command registry. Rust-first wire DTOs are generated into TypeScript, the Tauri and Browser Mock Adapters satisfy one Interface, and a repository gate rejects direct Tauri imports outside the Adapter. Migration proceeds by product slice so every commit remains runnable.

**Tech Stack:** Rust macros and serde, Tauri 2, TypeScript, BrowserMockAdapter, Vitest, Playwright, Node test runner

## Global Constraints

- Begin only after Batch 0 is committed and locally green.
- Do not combine Graph and DAG merely because both can be drawn; preserve distinct semantics and DTOs.
- Do not convert a failure to `null`, an empty list, or a successful empty state.
- Raw `invoke` and `listen` may remain only in `src/desktop/contract.ts`; `safeInvoke` is deleted at the final gate.
- Use generated camelCase command arguments at React call sites; Tauri owns Rust snake_case deserialization.
- Every shell command starts with `rtk`; stage exact paths only.

---

### Task 1: Make registry completeness an executable invariant

**Files:**
- Modify: `src-tauri/src/bootstrap.rs`
- Modify: `src-tauri/src/desktop_contract.rs`
- Modify: `src-tauri/tests/desktop_contract.rs`

**Interfaces:**
- Consumes: `bootstrap::DESKTOP_COMMANDS`
- Produces: `desktop_contract::contract_coverage() -> ContractCoverage { migrated, pending }`
- Invariant: `migrated ∪ pending` equals the composition-root registry, the sets do not overlap, and a command moves from pending to migrated only with generated args/result types and validation

- [ ] **Step 1: Add the failing completeness assertion**

Add a test that compares the union of explicitly migrated and pending commands to the actual handler registry:

```rust
#[test]
fn every_registered_desktop_command_is_classified() {
    let registered = proxybot::bootstrap::DESKTOP_COMMANDS
        .iter()
        .map(|path| path.rsplit("::").next().unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    let coverage = proxybot::desktop_contract::contract_coverage();
    assert!(coverage.migrated.is_disjoint(&coverage.pending));
    assert_eq!(
        coverage.migrated.union(&coverage.pending).copied().collect(),
        registered,
    );
}
```

Expose `bootstrap` and `DESKTOP_COMMANDS` as `pub(crate)`/`pub` only as required by the integration test.

- [ ] **Step 2: Run the test and confirm missing contracts are listed**

Run: `rtk cargo test -p proxybot --test desktop_contract every_registered_desktop_command_is_classified -- --exact`

Expected: FAIL because the 95 registered-but-unmigrated commands are not classified.

- [ ] **Step 3: Add the explicit pending migration set**

Keep the existing generated `desktopCommandNames` as migrated-only. Add `PENDING_DESKTOP_COMMANDS` containing the exact 95 names reported by the failing test. Implement:

```rust
pub struct ContractCoverage {
    pub migrated: BTreeSet<&'static str>,
    pub pending: BTreeSet<&'static str>,
}
```

The test failure prints missing/extra/overlap names. A command is not considered migrated until its args/result declaration, validator, Browser fixture, and React caller all exist.

- [ ] **Step 4: Regenerate and run contract tests**

```bash
rtk pnpm contract:check
rtk cargo test -p proxybot --test desktop_contract every_registered_desktop_command_is_classified -- --exact
rtk git add src-tauri/src/bootstrap.rs src-tauri/src/desktop_contract.rs src-tauri/tests/desktop_contract.rs
rtk git commit -m "test: classify desktop contract migration"
```

Expected: PASS with 36 migrated and 95 pending commands. Every later migration removes names from `PENDING_DESKTOP_COMMANDS`; Task 5 deletes the pending set after it reaches zero.

### Task 2: Correct Graph and DAG wire semantics

**Files:**
- Modify: `src-tauri/src/commands/graph.rs`
- Modify: `src-tauri/src/dag.rs`
- Modify: `src-tauri/src/desktop_contract.rs`
- Modify: `src/components/graph/types.ts`
- Modify: `src/components/graph/GraphPage.tsx`
- Modify: `src/test/GraphPage.test.tsx`
- Modify: `src/generated/desktop-contract.ts` through generation

**Interfaces:**
- Produces: `get_graph_data(args: { maxRequests: number }) -> RequestDependencyGraph`
- Produces: `get_traffic_dag(args: {}) -> TokenDependencyDag`
- Produces: `get_device_dag(args: { deviceId: number }) -> TokenDependencyDag`
- Produces: UI state `{ kind: "request-dependencies"; graph: RequestDependencyGraph } | { kind: "token-dag"; graph: TokenDependencyDag }`

- [ ] **Step 1: Add incompatible-payload contract tests**

Add Browser Adapter tests:

```ts
await expect(
  new BrowserMockAdapter({ get_traffic_dag: () => ({ requests: [], edges: [] }) })
    .contract.call("get_traffic_dag", {}),
).rejects.toMatchObject({ code: "invalid_result" });

await expect(
  new BrowserMockAdapter({ get_graph_data: () => ({ nodes: [], edges: [], adjacency_list: {} }) })
    .contract.call("get_graph_data", { maxRequests: 100 }),
).rejects.toMatchObject({ code: "invalid_result" });
```

- [ ] **Step 2: Confirm raw generic assertions currently allow the defect**

Run: `rtk pnpm exec vitest run src/test/GraphPage.test.tsx src/desktop/contract.test.ts`

Expected: FAIL because commands/types/validators do not yet distinguish the payloads.

- [ ] **Step 3: Generate two Rust-first DTO families**

Annotate the existing Graph request/edge types and DAG node/edge/container types with `desktop_contract_type!`. Export them as `RequestDependencyGraph` and `TokenDependencyDag`; do not introduce an untyped `GraphData` alias. Add both command signatures to `DesktopCommands` and exact runtime validators.

- [ ] **Step 4: Remove Graph fallback and render discriminated state**

`GraphPage` calls one selected command at a time through its injected `DesktopContract`. A failed DAG build shows a retryable error; it does not call `get_graph_data` as a semantic fallback. Use the discriminated union in the Interfaces block to choose the renderer.

- [ ] **Step 5: Verify and commit the correctness slice**

```bash
rtk pnpm contract:generate
rtk pnpm typecheck
rtk pnpm exec vitest run src/test/GraphPage.test.tsx src/desktop/contract.test.ts
rtk cargo test -p proxybot --locked --no-default-features graph
rtk cargo test -p proxybot --locked --no-default-features dag
rtk cargo test -p proxybot --locked --no-default-features desktop_contract
rtk git add src-tauri/src/commands/graph.rs src-tauri/src/dag.rs src-tauri/src/desktop_contract.rs src/components/graph/types.ts src/components/graph/GraphPage.tsx src/test/GraphPage.test.tsx src/desktop/contract.test.ts src/desktop/contract.ts src/generated/desktop-contract.ts
rtk git commit -m "fix: separate graph and dag desktop contracts"
```

### Task 3: Introduce structured desktop errors and cancellation

**Files:**
- Create: `src-tauri/src/desktop_error.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/proxy/commands.rs`
- Modify: `src-tauri/src/db.rs`
- Modify: `src-tauri/src/commands/device_setup.rs`
- Modify: `src-tauri/src/desktop_contract.rs`
- Modify: `src/desktop/contract.ts`
- Modify: `src/desktop/contract.test.ts`
- Modify: `src/generated/desktop-contract.ts` through generation

**Interfaces:**
- Produces: `DesktopErrorKind = "validation" | "unavailable" | "permission" | "conflict" | "timeout" | "cancelled" | "persistence" | "internal"`
- Produces: `DesktopCommandError { kind, code, message, retryable, context }`
- Produces: `DesktopContract.call(command, args, options?: { signal?: AbortSignal })`

- [ ] **Step 1: Add structured-error and cancellation tests**

Add Contract tests that reject this Adapter payload as a typed non-retryable validation error and cancel an unresolved call:

```ts
const error = {
  kind: "validation",
  code: "invalid_request_id",
  message: "Captured Request id is invalid",
  retryable: false,
  context: { field: "requestId" },
};
await expect(adapter.contract.call("get_request_detail", { id: "bad" }))
  .rejects.toMatchObject(error);

const controller = new AbortController();
const pending = adapter.contract.call("get_devices", {}, { signal: controller.signal });
controller.abort();
await expect(pending).rejects.toMatchObject({ kind: "cancelled", code: "aborted" });
```

- [ ] **Step 2: Run and confirm the legacy error model fails**

Run: `rtk pnpm exec vitest run src/desktop/contract.test.ts`

Expected: FAIL because the current Interface has only transport/contract/command kinds, no retryability, and no call options.

- [ ] **Step 3: Add the Rust wire error and Tauri normalization**

`DesktopCommandError` implements `Serialize` and `Display`. Add constructors `validation`, `unavailable`, `permission`, `conflict`, `timeout`, `persistence`, and `internal`. Convert the three listed command files first. Add `PENDING_LEGACY_ERROR_COMMANDS` initialized from every other registered command returning `Result<_, String>`; extend the coverage test so each registered command is either structured or explicitly pending, never both.

- [ ] **Step 4: Add AbortSignal semantics at the Contract Seam**

The Contract rejects immediately if the signal is already aborted. For an in-flight Tauri call, abort marks the invocation inactive and rejects with `cancelled`; a late Adapter result is discarded. `subscribe` disposal retains its existing behavior. Timeout remains a Rust/command policy, not a hidden frontend timer.

- [ ] **Step 5: Generate, verify, and commit the error Interface**

```bash
rtk pnpm contract:generate
rtk pnpm exec vitest run src/desktop/contract.test.ts
rtk cargo test -p proxybot --locked --no-default-features desktop_error
rtk cargo test -p proxybot --test desktop_contract
rtk git add src-tauri/src/desktop_error.rs src-tauri/src/lib.rs src-tauri/src/proxy/commands.rs src-tauri/src/db.rs src-tauri/src/commands/device_setup.rs src-tauri/src/desktop_contract.rs src/desktop/contract.ts src/desktop/contract.test.ts src/generated/desktop-contract.ts
rtk git commit -m "feat: structure desktop errors and cancellation"
```

### Task 4: Migrate Core and Advanced production callers

**Files:**
- Modify: `src-tauri/src/desktop_contract.rs`
- Modify: `src/desktop/contract.ts`
- Modify: `src/hooks/useCa.ts`
- Modify: `src/hooks/useNetwork.ts`
- Modify: `src/components/devices/DevicesPage.tsx`
- Modify: `src/components/certs/CertsPage.tsx`
- Modify: `src/components/certs/DecryptionRules.tsx`
- Modify: `src/components/breakpoint/BreakpointPanel.tsx`
- Modify: `src/components/replay/ReplayPage.tsx`
- Modify: `src/components/composer/ComposerPage.tsx`
- Modify: `src/components/topology/TopologyDetail.tsx`
- Modify: `src/components/topology/TopologyFilter.tsx`
- Modify: `src/components/topology/hooks/useTopologyGraph.ts`
- Modify: `src/test/DevicesPage.test.tsx`
- Modify: `src/test/CertsPage.test.tsx`
- Modify: `src/test/ReplayPage.test.tsx`
- Modify: `src/test/ComposerPage.test.tsx`
- Create: `src/test/BreakpointPanel.test.tsx`
- Modify: `src/components/topology/__tests__/TopologyFilter.test.tsx`
- Modify: `src/components/topology/__tests__/TopologyDetail.test.tsx`
- Modify: `src/components/topology/__tests__/useTopologyGraph.test.ts`
- Modify: `src/generated/desktop-contract.ts` through generation

**Interfaces:**
- Consumes: injected `DesktopContract`
- Produces: generated DTOs for CA, Network, Device, TLS Rule, Breakpoint, Replay, Composer, and Topology commands/events
- Produces: explicit query state `idle | loading | ready | empty | stale | error` in migrated hooks/pages

- [ ] **Step 1: Add one failed-operation test per product slice**

Each page test injects a `BrowserMockAdapter` handler that throws and asserts a visible retry action. Use this exact error shape:

```ts
throw new DesktopError("command", "get_devices", "device_read_failed", "Could not load devices");
```

The page must show `Could not load devices`; it must not show a successful empty-device state.

- [ ] **Step 2: Confirm the direct callers fail the injected-Adapter tests**

Run:

```bash
rtk pnpm exec vitest run src/test/DevicesPage.test.tsx src/test/CertsPage.test.tsx src/test/ReplayPage.test.tsx src/test/ComposerPage.test.tsx src/components/topology/__tests__
```

Expected: FAIL where components still import raw Tauri or `safeInvoke`.

- [ ] **Step 3: Add Rust-first DTOs and command signatures by slice**

For every listed caller, export the command's actual Rust args/result type, add its validator, return `DesktopCommandError`, remove the name from both pending sets, regenerate, and replace snake_case call arguments with generated camelCase fields. Keep view-only state local to the React Module.

- [ ] **Step 4: Inject the contract and preserve explicit failures**

Each page accepts `{ contract = desktop }`; hooks accept a `DesktopContract` argument from their owning page. Replace `null` fallbacks with explicit `error` state and retry. Breakpoint event subscription moves to `contract.subscribe("breakpoint:new", ...)`.

- [ ] **Step 5: Verify and commit the Core/Advanced migration**

```bash
rtk pnpm contract:generate
rtk pnpm typecheck
rtk pnpm test:ui
rtk pnpm test:e2e
rtk cargo test -p proxybot --locked --no-default-features desktop_contract
rtk git add src-tauri/src/desktop_contract.rs src/desktop/contract.ts src/generated/desktop-contract.ts src/hooks src/components/devices src/components/certs src/components/breakpoint src/components/replay src/components/composer src/components/topology src/test e2e
rtk git commit -m "refactor: route core desktop calls through contract"
```

### Task 5: Migrate Labs callers and finish registry completeness

**Files:**
- Modify: `src-tauri/src/desktop_contract.rs`
- Modify: `src/desktop/contract.ts`
- Modify: `src/stores/sslBypassStore.tsx`
- Modify: `src/components/ssl-bypass/**`
- Modify: `src/components/ai/**`
- Modify: `src/components/gen/GenPage.tsx`
- Modify: `src/components/deploy/**`
- Modify: `src/test/AiPage.test.tsx`
- Modify: `src/test/GenPage.test.tsx`
- Modify: `src/test/DeployPage.test.tsx`
- Modify: `src/test/SettingsPage.test.tsx`
- Create: `src/stores/sslBypassStore.test.tsx`
- Modify: `src/generated/desktop-contract.ts` through generation

**Interfaces:**
- Produces: generated DTOs and events for SSL bypass, AI/inference, generation, vision, deployment, workspace, network-condition, app-signature, and Spec Generation commands
- Produces: full equality between `bootstrap::DESKTOP_COMMANDS` and generated `desktopCommandNames`

- [ ] **Step 1: Add invalid-payload and unavailable-capability fixtures**

For each Labs family, add at least one success fixture matching real Rust serialization and one invalid fixture rejected by the Contract. For disabled Frida, assert the command rejects visibly with an unavailable error rather than returning an empty device list.

- [ ] **Step 2: Run the completeness test to obtain the exact remaining command names**

Run: `rtk cargo test -p proxybot --test desktop_contract every_registered_desktop_command_is_classified -- --exact`

Expected: PASS and report the remaining pending names as Labs/workspace commands owned by this task.

- [ ] **Step 3: Generate the remaining wire DTOs and migrate callers**

Add declarations, validators, structured Rust errors, and injected contract usage for every pending name reported by Step 2, removing each migrated name from `PENDING_DESKTOP_COMMANDS` and `PENDING_LEGACY_ERROR_COMMANDS`. Preserve domain distinctions: CaptureSession is not SpecGenerationRun, and InstrumentationSession uses `instrumentationSessionId` in TypeScript even if Rust migration of stored names waits for Batch 2. Delete both pending constants when empty and change the coverage test to require `registered == migrated == structured_errors`.

- [ ] **Step 4: Prove registry equality and commit**

```bash
rtk pnpm contract:generate
rtk pnpm contract:check
rtk cargo test -p proxybot --test desktop_contract
rtk pnpm typecheck
rtk pnpm test:ui
rtk git add src-tauri/src/bootstrap.rs src-tauri/src/desktop_contract.rs src-tauri/tests/desktop_contract.rs src/desktop src/generated/desktop-contract.ts src/stores/sslBypassStore.tsx src/components/ssl-bypass src/components/ai src/components/gen src/components/deploy src/test
rtk git commit -m "refactor: complete generated desktop contract"
```

### Task 6: Enforce the single Adapter Seam

**Files:**
- Create: `scripts/check-desktop-adapter.mjs`
- Create: `scripts/check-desktop-adapter.test.mjs`
- Modify: `package.json`
- Delete: `src/utils/safeInvoke.ts`
- Modify: `.github/workflows/ci.yml`

**Interfaces:**
- Produces: `pnpm contract:adapter-check`
- Allows: raw `@tauri-apps/api/core` and `@tauri-apps/api/event` imports only in `src/desktop/contract.ts`

- [ ] **Step 1: Write the failing scanner test**

The test creates a temporary source tree with one allowed Adapter and one forbidden caller, runs the scanner, and asserts the forbidden relative path appears in stderr. The scanner walks `.ts`/`.tsx` files and rejects imports matching:

```js
/@tauri-apps\/api\/(core|event)|(?:^|\/)safeInvoke(?:\.ts)?/
```

- [ ] **Step 2: Run the scanner against the repository**

Run: `rtk node scripts/check-desktop-adapter.mjs src`

Expected: FAIL until every direct caller is migrated and `safeInvoke.ts` is deleted.

- [ ] **Step 3: Wire the gate into local and hosted CI**

Add:

```json
"contract:adapter-check": "node scripts/check-desktop-adapter.mjs src",
"test:adapter-check": "node --test scripts/check-desktop-adapter.test.mjs"
```

Run both immediately after `contract:check` in `ci:local` and the hosted Rust/frontend contract gate.

- [ ] **Step 4: Verify the complete Seam and commit**

```bash
rtk pnpm contract:adapter-check
rtk pnpm test:adapter-check
rtk pnpm ci:local
rtk pnpm test:e2e
rtk git add scripts/check-desktop-adapter.mjs scripts/check-desktop-adapter.test.mjs package.json .github/workflows/ci.yml src
rtk git commit -m "test: enforce desktop adapter seam"
```
