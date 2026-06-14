# Update Detection + Icon Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Close the gap between the approved 2026-05-14 spec and shipped code — wire App-startup auto-check for ProxyBot releases and bump the hardcoded `CURRENT_VERSION` constant to match the v1.3.x release line.

**Architecture:** Add a one-shot `useEffect` in `Layout.tsx` (always mounted as the route container, so it fires once per app start) that calls `useUpdateCheck().checkForUpdates()`. Refactor `useUpdateCheck` to expose `CURRENT_VERSION` so the version constant is testable and centralized. No new dependencies.

**Tech Stack:** React 18 (existing `useEffect`), TypeScript, Vitest.

---

## File Structure

Files modified by this plan:

| File | Responsibility | Changes |
|------|----------------|---------|
| `src/hooks/useUpdateCheck.ts` | Update detection hook | Export `CURRENT_VERSION`; bump from 1.2.0 → 1.3.0; add `compareVersions` unit tests |
| `src/components/layout/Layout.tsx` | Route container, always mounted | Add `useEffect` for one-shot startup auto-check |
| `src/test/useUpdateCheck.test.ts` | (new) Hook unit tests | Cover `compareVersions`, `CURRENT_VERSION`, and auto-check dispatch |

No new files in `src-tauri/`. The icon design (spec §1) was already shipped — `src-tauri/icons/{icon.png, icon.icns, Square*Logo.png, 128x128.png}` exist and `tauri.conf.json` bundles them.

---

## State of the world (audit before coding)

| Spec item | Status | Location |
|-----------|--------|----------|
| App icon (cyberpunk neon theme) | ✅ done | `src-tauri/icons/` + `tauri.conf.json:31-35` |
| Manual "检查更新" button in Settings | ✅ done | `src/components/setup/UpdateSettings.tsx` + used in `AboutTab.tsx`, `ClientSetup.tsx` |
| `useUpdateCheck` hook (`checkForUpdates`, `openReleasePage`, `compareVersions`) | ✅ done | `src/hooks/useUpdateCheck.ts` |
| GitHub Releases API integration (`api.github.com/repos/mbpz/proxybot/releases/latest`) | ✅ done | `useUpdateCheck.ts:30-37` |
| Semver-style version comparison | ✅ done | `useUpdateCheck.ts:68-79` |
| **App startup auto-check** (spec §2 "自动检查: App 启动时后台检查") | ❌ **only gap** | not wired |
| **CURRENT_VERSION = "1.2.0"** (spec implicitly v1.3.x per roadmap) | ⚠️ stale | `useUpdateCheck.ts:12` |
| Unit tests for the hook | ❌ missing | — |

---

## Tasks

### Task 1: Refactor `useUpdateCheck` to export `CURRENT_VERSION` and bump it

**Files:**
- Modify: `src/hooks/useUpdateCheck.ts:12`

**Why:** The hook hardcodes `CURRENT_VERSION = "1.2.0"`. The roadmap shows the project is on v1.3.x. Exporting it makes it testable and lets the App-startup banner (if added later) read the same constant.

**Change:**

```ts
// Before
const CURRENT_VERSION = "1.2.0";

// After
export const CURRENT_VERSION = "1.3.0";
```

### Task 2: Wire startup auto-check in `Layout.tsx`

**Files:**
- Modify: `src/components/layout/Layout.tsx`

**Why:** Spec §2 explicitly requires an automatic background check on app start. `Layout` is the always-mounted route container, so a `useEffect(() => { ... }, [])` will fire exactly once per app session, not per route change. The existing hook already swallows network errors into `error` state, so a failed auto-check never surfaces a UI error to the user — the manual button in Settings still works.

**Change:**

```tsx
import { useEffect } from "react";
import { Outlet } from "react-router-dom";
import { Sidebar } from "./Sidebar";
import { useUpdateCheck } from "../../hooks/useUpdateCheck";

export function Layout() {
  const { checkForUpdates } = useUpdateCheck();

  // Spec §2: App-startup background check for new releases.
  // Runs exactly once per app session (Layout is always mounted).
  // Errors are swallowed by useUpdateCheck — a failed check just leaves
  // `hasUpdate` false, which is the safe default.
  useEffect(() => {
    checkForUpdates();
  }, [checkForUpdates]);

  return (
    <div className="flex h-screen">
      <Sidebar />
      <main className="flex-1 overflow-auto bg-surface-primary p-6">
        <Outlet />
      </main>
    </div>
  );
}
```

