# Batch 6 Capability and Verified Release Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enforce Core/Advanced/Labs availability across every product surface and allow public release only after the uploaded asset is independently verified.

**Architecture:** A shared Rust-first capability catalog drives generated React policy and command authorization. A repository-owned BuildFlavor script becomes the only build entry used by local, CI, and Release. Release creates a Draft, publishes a machine-readable manifest and supply-chain artifacts, re-downloads the exact assets, verifies them, then waits for recorded iOS/Android evidence before publication.

**Tech Stack:** Rust, Tauri 2, React/TypeScript, Node scripts, pnpm/Corepack, GitHub Actions and `gh`, codesign, spctl, xcrun notarytool/stapler, SPDX SBOM, GitHub provenance

## Global Constraints

- Begin only after Batch 5's unified Investigation Workspace is green.
- Core is enabled by default; Advanced and Labs require explicit opt-in and visible prerequisites/risk.
- A route or raw desktop command cannot bypass a disabled capability.
- `package.json#packageManager` is the only pnpm version authority.
- Build preparation may fetch locked resources; verification and release build run from already-fetched inputs and must not download undeclared resources.
- Development Build, Release Candidate, and Verified Release are distinct states.
- A failed or missing uploaded-asset or physical-device gate leaves the GitHub Release in Draft.
- Every shell command starts with `rtk`; stage exact paths only.

---

### Task 1: Define one capability catalog and decision Interface

