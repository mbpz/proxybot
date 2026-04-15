# Review Request: Add Export JSON Button

## Summary

Added an "Export JSON" button next to the existing "Export HAR" button in the requests filter bar. The button exports the currently filtered requests as a plain JSON file.

## Changes

**File:** `/Users/jinguo.zeng/dmall/project/proxybot/src/App.tsx`

1. Added `exportJson` function:
   - Serializes `filterRequests(requests)` to JSON with 2-space indentation
   - Downloads as `proxybot-requests-YYYY-MM-DD.json`

2. Added "Export JSON" button next to "Export HAR" in the filter bar

## No Backend Changes

This is a pure frontend change. No Rust/Tauri changes required.

## Testing

- Click "Export JSON" and verify a `.json` file is downloaded
- Verify the JSON contains the correctly filtered requests
- Verify existing "Export HAR" button still works

## Risk

- Low — only adds a new export option, no modifications to existing functionality

---

# Review Request: Dark/Light Mode Toggle

## Summary

Added theme state and toggle button to switch between dark and light mode. CSS was refactored from `prefers-color-scheme` media queries to explicit `html.dark` and `html.light` class selectors.

## Changes

**Files:**
- `/Users/jinguo.zeng/dmall/project/proxybot/src/App.tsx`
- `/Users/jinguo.zeng/dmall/project/proxybot/src/App.css`

### App.tsx
1. Added `theme` state (`'dark' | 'light'`, default `'dark'`)
2. Added `toggleTheme` function
3. Added `useEffect` to sync theme to `document.documentElement.classList`
4. Added toggle button in Setup panel header showing `☀️ Light` / `🌙 Dark`

### App.css
1. Replaced `@media (prefers-color-scheme: dark)` with `html.dark { }` class selector
2. Added `html.light { }` class selector with light mode overrides

## Light Mode Color Palette

| Element | Light Value |
|---------|-------------|
| html bg | `#f5f5f7` |
| html text | `#1a1a1a` |
| cards/panels | `white` |
| headings | `#1d1d1f` |
| body text | `#515154` |
| accent | `#0071e3` |
| success | `#34c759` |
| error | `#ff3b30` |
| borders | `#e5e5e5` |
| muted | `#86868b` |

## Testing

- Click toggle button and verify theme switches immediately
- Verify all UI elements (cards, tables, buttons, tabs) render correctly in both modes
- Verify no flash of wrong theme on initial load
- Verify `prefers-color-scheme` is no longer the mechanism (test in browser devtools)

## Risk

- Low — purely additive UI change, existing dark mode behavior preserved via class toggle

---

# Review Request: Items 1-4 Implementation (Background Toggle, Settings Panel, Three Tabs, Request Actions)

## Summary

Implemented all 4 items from ARCHITECT-BRIEF.md:

### Item 1 — Background Process Toggle
- Added `keepRunning` state to App.tsx
- Added `toggleKeepRunning` function that calls `set_keep_running` Rust command
- Added `beforeunload` handler that calls `hide_window` when `keepRunning` is true
- Added `KeepRunningState` struct in Rust to persist preference
- Added `hide_window` and `set_keep_running`/`get_keep_running` commands

### Item 2 — Settings Panel
- Added settings button (gear icon) in header
- Added `showSettings` state to control panel visibility
- Settings panel slides in from right (reuses detail-panel-overlay CSS)
- Contains: 透明代理 section, CA证书 section, 后台运行 toggle, 清除历史 button

### Item 3 — Three Main Tabs
- Added `mainTab` state: `'http' | 'wss' | 'dns'`
- Top tabs show HTTP Requests/WSS Messages/DNS Queries with counts
- HTTP tab: controls, setup panel, CA guide, requests table (previously visible by default)
- WSS tab: WSS messages section only
- DNS tab: DNS queries section only

### Item 4 — Request Row Action Buttons
- Added Actions column to HTTP requests table
- Two buttons per row: Copy as cURL (📋) and Replay (⧉)
- `copyAsCurl` builds curl command from request headers/body and copies to clipboard
- `replayRequest` calls Rust `replay_request` command which re-issues the request

## Files Changed

### Rust (src-tauri/src/)
- `lib.rs`: Added KeepRunningState, registered new commands
- `proxy.rs`: Added KeepRunningState struct, hide_window, set_keep_running, get_keep_running, replay_request commands

### Frontend (src/)
- `App.tsx`: Added keepRunning, showSettings, mainTab state; settings button; top-tabs; action buttons; settings panel
- `App.css`: Added top-tabs, settings-btn, settings-section, toggle-switch, actions styles

## Verification

- `cargo check` passes (1 crate compiled)
- `npm run build` passes (34 modules transformed)

## Notes

- Replay uses `Box::leak` for static lifetime on ServerName (same pattern as existing HTTPS code)
- The `selectedTab` state was replaced by `appFilter` (already existed in filter bar)
- CA certificate download now uses `openPath` from @tauri-apps/plugin-opener directly

## Risks

- `Box::leak` in replay_request creates a memory leak per replay, but it's negligible for dev tooling
- Settings panel reuses detail-panel styles; may need adjustment for specific settings styling
