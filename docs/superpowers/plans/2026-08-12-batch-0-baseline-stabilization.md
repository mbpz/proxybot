# Batch 0 Baseline Stabilization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Finish the existing Alerts Desktop Contract slice, eliminate its false empty-input scan, and make every hosted CI job execute with one pnpm version authority.

**Architecture:** This batch changes no product architecture. It finishes the already-started vertical slice through the generated Desktop Contract, adds a repository test for workflow package-manager drift, and preserves explicit loading/error/retry state. The working tree at plan start contains the Alerts slice and must be reviewed as one owned change set before staging.

**Tech Stack:** Rust, Tauri Desktop Contract generator, React/TypeScript, Vitest, Playwright, Node test runner, GitHub Actions, pnpm 10.33.0

## Global Constraints

- Stable baseline at design time is `c492759`; the current Alerts changes are uncommitted and are not yet stable capability.
- Do not add Activity, CaptureSession persistence, or workspace redesign in this batch.
- `scan_request_anomalies` may remain a typed command for a later evidence-driven caller, but the Alerts page must never call it with an empty host and bodies.
- `package.json#packageManager` is the only pnpm version declaration.
- Every shell command starts with `rtk`; stage exact paths only.

---

### Task 1: Close the Alerts false-scan path

**Files:**
- Modify: `src/components/alerts/AlertsPage.tsx`
- Modify: `src/test/AlertsPage.test.tsx`
- Verify: `src-tauri/src/anomaly.rs`
- Verify: `src-tauri/src/desktop_contract.rs`
- Verify: `src/desktop/contract.ts`

**Interfaces:**
- Consumes: `DesktopContract.call("get_alerts" | "get_alert_count" | "get_traffic_baseline", args)`
- Produces: `AlertsPage({ contract?: DesktopContract })` with independent alerts and baseline retry states and no context-free anomaly scan action

- [ ] **Step 1: Add the failing user-visible test**

Add this assertion to the Alerts page rendering test after all initial reads settle:

```tsx
expect(screen.queryByRole("button", { name: /scan now/i })).not.toBeInTheDocument();
expect(adapter.calls.some(({ command }) => command === "scan_request_anomalies")).toBe(false);
```

- [ ] **Step 2: Run the focused test and confirm the current button fails it**

Run: `rtk pnpm exec vitest run src/test/AlertsPage.test.tsx`

Expected: FAIL because `Scan Now` is rendered or `scan_request_anomalies` is called.

- [ ] **Step 3: Remove only the context-free scan UI and state**

Delete `scanning`, `scanNow`, and the `Scan Now` button from `AlertsPage`. Keep the generated anomaly DTO and command registered for Batch 5, where a real Captured Request supplies `host`, `ip`, `reqBody`, and `respBody`.

- [ ] **Step 4: Verify the Alerts contract and UI slice**

Run:

```bash
rtk pnpm contract:check
rtk pnpm exec vitest run src/desktop/contract.test.ts src/test/AlertsPage.test.tsx
rtk cargo test -p proxybot --locked --no-default-features desktop_contract
```

Expected: all commands exit 0; invalid baseline/anomaly payload fixtures still reject with `DesktopError`.

- [ ] **Step 5: Commit the completed Alerts slice**

Review `rtk git diff --check` and verify the staged names are exactly the existing 14 Alerts/docs/E2E paths listed below. Then run:

```bash
rtk git add docs/architecture.md docs/roadmap.md e2e/navigation.spec.ts e2e/ssl-bypass.spec.ts e2e/topology.spec.ts e2e/ws-frames.spec.ts src-tauri/src/anomaly.rs src-tauri/src/desktop_contract.rs src-tauri/tests/desktop_contract.rs src/components/alerts/AlertsPage.tsx src/desktop/contract.test.ts src/desktop/contract.ts src/generated/desktop-contract.ts src/test/AlertsPage.test.tsx
rtk git commit -m "feat: migrate alerts to desktop contract"
```

### Task 2: Make pnpm workflow drift testable

**Files:**
- Create: `scripts/workflow-config.test.mjs`
- Modify: `package.json`
- Test: `scripts/workflow-config.test.mjs`

**Interfaces:**
- Consumes: `.github/workflows/ci.yml`, `.github/workflows/release.yml`, and `package.json#packageManager`
- Produces: `pnpm test:workflow-config`, a zero-network repository configuration gate