**Files:**
- Create: `src-tauri/src/capability/mod.rs`
- Create: `src-tauri/src/capability/catalog.rs`
- Create: `src-tauri/src/capability/state.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `CONTEXT.md`

**Interfaces:**
- Produces: `CapabilityLevel::{Core, Advanced, Labs}`
- Produces: `CapabilityId` enum covering every non-Core destination/command family
- Produces: `CapabilityDescriptor { id, level, label, description, prerequisites, risk }`
- Produces: `CapabilityState { enabled: BTreeSet<CapabilityId> }`
- Produces: `CapabilityGate::require(id) -> Result<(), CapabilityUnavailable>`

- [ ] **Step 1: Add default and prerequisite tests**

Assert every Core capability is enabled by default; every Advanced/Labs capability is disabled. Enabling `PfRedirect` without the macOS/admin prerequisite fails with a reason; enabling `SslBypass` does not enable AI or generation.

- [ ] **Step 2: Run and confirm catalog is absent**

Run: `rtk cargo test -p proxybot --lib capability`

Expected: FAIL because the Module does not exist.

- [ ] **Step 3: Implement exhaustive static descriptors**

Do not infer level from route names. Add descriptors for PF, DNS Server/Upstream, advanced TLS, MCP, Rhai/extensions, Mobile Dashboard, batch Replay, anomaly analysis, advanced Relationships, SSL bypass, AI, spec/mock/scaffold, and deploy. Persist only enabled IDs; descriptors remain code-owned and versioned.

- [ ] **Step 4: Update domain language and commit**

Define **Capability Gate**, **Core Capability**, **Advanced Capability**, and **Labs Capability** in `CONTEXT.md`. Then run:

```bash
rtk cargo test -p proxybot --lib capability
rtk git add CONTEXT.md src-tauri/src/capability src-tauri/src/lib.rs
rtk git commit -m "feat: define product capability gate"
```

### Task 2: Enforce capabilities in Desktop Contract and React routing

**Files:**
- Modify: `src-tauri/src/bootstrap.rs`
- Modify: `src-tauri/src/desktop_contract.rs`
- Modify: `src/desktop/contract.ts`
- Modify: `src/main.tsx`
- Modify: `src/components/layout/Sidebar.tsx`
- Create: `src/features/capabilities/CapabilitySettings.tsx`
- Create: `src/features/capabilities/CapabilityRoute.tsx`
- Create: `src/features/capabilities/CapabilityRoute.test.tsx`
- Modify: `src/components/settings/SettingsPage.tsx`
- Modify: `src/generated/desktop-contract.ts` through generation
- Modify: `src/components/layout/Navigation.test.tsx`

**Interfaces:**
- `get_capabilities({}) -> CapabilityDescriptor[]`
- `get_capability_state({}) -> CapabilityState`
- `set_capability_enabled({ id, enabled }) -> CapabilityState`
- `CapabilityRoute({ capability, children })`
- Disabled commands throw `DesktopError { kind: "unavailable", code: "capability_disabled", retryable: false }`

- [ ] **Step 1: Add route, direct URL, and command bypass tests**

Navigate directly to `/ssl-bypass` with Labs disabled and assert an unavailable explanation plus enable link; the feature Module must not mount. Call `frida_list_devices` directly and assert `capability_disabled`. Enable SSL bypass and assert only its route/commands become available.

- [ ] **Step 2: Run and confirm hidden navigation is bypassable**

```bash
rtk pnpm exec vitest run src/features/capabilities/CapabilityRoute.test.tsx src/components/layout/Navigation.test.tsx
rtk cargo test -p proxybot --lib capability
```

Expected: FAIL because routes/commands are currently unconditional.

- [ ] **Step 3: Add command-family authorization**

Wrap registered non-Core handlers with a small Adapter that calls `CapabilityGate::require` before the Implementation. Process modes also require capabilities: MCP stdio refuses startup unless enabled in Process Config. Do not scatter checks inside each command.

- [ ] **Step 4: Generate React policy and settings UI**

React loads descriptors/state through Contract. Settings groups Advanced and Labs, displays prerequisites/risk, and requires a confirmation for enablement. Sidebar and routes consume the same state. Deep links render the gate explanation rather than redirecting silently.

- [ ] **Step 5: Verify and commit enforcement**

```bash
rtk pnpm contract:generate
rtk cargo test -p proxybot --locked --no-default-features capability
rtk cargo test -p proxybot --locked --no-default-features desktop_contract
rtk pnpm exec vitest run src/features/capabilities/CapabilityRoute.test.tsx src/components/layout/Navigation.test.tsx src/test/SettingsPage.test.tsx
rtk pnpm typecheck
rtk pnpm test:e2e
rtk git add src-tauri/src/bootstrap.rs src-tauri/src/desktop_contract.rs src/desktop src/generated/desktop-contract.ts src/main.tsx src/components/layout src/components/settings src/features/capabilities src/test e2e
rtk git commit -m "feat: enforce capability availability"
```

### Task 3: Establish one BuildFlavor entry point

**Files:**
- Create: `scripts/build-flavor.mjs`
- Create: `scripts/build-flavor.test.mjs`
- Modify: `package.json`
- Modify: `.github/workflows/ci.yml`
- Modify: `.github/workflows/release.yml`
- Delete: `src-tauri/build.sh`

**Interfaces:**
- `node scripts/build-flavor.mjs core-ci`
- `node scripts/build-flavor.mjs desktop-smoke`
- `node scripts/build-flavor.mjs release --target aarch64-apple-darwin --bundles dmg`
- `release` refuses to run unless `resources:check` succeeds and signing/notary inputs are present

- [ ] **Step 1: Add dry-run command-sequence tests**

Invoke the script with `PROXYBOT_BUILD_DRY_RUN=1` and assert exact ordered commands. `core-ci` includes contract/bypass/version/Rust/UI/build checks. `desktop-smoke` uses production Tauri config and checked resources without signing. `release` adds `frida-runtime`, target, DMG, signing/notary preconditions, and no resource fetch.

- [ ] **Step 2: Run and confirm entry point is absent**

Run: `rtk node --test scripts/build-flavor.test.mjs`

Expected: FAIL because `build-flavor.mjs` is missing.

- [ ] **Step 3: Implement the command dispatcher without shell interpolation**

Use `spawnSync(binary, args, { stdio: "inherit" })` with fixed command arrays. Reject unknown flavors/arguments. Add package scripts `build:core-ci`, `build:desktop-smoke`, and `build:release`. CI and Release call only these scripts for build/test sequences.

- [ ] **Step 4: Remove the obsolete Yew builder and verify consumers**

Run `rtk rg -n "src-tauri/build.sh|wasm-pack build|build-flavor" .`. Expected: no supported caller references `src-tauri/build.sh`; delete it and update documentation.

- [ ] **Step 5: Verify and commit BuildFlavor**

```bash
rtk node --test scripts/build-flavor.test.mjs
rtk env PROXYBOT_BUILD_DRY_RUN=1 node scripts/build-flavor.mjs core-ci
rtk pnpm version:check
rtk git add scripts/build-flavor.mjs scripts/build-flavor.test.mjs package.json .github/workflows/ci.yml .github/workflows/release.yml src-tauri/build.sh README.md docs/releasing.md
rtk git commit -m "build: unify repository build flavors"
```

### Task 4: Lock every release resource, including Frida Core devkit

**Files:**
- Modify: `src-tauri/resources/resources.lock`
- Modify: `scripts/fetch-bundle-resources.sh`
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/build.rs`
- Create: `scripts/resource-lock.test.mjs`
- Modify: `.github/workflows/ci.yml`
- Modify: `.github/workflows/release.yml`

