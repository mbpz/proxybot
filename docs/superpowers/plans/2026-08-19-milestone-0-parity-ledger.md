# Milestone 0 Parity Ledger Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build an auditable, machine-checked clean-room ledger for every Rockxy Community capability that ProxyBot intends to reproduce or explicitly exclude.

**Architecture:** A Markdown document contains one fenced JSON parity manifest so humans and Node tooling consume the same source of truth. A dependency-free Node checker validates the pinned Rockxy reference, evidence strength, inventory coverage, ownership, acceptance criteria, local evidence paths, and completion claims; narrative evidence and roadmap documents explain the legal/product boundary without becoming a second status authority.

**Tech Stack:** Node.js ESM, `node:test`, Markdown, JSON, pnpm, Git

**Spec:** `docs/superpowers/specs/2026-08-19-rockxy-community-parity-design.md`

## Global Constraints

- Reference Rockxy snapshot: `RockxyApp/Rockxy@6a676d631820b577cf3a651c78d856733a7df995` captured on `2026-08-19`.
- ProxyBot planning baseline: `a3596c777335eaca5c323a445b76a4e127704bf2`; current implementation branch starts from the committed parity design and milestone plan.
- Reproduce public Community behavior only; do not copy AGPL code, tests, fixtures, copy, icons, images, or private/Pro behavior.
- Keep ProxyBot's MIT license, product identity, Rust MITM Runtime, Tauri desktop shell, and React/TypeScript UI.
- Rockxy target evidence for a Community-scope row must be at least `source-backed`; a pinned observable build may instead use `observable-build` or `release-proven`.
- A ProxyBot `Present` claim requires `test-backed` or `release-proven` evidence and at least one existing `test:` path.
- The only machine-readable status authority is the fenced JSON manifest in `docs/parity/rockxy-community-matrix.md`.
- The checker must remain dependency-free and must never fetch the network; all Rockxy URLs are reviewed and pinned before commit.
- Every shell command starts with `rtk`.
- Every implementation task follows RED -> GREEN -> refactor, receives an independent review, stages exact paths, and commits separately.

---

### Task 1: Parity matrix contract checker

**Files:**

- Create: `scripts/check_parity_matrix.mjs`
- Create: `scripts/check_parity_matrix.test.mjs`

**Interfaces:**

- Consumes: a Markdown path from `process.argv[2]`, defaulting to `docs/parity/rockxy-community-matrix.md`.
- Produces: exit `0` and `parity matrix: <count> capabilities validated against RockxyApp/Rockxy@<commit>` on success; exit `1` with one `parity matrix: ...` line per violation on failure.
- Manifest envelope: `{ schema_version: 1, reference: { repository, commit, captured_at, source_license, public_artifact, excluded_artifacts }, capabilities: Capability[] }`.
- Capability fields: `id`, `category`, `capability`, `scope`, `target_evidence_grade`, `target_evidence`, `proxybot_status`, `proxybot_evidence_grade`, `proxybot_evidence`, `owner`, `acceptance`.
- Allowed scopes: `community`, `private`, `future`.
- Allowed evidence grades in ascending order: `documented`, `source-backed`, `test-backed`, `observable-build`, `release-proven`.
- Allowed statuses: `Present`, `Partial`, `Missing`, `Out-of-scope private`, `Future-not-shipped`.
- Required inventory categories: `capture`, `filtering`, `focus-noise`, `workspaces`, `assistant`, `mcp`, `setup`, `certificates`, `proxy-rules`, `compose-compare`, `sessions-export`, `scripting`, `protocols`, `logs`, `nearby-transfer`, `updates`, `security`, `accessibility`, `performance`.

- [ ] **Step 1: Write the CLI behavior tests first**

Use `node:test`, `spawnSync`, `mkdtempSync`, and `writeFileSync`. Build a literal valid manifest containing one row per required category; each row uses a unique `RXC-###` ID, the fixed 40-character commit, a URL of the form `https://github.com/RockxyApp/Rockxy/blob/<commit>/README.md`, `Partial`, `source-backed`, `source:package.json`, owner `M1`, and a hand-written acceptance statement longer than 24 characters. The tests invoke the actual checker process and cover these mutations independently:

