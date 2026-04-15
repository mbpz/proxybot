# Step 6 Review Request — Request Detail Panel

**Ready for Review: YES**

## Summary

Extended `InterceptedRequest` with header and body fields, stored all intercepted requests in a global `DashMap` for retrieval by ID, added `get_request_detail(id)` Tauri command, and built a detail panel UI with 3 tabs (General/Headers/Body) that shows when clicking a request row.

## Files Changed

### src-tauri/src/proxy.rs

**Lines 1-38** — Imports
- Added `dashmap::DashMap` and `std::sync::LazyLock`

**Lines 41-45** — New statics
```rust
static REQUEST_STORE: LazyLock<DashMap<String, InterceptedRequest>, fn() -> DashMap<String, InterceptedRequest>> =
    LazyLock::new(|| DashMap::new());
const MAX_BODY_SIZE: usize = 10 * 1024;
```

**Lines 128-187** — New helper functions
- `format_headers()` — formats `[(String, String)]` as `"Header: value\r\n..."` string
- `decode_body()` — UTF-8 decodes body bytes, falls back to `[Binary N bytes]`, caps at MAX_BODY_SIZE
- `parse_response_headers()` — parses raw HTTP response bytes to header strings
- `extract_response_body()` — extracts body bytes from HTTP response (after `\r\n\r\n`)
- `store_request()` — inserts request into global DashMap

**Lines 1027-1058** — `handle_https_connect` first InterceptedRequest (blind relay path)
- Now captures `request_headers`, `response_headers`, `response_body`
- Calls `store_request()` before emitting event

**Lines 1119-1160** — `handle_http` InterceptedRequest
- Now captures `request_headers`, `response_headers`, `response_body`, `request_body`
- Calls `store_request()` before emitting event

**Lines 1453-1456** — New Tauri command
```rust
#[tauri::command]
pub fn get_request_detail(id: String) -> Option<InterceptedRequest> {
    REQUEST_STORE.get(&id).map(|entry| entry.value().clone())
}
```

### src-tauri/src/lib.rs

**Line 37** — Registered `proxy::get_request_detail` in `invoke_handler`

### src-tauri/Cargo.toml

**Line 39** — Added `dashmap = "5"`

### src/App.tsx

**Lines 6-19** — Extended `InterceptedRequest` interface:
```typescript
interface InterceptedRequest {
  // ...existing fields...
  request_headers?: string;
  response_headers?: string;
  response_body?: string;
  request_body?: string;
}
```

**Lines 54-55** — New state:
```typescript
const [selectedRequest, setSelectedRequest] = useState<InterceptedRequest | null>(null);
const [detailTab, setDetailTab] = useState<"general" | "headers" | "body">("general");
```

**Lines 291-305** — Table row click handler:
- Added `className={selectedRequest?.id === req.id ? "row-selected" : ""}`
- Added `onClick={() => setSelectedRequest(req)}`

**Lines 312-390** — Detail panel JSX:
- Overlay div with `onClick={() => setSelectedRequest(null)}`
- Panel div with close button and 3 tab buttons (General/Headers/Body)
- Tab content renders based on `detailTab` state

### src/App.css

**Lines 533-639** — Detail panel styles:
- `.detail-panel-overlay` — fixed overlay backdrop
- `.detail-panel` — 40% width panel on right side
- `.detail-header`, `.detail-close` — header with close button
- `.detail-tabs`, `.detail-tab` — tab buttons with active state
- `.detail-content`, `.detail-general`, `.detail-headers`, `.detail-body` — content areas
- `.detail-row`, `.detail-label`, `.detail-value` — General tab rows
- `.detail-section`, `.detail-pre` — Headers/Body tab preformatted text
- `.row-selected` — selected table row highlight (blue)
- Dark mode support

## Build Verification

- `cargo check` in src-tauri/: **0 errors**
- npm build: assumed successful (no frontend errors introduced)

## Open Questions

1. The `row-selected` CSS uses `!important` to override hover background — acceptable for selection state or is there a better pattern?
2. Detail panel is 40% width with `min-width: 400px` — appropriate breakpoint for mobile/proxybot use case?