**Interfaces:**
- `resources.lock` records destination, source URL, uncompressed SHA-256, compression, version, and license/source metadata for every bundled or linked third-party artifact
- `resources:fetch` is the only network preparation step
- `resources:check` verifies an already prepared offline release input
- `FRIDA_DEVKIT_DIR` points to the locked prepared devkit root

- [ ] **Step 1: Add supply-chain completeness tests**

Parse `resources.lock` and assert unique safe destinations, HTTPS sources, 64-hex digests, explicit version, and license/source reference. Assert `src-tauri/Cargo.toml` does not enable Frida `auto-download` and `release.yml` never runs `resources:fetch` in the build step.

- [ ] **Step 2: Run and confirm native Frida violates the lock**

Run: `rtk node --test scripts/resource-lock.test.mjs`

Expected: FAIL because native Frida uses `auto-download` and lacks a lock entry.

- [ ] **Step 3: Add architecture-specific Frida Core devkit archives**

Use the exact `16.5.2` version declared by `frida-sys 0.14.2`. Add `frida-core-devkit-16.5.2-macos-arm64.tar.xz` and `frida-core-devkit-16.5.2-macos-x86_64.tar.xz`; GitHub release metadata identifies them as 43,352,160 and 33,555,004 bytes respectively. During the explicit prepare step, download each asset once, compute its archive and extracted `frida-core.h`/`libfrida-core.a` SHA-256 values with `shasum -a 256`, record those exact values in `resources.lock`, then rerun `resources:fetch` from an empty cache to prove them. Extend the fetch script to safely extract `tar.xz` into versioned destinations and reject a declared size/hash mismatch before installation.

- [ ] **Step 4: Disable dependency-time downloads**

Remove `features = ["auto-download"]` from the optional `frida` dependency. `build.rs` requires `FRIDA_DEVKIT_DIR` when `CARGO_FEATURE_FRIDA_RUNTIME` is set, validates expected files, and emits link/include metadata. `desktop-smoke` and `release` export the locked target-specific path.

- [ ] **Step 5: Prove offline release-flavor linking and commit**

```bash
rtk pnpm resources:fetch
rtk pnpm resources:check
rtk node --test scripts/resource-lock.test.mjs
rtk cargo check -p proxybot --lib --tests --locked --no-default-features --features frida-runtime
rtk git add src-tauri/resources/resources.lock scripts/fetch-bundle-resources.sh scripts/resource-lock.test.mjs src-tauri/Cargo.toml src-tauri/build.rs .github/workflows/ci.yml .github/workflows/release.yml
rtk git commit -m "build: lock native release resources"
```

### Task 5: Generate a machine-readable release manifest

**Files:**
- Create: `scripts/release-manifest.mjs`
- Create: `scripts/release-manifest.test.mjs`
- Modify: `scripts/version.mjs`
- Modify: `.github/workflows/release.yml`

