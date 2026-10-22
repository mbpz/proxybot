# ADR-0004: Generated Desktop Contract as the IPC seam

- **Status:** Accepted (migration in progress)
- **Date:** 2026-10-22
- **Deciders:** ProxyBot maintainers

## Context

The React application talks to the Rust desktop shell through Tauri commands and
events. Hand-written TypeScript mirrors drift from Rust signatures, and
ad-hoc `invoke(...)` calls that convert failures into `null`, empty arrays, or
"no data" states make real failures indistinguishable from an empty capture.
Both problems have already caused user-visible "silent empty state" bugs.

## Decision

1. Desktop command and event metadata is generated from Rust. The generated
   TypeScript is committed so frontend builds do not require a Rust toolchain,
   and CI regenerates it in check mode to catch drift.
2. New or changed IPC goes through the generated Desktop Contract. Types for
   commands, results, and events come from the generated module.
3. Failures are typed product states. Transport and result-shape failures must
   remain visible and retryable; they must not be converted into `false`,
   empty, or `null` data.
4. UI tests use the `BrowserMockAdapter` implementing the same contract shape,
   so tests exercise the real command surface rather than ad-hoc mocks.
5. The migration target is that all production UI calls use the contract. Until
   that is complete, remaining direct `invoke`/`safeInvoke` call sites are
   tracked as known debt rather than imitated by new code.

## Consequences

- Rust signatures are the source of truth; drift is detected in CI.
- Error handling becomes consistent across screens.
- The generated file must not be edited by hand.
- A command rename or removal is a contract change: update Rust, regenerate,
  update the UI, and check for stale mocks in tests.

## Evidence

- `src-tauri/src/desktop_contract.rs` (contract declaration and generation)
- `src/generated/desktop-contract.ts` (generated TypeScript)
- `src/desktop/contract.ts`, `src/desktop/testing.ts` (runtime and mock adapters)
- `package.json` (`contract:generate`, `contract:check`)
- `.github/workflows/ci.yml` (generated contract check)
- `docs/architecture.md` ("React desktop Adapter")