```js
test("accepts a complete pinned parity manifest", () => {
  const result = runChecker(validManifest());
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /19 capabilities validated/);
});

test("rejects duplicate capability IDs", () => {
  const manifest = validManifest();
  manifest.capabilities[1].id = manifest.capabilities[0].id;
  assertFailure(manifest, "duplicate capability id RXC-001");
});

test("rejects a row without an owner", () => {
  const manifest = validManifest();
  manifest.capabilities[0].owner = "";
  assertFailure(manifest, "capability RXC-001 is missing owner");
});

test("rejects a row without independent acceptance criteria", () => {
  const manifest = validManifest();
  manifest.capabilities[0].acceptance = "";
  assertFailure(manifest, "capability RXC-001 is missing acceptance criteria");
});

test("rejects floating Rockxy references", () => {
  const manifest = validManifest();
  manifest.capabilities[0].target_evidence[0] =
    "https://github.com/RockxyApp/Rockxy/blob/main/README.md";
  assertFailure(manifest, "floating Rockxy reference");
});

test("rejects documented-only evidence for Community parity scope", () => {
  const manifest = validManifest();
  manifest.capabilities[0].target_evidence_grade = "documented";
  assertFailure(manifest, "target evidence grade documented is below source-backed");
});

test("rejects unsupported Present claims", () => {
  const manifest = validManifest();
  manifest.capabilities[0].proxybot_status = "Present";
  assertFailure(manifest, "Present claim requires test-backed or release-proven ProxyBot evidence");
});

test("rejects a missing inventory category", () => {
  const manifest = validManifest();
  manifest.capabilities = manifest.capabilities.filter((row) => row.category !== "performance");
  assertFailure(manifest, "missing required inventory category: performance");
});
```

- [ ] **Step 2: Run the focused tests and confirm strict RED**

Run: `rtk node --test scripts/check_parity_matrix.test.mjs`

Expected: FAIL because `scripts/check_parity_matrix.mjs` does not exist or cannot validate the fixtures.

- [ ] **Step 3: Implement the dependency-free checker**

Extract exactly one JSON object between these markers:

````markdown
<!-- parity-matrix:start -->
```json
{ "schema_version": 1, "reference": {}, "capabilities": [] }
```
<!-- parity-matrix:end -->
````

Implement these validations without fetching Rockxy:

1. The envelope is present, JSON parses, `schema_version === 1`, repository is exactly `RockxyApp/Rockxy`, commit matches `/^[0-9a-f]{40}$/`, and capture date is `YYYY-MM-DD`.
2. Every capability has the exact required fields, an ID matching `/^RXC-[0-9]{3}$/`, a unique ID, a required category, owner matching `/^M(?:[0-9]|1[0-7])$/`, non-empty evidence arrays, and acceptance text of at least 24 trimmed characters.
3. Every Rockxy target URL begins `https://github.com/RockxyApp/Rockxy/` and contains `/blob/<reference.commit>/` or `/tree/<reference.commit>/`; reject `/main/`, `/master/`, and `ref=main|master` explicitly.
4. `community` rows require target evidence rank at least `source-backed` and status `Present`, `Partial`, or `Missing`; `private` rows require `Out-of-scope private`; `future` rows require `Future-not-shipped`.
5. All `source:`, `test:`, and `docs:` ProxyBot evidence paths are repository-relative, contain no `..`, and exist below the repository root.
6. `Present` requires ProxyBot grade `test-backed` or `release-proven` and an existing `test:` evidence item. `Partial` requires at least `source-backed` and at least one existing `source:` or `test:` item. `Missing`, private, and future rows may use `documented` plus an existing `docs:` item.
7. Every required inventory category appears at least once.

Collect all violations before printing them. Export `extractManifest` and `validateManifest` for reuse, but exercise the CLI in tests.