**Interfaces:**
- `node scripts/release-manifest.mjs create --asset ProxyBot-1.3.0-mac-arm64.dmg --sbom ProxyBot-1.3.0-mac-arm64.spdx.json --target aarch64-apple-darwin --output ProxyBot-1.3.0-mac-arm64.manifest.json`
- `node scripts/release-manifest.mjs verify --manifest ProxyBot-1.3.0-mac-arm64.manifest.json --asset ProxyBot-1.3.0-mac-arm64.dmg`
- Produces schema `proxybot.release-manifest.v1`

- [ ] **Step 1: Add manifest completeness and tamper tests**

Create a temporary asset and assert the manifest includes git SHA, Product Version, target, Rust/Node/pnpm/Tauri versions, BuildFlavor, features, DB schema, resources.lock digest, Frida/apktool versions, length, SHA-256, SBOM/provenance names, signing Team, notary request ID, staple result, CI run URL, updater channel, and minimum compatible version. Change one byte and assert verify fails.

- [ ] **Step 2: Run and confirm generator is absent**

Run: `rtk node --test scripts/release-manifest.test.mjs`

Expected: FAIL because the script does not exist.

- [ ] **Step 3: Implement deterministic create/verify modes**

Read repository/tool values directly; require signing/notary/CI values from explicit environment variables in release mode. Sort object keys before writing. `verify` recomputes size, hashes, resource digest, Product Version, and target consistency.

- [ ] **Step 4: Add manifest to release assets and provenance**

Generate one manifest per architecture after notarization/stapling; attest and upload it beside DMG, checksum, and SBOM. Extend `version:check` to require these release workflow steps.

- [ ] **Step 5: Verify and commit manifest generation**

```bash
rtk node --test scripts/release-manifest.test.mjs
rtk pnpm version:check
rtk git add scripts/release-manifest.mjs scripts/release-manifest.test.mjs scripts/version.mjs .github/workflows/release.yml
rtk git commit -m "build: emit verifiable release manifest"
```

### Task 6: Publish to Draft and re-download exact assets

**Files:**
- Create: `scripts/verify-release-asset.sh`
- Create: `scripts/verify-release-asset.test.mjs`
- Modify: `.github/workflows/release.yml`
- Modify: `docs/releasing.md`

**Interfaces:**
- `scripts/verify-release-asset.sh ProxyBot-1.3.0-mac-arm64.dmg ProxyBot-1.3.0-mac-arm64.manifest.json "$RUNNER_TEMP/proxybot-install"`
- Produces verification report `proxybot.asset-verification.v1.json`
- GitHub Release remains Draft until the final publish job

- [ ] **Step 1: Add script argument and tamper tests**

Node tests invoke the shell script against missing files and a tampered fixture, asserting nonzero exit and no success report. Static workflow assertions require `draft: true`, a separate download job using `gh release download`, and no direct public publish from build artifacts.

- [ ] **Step 2: Run and confirm current workflow publishes immediately**

Run: `rtk node --test scripts/verify-release-asset.test.mjs`

Expected: FAIL because the workflow publishes a non-draft release directly from Actions artifacts.

- [ ] **Step 3: Create Draft before independent verification**

After both builds complete, create/update a Draft Release and upload DMGs, checksums, SBOMs, provenance references, and manifests. A new macOS job checks out the tag, downloads the exact GitHub Release assets with `gh release download`, and never uses the build workspace copies.

- [ ] **Step 4: Verify identity, installation, and upgrade**

The script verifies manifest/checksum, `codesign --verify --deep --strict`, `spctl`, `stapler validate`, mount, bundle identity/version, copy to an isolated install root, first launch acceptance, DB migration, and updater from the previous Verified Release when one exists. Write command results and hashes into the verification report and upload it back to the Draft.

- [ ] **Step 5: Verify workflow structure and commit**

```bash
rtk node --test scripts/verify-release-asset.test.mjs
rtk pnpm version:check
rtk git add scripts/verify-release-asset.sh scripts/verify-release-asset.test.mjs .github/workflows/release.yml docs/releasing.md
rtk git commit -m "release: verify uploaded draft assets"
```

### Task 7: Add physical-device evidence and final publication gate