- [ ] **Step 1: Write the failing Node test**

Create the following test:

```js
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const pkg = JSON.parse(readFileSync(new URL("../package.json", import.meta.url), "utf8"));
const workflows = ["ci.yml", "release.yml"].map((name) =>
  readFileSync(new URL(`../.github/workflows/${name}`, import.meta.url), "utf8"),
);

test("package.json is the only pnpm version authority", () => {
  assert.match(pkg.packageManager, /^pnpm@10\.33\.0\+/);
  for (const workflow of workflows) {
    assert.doesNotMatch(
      workflow,
      /pnpm\/action-setup@v4[\s\S]{0,120}\n\s+with:\n\s+version:/,
    );
  }
});
```

Add `"test:workflow-config": "node --test scripts/workflow-config.test.mjs"` and run it.

- [ ] **Step 2: Confirm it fails against current workflow duplication**

Run: `rtk pnpm test:workflow-config`

Expected: FAIL because both workflows specify `with: version: 10`.

- [ ] **Step 3: Remove the duplicate version blocks**

In every `pnpm/action-setup@v4` step in CI and Release, retain only:

```yaml
- name: Setup pnpm
  uses: pnpm/action-setup@v4
```

Do not change `packageManager` or the Node 20 floor in this task.

- [ ] **Step 4: Run the workflow and version identity gates**

Run:

```bash
rtk pnpm test:workflow-config
rtk pnpm version:check
```

Expected: both exit 0.

- [ ] **Step 5: Commit the workflow correction**

```bash
rtk git add .github/workflows/ci.yml .github/workflows/release.yml package.json scripts/workflow-config.test.mjs
rtk git commit -m "ci: use package manager pnpm version"
```

### Task 3: Run the full local baseline gate

**Files:**
- No file changes are expected; a failure returns work to Task 1 or Task 2 before this gate is rerun

**Interfaces:**
- Consumes: repository scripts and all supported local test Adapters
- Produces: a recorded green baseline suitable for Batch 1

- [ ] **Step 1: Run generated-contract and Rust checks**

```bash
rtk pnpm contract:check
rtk cargo fmt --all -- --check
rtk cargo test --workspace --locked --no-default-features
rtk cargo clippy --workspace --all-targets --locked --no-default-features -- -D warnings
```

Expected: all exit 0.

- [ ] **Step 2: Run frontend and browser checks**

```bash
rtk pnpm typecheck
rtk pnpm test:ui
rtk pnpm build
rtk pnpm test:e2e
```

Expected: all exit 0. A Playwright failure must retain its report and be fixed only within the owned Alerts/navigation fixture scope.

- [ ] **Step 3: Run the release-like unsigned bundle smoke**

Run:

```bash
rtk pnpm exec tauri build --bundles app
rtk pnpm test:desktop:acceptance
```

Expected: the packaged process reports a decrypted HTTPS Captured Request and clean stop/restart.

- [ ] **Step 4: Verify the final commit and working-tree boundary**

Run:

```bash
rtk git log -2 --oneline
rtk git status --short
```

Expected: Task 1 and Task 2 commits are present. Any remaining modification is explicitly identified as unrelated before Batch 1 begins.

This verification task ends without changing files or creating a commit. A failing check is fixed and committed in its owning Task 1 or Task 2 before the complete gate is rerun.

### Task 4: Confirm hosted CI reaches real tests

**Files:**
- No repository files; hosted failures return work to the exact owning task and commit

**Interfaces:**
- Consumes: GitHub Actions run for the pushed Batch 0 commits
- Produces: hosted evidence that all four CI jobs pass package-manager setup and execute their test commands

- [ ] **Step 1: Push the Batch 0 commits after local gates pass**

Run: `rtk git push origin main`

Expected: push succeeds and starts the CI workflow.

- [ ] **Step 2: Inspect the run through GitHub CLI**

Run:

```bash
rtk gh run list --workflow CI --limit 3
rtk gh run watch --exit-status
```

Expected: Rust Tests, Frontend Tests, Tauri Bundle Smoke, and E2E Tests all proceed beyond setup. The batch gate is green only when the run exits 0; otherwise capture the failing job log before any fix.

This evidence task ends without changing files or creating a commit. A reproducible workflow defect returns to Task 2, receives a failing repository test and fix there, and then reruns Tasks 3–4.
