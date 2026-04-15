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