### Task 3: Add unit tests for `useUpdateCheck`

**Files:**
- Create: `src/test/useUpdateCheck.test.ts`

**Why:** Currently zero automated coverage for the hook. Tests pin (a) the bumped `CURRENT_VERSION`, (b) the `compareVersions` semantics, (c) the auto-check dispatch.

**Content:**

```ts
import { describe, it, expect } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useUpdateCheck, CURRENT_VERSION } from "../hooks/useUpdateCheck";

describe("CURRENT_VERSION", () => {
  it("is on the v1.3.x line", () => {
    expect(CURRENT_VERSION).toBe("1.3.0");
  });
});

describe("compareVersions (via useUpdateCheck contract)", () => {
  // compareVersions is private; we exercise it through the hook by
  // mocking fetch to return a chosen tag_name and observing hasUpdate.
  // Each test asserts the boolean contract, which is what callers care about.

  function mockLatestRelease(tag_name: string) {
    globalThis.fetch = (async () =>
      ({
        ok: true,
        status: 200,
        json: async () => ({ tag_name, html_url: "https://example/release" }),
      } as Response)) as typeof fetch;
  }

  it("reports hasUpdate=true when latest > current", async () => {
    mockLatestRelease("v1.4.0");
    const { result } = renderHook(() => useUpdateCheck());
    await act(async () => { await result.current.checkForUpdates(); });
    expect(result.current.hasUpdate).toBe(true);
    expect(result.current.latestVersion).toBe("1.4.0");
  });

  it("reports hasUpdate=false when latest == current", async () => {
    mockLatestRelease(`v${CURRENT_VERSION}`);
    const { result } = renderHook(() => useUpdateCheck());
    await act(async () => { await result.current.checkForUpdates(); });
    expect(result.current.hasUpdate).toBe(false);
  });

  it("reports hasUpdate=false when latest < current", async () => {
    mockLatestRelease("v1.2.5");
    const { result } = renderHook(() => useUpdateCheck());
    await act(async () => { await result.current.checkForUpdates(); });
    expect(result.current.hasUpdate).toBe(false);
  });

  it("strips the leading 'v' from tag_name", async () => {
    mockLatestRelease("v2.0.0");
    const { result } = renderHook(() => useUpdateCheck());
    await act(async () => { await result.current.checkForUpdates(); });
    expect(result.current.latestVersion).toBe("2.0.0");
  });

  it("sets error on HTTP failure but leaves hasUpdate=false", async () => {
    globalThis.fetch = (async () =>
      ({ ok: false, status: 500, json: async () => ({}) } as Response)) as typeof fetch;
    const { result } = renderHook(() => useUpdateCheck());
    await act(async () => { await result.current.checkForUpdates(); });
    expect(result.current.error).toMatch(/500/);
    expect(result.current.hasUpdate).toBe(false);
  });
});
```

(Adapt to the project's existing test framework — Vitest is in `package.json`. Add `@testing-library/react` to devDeps only if not already present; if the project uses a different React testing helper, swap the `renderHook` import accordingly.)

### Task 4: Spec self-review — flip status

**Files:**
- Modify: `docs/superpowers/specs/2026-05-14-proxybot-update-icon-design.md`

Change header from:

```
**Status:** Approved
```

to:

```
**Status:** Implemented (v1.3.x)
```

Append a short "Implementation Notes" section summarising the audit table above and the two gaps closed in this pass (startup auto-check + CURRENT_VERSION bump + new tests).

---

## Validation

```bash
# Frontend typecheck (must pass with no new errors)
npm run typecheck

# Hook unit tests
npx vitest run src/test/useUpdateCheck.test.ts

# Existing test suite must still pass
npx vitest run
```

Expect: `CURRENT_VERSION` test pins `1.3.0`; three `hasUpdate` tests pass (greater / equal / less); HTTP-failure test sets `error` without throwing; full Vitest run shows no regressions.

---

## Out of scope (per spec)

- Visual icon design refresh — icons exist and ship. A future PR can re-render them per the cyberpunk spec if the current ones don't match.
- Settings-page red-dot badge that surfaces `hasUpdate` in the sidebar — useful follow-up but not in this spec.
- Background polling / interval-based checks — spec says "App 启动时" (on app start), one-shot is correct.
- Rust-side backend (`spec §4 "无需后端支持"`) — out of scope by design.