**Files:**
- Create: `docs/release-evidence/device-capture.schema.json`
- Create: `scripts/verify-device-evidence.mjs`
- Create: `scripts/verify-device-evidence.test.mjs`
- Modify: `.github/workflows/release.yml`
- Modify: `docs/releasing.md`
- Modify: `README.md`

**Interfaces:**
- `node scripts/verify-device-evidence.mjs --tag v1.3.0 --asset-sha "$ASSET_SHA256" --ios ios-device-evidence.json --android android-device-evidence.json`
- Evidence fields: platform, operator, device model, OS version, git SHA, artifact SHA-256, certificate setup, proxy setup, HTTPS Request/Response inspected, Session stopped, timestamp
- Final job runs `gh release edit "$RELEASE_TAG" --draft=false` only after asset and both device evidence reports validate

- [ ] **Step 1: Add evidence schema tests**

Fixtures include valid iOS/Android evidence, wrong artifact SHA, wrong git SHA, missing Request/Response check, and stale timestamp. Assert only the matching pair passes.

- [ ] **Step 2: Run and confirm no publication gate exists**

Run: `rtk node --test scripts/verify-device-evidence.test.mjs`

Expected: FAIL because schema/verifier are absent.

- [ ] **Step 3: Implement verifier and manual GitHub environment gate**

Use a protected `physical-device-release` GitHub environment. Operators upload signed-off JSON evidence as workflow inputs/artifacts. The verifier validates schema, tag commit, asset SHA, supported platform, and all boolean journey checks before the final job makes the Release public.

- [ ] **Step 4: Align product claims with release state**

README continues to label existing artifacts previews until the first Verified Release. The workflow release notes link manifest, SBOM, provenance, asset verification, and device evidence. A failed/missing gate states that the candidate remains Draft; it never rewrites failure as success.

- [ ] **Step 5: Run all repository configuration gates and commit**

```bash
rtk node --test scripts/verify-device-evidence.test.mjs
rtk pnpm test:workflow-config
rtk pnpm version:check
rtk git add docs/release-evidence/device-capture.schema.json scripts/verify-device-evidence.mjs scripts/verify-device-evidence.test.mjs .github/workflows/release.yml docs/releasing.md README.md
rtk git commit -m "release: require physical device evidence"
```

### Task 8: Run the final convergence gate

**Files:**
- No file changes are expected; a failure stops the final gate and returns work to the exact owning task above

**Interfaces:**
- Consumes: all Core Modules, CapabilityGate, BuildFlavor, and release verification scripts
- Produces: a candidate tag eligible to enter the Draft Release pipeline

- [ ] **Step 1: Run the complete local Core gate**

```bash
rtk pnpm build:core-ci
rtk pnpm test:e2e
rtk pnpm build:desktop-smoke
rtk pnpm test:desktop:acceptance
```

Expected: all exit 0.

- [ ] **Step 2: Run release input verification without credentials**

```bash
rtk pnpm resources:check
rtk node --test scripts/release-manifest.test.mjs scripts/verify-release-asset.test.mjs scripts/verify-device-evidence.test.mjs
rtk env PROXYBOT_BUILD_DRY_RUN=1 node scripts/build-flavor.mjs release --target aarch64-apple-darwin --bundles dmg
```

Expected: all tests/dry-run exit 0 and reveal no implicit network fetch.

- [ ] **Step 3: Review product-surface and Adapter invariants**

```bash
rtk pnpm contract:adapter-check
rtk rg -n "@tauri-apps/api/(core|event)|safeInvoke" src --glob '!src/desktop/contract.ts'
rtk rg -n "Unknown Activity|No findings" src src-tauri
rtk git diff --check
```

Expected: Adapter search has no matches; inference search is absent or appears only in negative tests/explicit completed-empty projection copy.

- [ ] **Step 4: Record the green handoff without changing files**

Run `rtk git status --short` and `rtk git log -8 --oneline`. Expected: no uncommitted Batch 6 files and the eight task commits are visible. If any prior gate failed, stop here and return to its owning task; do not patch opportunistically in this final gate and do not create an empty commit. Push the green commit series and use the hosted Draft Release pipeline for signing, notarization, uploaded-asset verification, and physical-device gates.