- [ ] **Step 4: Run focused tests and confirm GREEN**

Run: `rtk node --test scripts/check_parity_matrix.test.mjs`

Expected: all eight tests pass and the process exits `0`.

- [ ] **Step 5: Run a mutation check**

Temporarily remove the `Present`-claim branch from the checker with `apply_patch`, run the focused tests, and confirm the `rejects unsupported Present claims` case fails. Restore the branch with `apply_patch` and rerun to GREEN.

- [ ] **Step 6: Review and commit exact files**

Run `rtk git diff --check`, inspect `rtk git diff -- scripts/check_parity_matrix.mjs scripts/check_parity_matrix.test.mjs`, then stage and commit only those two paths:

```bash
rtk git add scripts/check_parity_matrix.mjs scripts/check_parity_matrix.test.mjs
rtk git commit -m "test: define parity matrix contract"
```

---

### Task 2: Fixed evidence ledger and complete capability matrix

**Files:**

- Create: `docs/parity/rockxy-community-evidence.md`
- Create: `docs/parity/rockxy-community-matrix.md`
- Modify: `package.json`

**Interfaces:**

- Consumes: the Task 1 manifest schema and checker CLI.
- Produces: `pnpm parity:check`, which runs `node --test scripts/check_parity_matrix.test.mjs && node scripts/check_parity_matrix.mjs`.
- Produces: 47 stable capability rows, `RXC-001` through `RXC-047`, including 44 public Community rows and three explicit boundary rows.

- [ ] **Step 1: Add the package command before the matrix exists**

Insert immediately after `test:workflow-config`:

```json
"parity:check": "node --test scripts/check_parity_matrix.test.mjs && node scripts/check_parity_matrix.mjs",
```

Run: `rtk pnpm parity:check`

Expected: RED because the default matrix document is missing.

- [ ] **Step 2: Write the evidence ledger**

Create `docs/parity/rockxy-community-evidence.md` with:

- the fixed repository, commit, capture date, AGPL-3.0-or-later public source boundary, official downstream DMG/private/Pro exclusion, and exact pinned links to `README.md`, `LICENSE`, and `LICENSING.md`;
- definitions for `documented`, `source-backed`, `test-backed`, `observable-build`, and `release-proven`, stating that a README claim alone is not parity scope evidence;
- a clean-room method: inspect behavior and public protocols, record evidence here, implement independently in ProxyBot's architecture, independently author fixtures, and never copy Rockxy source/tests/assets/copy/icons/images;
- evidence families with pinned source/test links for capture, filters/workspaces, Assistant/MCP, setup/certificates/system proxy, rules/Compose/Compare, sessions/export, scripting/plugins, protocol inspection, logs/nearby, update/security, and accessibility/performance;
- ProxyBot evidence syntax: `source:<path>`, `test:<path>`, `docs:<path>`; status claims describe current evidence, not roadmap intent.

- [ ] **Step 3: Write the machine-readable matrix**

Create `docs/parity/rockxy-community-matrix.md` with a short reader guide, status legend, and exactly one fenced JSON manifest. Use the following stable inventory and ownership; split no row and merge no row in this milestone:

| IDs | Capabilities | Category | Owner | Initial ProxyBot status |
| --- | --- | --- | --- | --- |
| RXC-001 | HTTP/HTTPS capture and decrypted persistence | capture | M2 | Present |
| RXC-002 | WebSocket capture and frame inspection | protocols | M10 | Partial |
| RXC-003 | advanced multi-field filtering and search | filtering | M3 | Partial |
| RXC-004..005 | Focus Sets; workspace Noise Control | focus-noise | M3 | Missing |
| RXC-006..007 | persisted workspace state; multi-tab workspaces | workspaces | M3, M6 | Partial, Missing |
| RXC-008 | evidence-grounded Assistant | assistant | M12 | Partial |
| RXC-009 | authenticated read-only MCP | mcp | M13 | Partial |
| RXC-010 | Developer Setup Hub and verification | setup | M8 | Partial |
| RXC-011..012 | root CA lifecycle; custom certificates | certificates | M8 | Partial, Missing |
| RXC-013..020 | selective TLS, bypass, block, Map Local, Map Remote, breakpoints, modify headers, network conditions | proxy-rules | RXC-013 M8; RXC-014..020 M7 | Partial |
| RXC-021..022 | Compose; Compare | compose-compare | M4, M6 | Partial, Missing |
| RXC-023..026 | session archive, HAR import/export, redacted copy formats, custom header columns | sessions-export | RXC-023/024/026 M6; RXC-025 M4 | Missing, Partial, Partial, Missing |
| RXC-027..028 | scripting; plugins/OpenAPI/Gist exporters | scripting | M11 | Partial |
| RXC-029..034 | GraphQL, gRPC/Protobuf, custom previewers, AI inspection, Web3 RPC, x402 | protocols | M10 | Partial, Partial, Partial, Partial, Missing, Missing |
| RXC-035 | logs and request timeline | logs | M14 | Missing |
| RXC-036 | error/performance insights | performance | M14 | Partial |
| RXC-037 | authenticated nearby transfer | nearby-transfer | M14 | Missing |
| RXC-038 | crash-safe system proxy automation | setup | M8 | Missing |
| RXC-039 | upstream HTTP/HTTPS/SOCKS5/PAC routing | proxy-rules | M9 | Missing |
| RXC-040 | software update behavior | updates | M16 | Partial |
| RXC-041 | capability and desktop security enforcement | security | M16 | Partial |
| RXC-042 | keyboard and accessibility workflows | accessibility | M15 | Partial |
| RXC-043 | high-volume workspace performance | performance | M15 | Partial |
| RXC-044 | independently verified release provenance | updates | M16 | Partial |
| RXC-045 | official downstream DMG/private/Pro behavior | security | M0 | Out-of-scope private |
| RXC-046 | Rockxy future redacted evidence bundles | sessions-export | M0 | Future-not-shipped |
| RXC-047 | Rockxy future team collaboration | workspaces | M0 | Future-not-shipped |

Use the pinned Rockxy source/test paths already recorded in the evidence ledger. Give every Community row `source-backed` or `test-backed` target evidence. Use local source/test paths found in the repository for `Present` and `Partial`; use `docs:docs/superpowers/plans/2026-08-19-rockxy-community-parity-milestones.md` for `Missing`, private, and future rows. Every acceptance statement must describe an observable independent ProxyBot outcome and must not repeat Rockxy wording.

Assign these exact pinned Rockxy paths to the corresponding rows; prefix each with `https://github.com/RockxyApp/Rockxy/blob/6a676d631820b577cf3a651c78d856733a7df995/`:

| Row | Rockxy path |
| --- | --- |
| RXC-001 | `RockxyTests/Core/ProxyEngine/TLSInterceptHandlerTests.swift` |
| RXC-002 | `Rockxy/Core/ProxyEngine/WebSocketFrameHandler.swift` |
| RXC-003 | `Rockxy/Models/UI/FilterRuleEvaluator.swift` |
| RXC-004 | `Rockxy/Views/Sidebar/FocusSetEditorSheet.swift` |
| RXC-005 | `Rockxy/Views/Sidebar/NoiseControlManagerSheet.swift` |
| RXC-006 | `Rockxy/Models/UI/WorkspaceState.swift` |
| RXC-007 | `Rockxy/Models/UI/WorkspaceStore.swift` |
| RXC-008 | `Rockxy/Core/Assistant/DebugAssistantEngine.swift` |
| RXC-009 | `Rockxy/Core/MCPServer/MCPToolRegistry.swift` |
| RXC-010 | `Rockxy/Models/UI/DeveloperSetupWorkflow.swift` |
| RXC-011 | `RockxyTests/Core/Certificate/CALifecycleTests.swift` |
| RXC-012 | `RockxyTests/Core/Certificate/CustomCertificateManagerTests.swift` |
| RXC-013 | `RockxyTests/Core/ProxyEngine/SSLProxyingManagerTests.swift` |
| RXC-014 | `RockxyTests/Core/ProxyEngine/BypassProxyManagerTests.swift` |
| RXC-015 | `RockxyTests/Core/RuleEngine/BlockListSettingsCodecTests.swift` |
| RXC-016 | `Rockxy/Core/Utilities/MapLocalSnapshotService.swift` |
| RXC-017 | `RockxyTests/Core/ProxyEngine/MapRemoteRewriteTests.swift` |
| RXC-018 | `Rockxy/Core/RuleEngine/BreakpointManager.swift` |
| RXC-019 | `Rockxy/Models/Rules/ModifyHeaderRuleBuilder.swift` |
| RXC-020 | `Rockxy/Models/Rules/NetworkConditionPreset.swift` |
| RXC-021 | `Rockxy/ViewModels/ComposeStore.swift` |
| RXC-022 | `Rockxy/Views/Diff/DiffEngine.swift` |
| RXC-023 | `Rockxy/Core/Plugins/BuiltInPlugins/SessionSerializer.swift` |
| RXC-024 | `Rockxy/Core/Plugins/BuiltInPlugins/HARImporter.swift` |
| RXC-025 | `Rockxy/Core/Plugins/BuiltInPlugins/HARExporter.swift` |
| RXC-026 | `Rockxy/Views/Settings/CustomHeaderColumnsView.swift` |
| RXC-027 | `Rockxy/Core/Plugins/ScriptRuntime.swift` |
| RXC-028 | `Rockxy/Core/Plugins/PluginManager.swift` |
| RXC-029 | `Rockxy/Views/Inspector/GraphQLInspectorView.swift` |
| RXC-030 | `Rockxy/Views/Inspector/GRPCInspectorView.swift` |
| RXC-031 | `Rockxy/Core/Utilities/PreviewRenderer.swift` |
| RXC-032 | `Rockxy/Core/Detection/AITrafficDetector.swift` |
| RXC-033 | `Rockxy/Core/Detection/Web3RPCDetector.swift` |
| RXC-034 | `Rockxy/Core/Detection/X402Detector.swift` |
| RXC-035 | `Rockxy/Core/LogEngine/LogCaptureEngine.swift` |
| RXC-036 | `Rockxy/Views/Timeline/RequestTimelineView.swift` |
| RXC-037 | `Rockxy/Core/Services/RockxyNearbyTransferProtocol.swift` |
| RXC-038 | `Rockxy/Core/TrafficCapture/SystemProxyManager.swift` |
| RXC-039 | `Rockxy/Core/UpstreamProxy/UpstreamProxyConnector.swift` |
| RXC-040 | `Rockxy/Core/Updates/SoftwareUpdateController.swift` |
| RXC-041 | `Shared/CallerValidation.swift` |
| RXC-042 | `Rockxy/Models/UI/KeyboardShortcutReference.swift` |
| RXC-043 | `RockxyTests/Models/UI/WorkspaceStoreCapacityTests.swift` |
| RXC-044 | `releases/validate_metadata.py` |
| RXC-045 | `LICENSING.md` |
| RXC-046..047 | `README.md` |

Before writing the matrix, verify every listed path is present in the fixed tree using `rtk gh api`; if a listed path differs only by an audited rename, use the exact path returned by that fixed tree and record the correction in the task report. This is evidence correction, not scope expansion.

For RXC-001 specifically, use ProxyBot grade `test-backed` and include both `source:src-tauri/src/acceptance.rs` and `test:scripts/test_desktop_acceptance.mjs`. All other initial claims must obey the Task 1 grade rules; when current local evidence is insufficient for `Partial`, classify the row `Missing` rather than inventing a source.

- [ ] **Step 4: Run the real matrix checker and correct only factual validation failures**

Run: `rtk pnpm parity:check`

Expected: Task 1 tests pass, the real 47-row manifest passes, and stdout names the fixed commit.

