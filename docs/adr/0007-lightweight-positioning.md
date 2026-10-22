# ADR-0007: Lightweight positioning and removal of Android SSL bypass

- **Status:** Accepted
- **Date:** 2026-10-22
- **Deciders:** Project owner (product decision) and maintainers
- **Archive:** `archive/pre-lightweight-2026-10-22`

## Context

ProxyBot's positioning is a **lightweight macOS MITM traffic debugger** for
mobile application developers: explicit proxy, inspect, modify, replay, export.
That promise is undermined by a second product inside the repository.

Android SSL bypass bundled a native Frida runtime, an Apktool-based APK patcher,
and a build-time download pipeline for `apktool.jar` and three Frida gadget
binaries. The feature never had a supported end-to-end user journey, required
network access and large third-party downloads to build, significantly enlarged
the bundle, and added native/elevated code paths to the security surface. It was
classified as Labs, but Labs still occupied navigation, CI, packaging, and
review attention.

Dead code and placeholder modules (an unreachable CDP server, an AI pipeline
placeholder, a Prometheus metrics server, plugin loader/WASM host stubs, and an
unused legacy command surface) were also reachable from the composition root or
CI without serving a product entry point.

## Decision

The project owner approved a direct removal, archived first at tag
`archive/pre-lightweight-2026-10-22`:

1. Remove Android SSL bypass end to end: the Frida runtime modules, the SSL
   bypass commands, the APK patcher, the unused ADB helper, the UI under
   `src/components/ssl-bypass/`, its store, routes, and e2e coverage.
2. Remove the bundled resource pipeline: `apktool.jar` and Frida gadget
   downloads, `scripts/fetch-bundle-resources.sh`, the resources directory, the
   bundle resource configuration, the `resources:fetch`/`resources:check`
   scripts, the `frida-runtime` Cargo feature and dependency, and the related CI
   and release-workflow steps.
3. Remove unreachable placeholder modules and development server stubs that have
   no product entry point; keep every capability that has a real user path.
4. Converge the default UI to three destinations: **Capture**, **Setup**, and
   **Settings**. Rules, Replay, Composer, DNS, Alerts, Graph, Topology, and the
   remaining Advanced/Labs tools move into the Capture context (context
   navigation, menus, command palette). Existing deep-link paths remain valid.
5. Archive historical records under `docs/history/` and exclude them from the
   documentation site; add ADRs, CHANGELOG, and SUPPORT so the decision and its
   consequences are recorded.
6. Do not reintroduce a removed capability without a new ADR, a supported user
   journey, a failure model, and a real-adapter acceptance test.

### Impact

| Area | At the archive tag | After this decision |
| --- | --- | --- |
| Android SSL bypass | Frida runtime + APK patcher + UI | Removed; use upstream Frida tooling outside ProxyBot |
| Build inputs | Network downloads of Apktool/Frida assets | No downloads; ordinary source build |
| Bundle | Extra jar + three gadget binaries + Frida Core | Smaller bundle, fewer native surfaces |
| Default navigation | Five equal-weight destinations | Three: Capture, Setup, Settings |
| Deep links | — | Preserved for all retained tools |
| Advanced | pf, DNS, MCP, scripting | Unchanged, still Advanced |
| Labs | SSL bypass, AI, generation/deployment | SSL bypass removed; remaining Labs stay opt-in |

### Rollback

The decision is reversible for archaeology, not for support: check out the
archive tag (`git checkout archive/pre-lightweight-2026-10-22 -- <path>` or diff
against the tag) to restore the removed code and resource pipeline, then revert
this ADR and the release/CI changes. Rollback is not recommended: the removed
feature never had a supported journey, and restoring it also restores
network-dependent builds and the native attack surface.

## Consequences

- The product claim and the shipped surface match: one lightweight MITM tool.
- Builds are reproducible without third-party downloads.
- Security review scope shrinks (no native Frida runtime, no APK patching).
- Some historical documents and the parity ledger reference removed paths; they
  are archived or updated separately.
- Users who relied on the Labs bypass must use dedicated Frida tooling; this is
  a deliberate scope reduction, recorded in the changelog.

## Evidence

- Archive tag: `archive/pre-lightweight-2026-10-22`
- `src-tauri/src/bootstrap.rs` (composition root; removed state and commands)
- `src-tauri/Cargo.toml` (removed `frida` dependency and `frida-runtime` feature)
- `src/main.tsx`, `src/components/layout/` (three Core destinations and preserved
  deep links)
- `CHANGELOG.md` (1.3.0 removals)
- `docs/history/` (archived records, not published)
- `docs/adr/0005-product-tiers.md` (Labs policy)
