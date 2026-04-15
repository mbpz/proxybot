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