- [ ] **Step 5: Prove the real manifest gate can fail**

Temporarily change RXC-001's target URL commit to `main` with `apply_patch`, run `rtk pnpm parity:check`, and confirm non-zero exit with `floating Rockxy reference`. Restore the fixed commit using `apply_patch` and rerun to GREEN.

- [ ] **Step 6: Review and commit exact files**

Run `rtk git diff --check`, inspect the three-file diff, then stage and commit only:

```bash
rtk git add docs/parity/rockxy-community-evidence.md docs/parity/rockxy-community-matrix.md package.json
rtk git commit -m "docs: add Rockxy Community parity ledger"
```

---

### Task 3: Product governance and Milestone 0 exit gate

**Files:**

- Modify: `CONTRIBUTING.md`
- Modify: `docs/roadmap.md`
- Modify: `docs/superpowers/plans/2026-08-19-rockxy-community-parity-milestones.md`

**Interfaces:**

- Consumes: the passing 47-row parity matrix and evidence definitions from Task 2.
- Produces: contributor clean-room policy, roadmap linkage, and checked M0.1-M0.7 progress without changing runtime source.

- [ ] **Step 1: Add the clean-room contribution boundary**

Under `Change expectations` in `CONTRIBUTING.md`, add a `Clean-room compatibility work` subsection stating:

1. only public Community behavior at the pinned evidence snapshot is eligible;
2. no Rockxy source, tests, fixtures, copy, icons, images, private/Pro behavior, or official DMG reverse engineering may be copied into ProxyBot;
3. implementations and fixtures must be independently authored from public protocols, ProxyBot requirements, and observable behavior;
4. compatibility pull requests must update the parity row, cite pinned evidence, name the acceptance test, and preserve ProxyBot's MIT-compatible provenance.

- [ ] **Step 2: Link the roadmap to the parity authority**

Update `docs/roadmap.md` last-reviewed date to `2026-08-19`. Add a concise `Rockxy Community compatibility program` section after `Lessons from comparable projects` that:

- links the design, master milestone plan, evidence ledger, and machine-readable matrix;
- states the fixed commit and clean-room Community-only boundary;
- says the matrix records evidence/status/owner/acceptance while the roadmap remains the product-priority authority;
- states Phase A -> B -> C ordering and that feature-count parity cannot waive ProxyBot's first-success, redaction, security, or release gates.

- [ ] **Step 3: Mark only Milestone 0 complete in the master checklist**

In `docs/superpowers/plans/2026-08-19-rockxy-community-parity-milestones.md`:

- change M0.1 through M0.7 from `[ ]` to `[x]`;
- change Phase A scoreboard status from `Not started` to `In progress (M0 complete)`;
- leave all M1-M17 checkboxes and Phase B/C status unchanged.

- [ ] **Step 4: Run all Milestone 0 gates on the committed implementation state**

Run these exact commands and record fresh outputs in the task report:

```bash
rtk pnpm parity:check
rtk pnpm version:check
rtk pnpm contract:check
rtk git diff --check
rtk git status --short
```

Expected: every command exits `0`; `git status --short` lists only the three Task 3 documentation files before commit. Confirm with `rtk git diff --name-only HEAD~2` after commit that the entire milestone changed only `CONTRIBUTING.md`, `docs/**`, `package.json`, and `scripts/*.mjs`.

- [ ] **Step 5: Commit the governance and completion update**

Stage exactly:

```bash
rtk git add CONTRIBUTING.md docs/roadmap.md docs/superpowers/plans/2026-08-19-rockxy-community-parity-milestones.md
rtk git commit -m "docs: complete parity baseline milestone"
```

- [ ] **Step 6: Verify the clean final state**

Run:

```bash
rtk pnpm parity:check
rtk pnpm version:check
rtk pnpm contract:check
rtk git diff --check HEAD~3..HEAD
rtk git status --short --branch
```

Expected: all gates pass, no uncommitted files remain, and the branch contains exactly three implementation commits after this plan commit.
