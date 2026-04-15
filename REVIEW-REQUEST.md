# Step 9 Review Request — HAR Export

**Ready for Review: YES**

## Summary

Implemented HAR (HTTP Archive) 1.2 export functionality. Users can click "Export HAR" to download the currently filtered requests as a `.har` file, compatible with Chrome DevTools, Charles Proxy, and Fiddler.

## Files Changed

### src-tauri/src/har.rs (NEW)

HAR 1.2 serialization with these structs:
- `HarLog` → `HarLogInner` → `HarCreator` + `Vec<HarEntry>`
- `HarEntry`, `HarRequest`, `HarResponse`, `HarHeader`, `HarQueryParam`, `HarContent`, `HarTimings`

Key functions:
- `build_har(requests)` — top-level builder
- `intercepted_req_to_har_entry(req)` — converts one `InterceptedRequest` to `HarEntry`
- `parse_headers(headers_str)` — parses `"Name: value\r\n..."` into `Vec<HarHeader>`
- `to_iso8601(secs, ms)` — uses `libc::gmtime_r` + `libc::snprintf` to produce UTC ISO 8601 (no external time crate)
- `timestamp_to_iso8601(ts)` — parses proxy timestamp `"secs.ms"` and delegates to `to_iso8601`
- `http_status_text(status)` — maps HTTP status codes to human-readable strings
- `extract_content_type(headers_str)` — extracts Content-Type from response headers

Unit tests: `parse_headers`, `http_status_text`, `extract_content_type`

### src-tauri/src/proxy.rs

**InterceptedRequest struct** — added `serde::Deserialize` derive (needed because Tauri command parameters must be deserializable):
```rust
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct InterceptedRequest { ... }
```

**export_har command**:
```rust
#[tauri::command]
pub fn export_har(requests: Vec<InterceptedRequest>) -> Result<String, String> {
    let har_log = crate::har::build_har(requests);
    serde_json::to_string_pretty(&har_log).map_err(|e| e.to_string())
}
```

### src-tauri/src/lib.rs

- Added `mod har;`
- Added `proxy::export_har` to `invoke_handler`

### src/App.tsx

**exportHar function**:
```typescript
const exportHar = async () => {
  try {
    const filtered = filterRequests(requests);
    const har = await invoke<string>("export_har", { requests: filtered });
    const blob = new Blob([har], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `proxybot-${new Date().toISOString().slice(0, 10)}.har`;
    a.click();
    URL.revokeObjectURL(url);
  } catch (e) {
    setError(String(e));
  }
};
```

**UI** — "Export HAR" button at the start of the filter bar:
```jsx
<button className="btn-export" onClick={exportHar}>Export HAR</button>
```

### src/App.css

**Added styles:**
- `.btn-export` — blue button matching the app's design language, light and dark mode variants

## Key Design Decisions

1. **No external time crate** — ISO 8601 formatting uses `libc::gmtime_r` + `libc::snprintf`, keeping the dependency list minimal
2. **HAR version 1.2** — compatible with Chrome DevTools / Charles Proxy / Fiddler
3. **Filtered export** — exports only the currently displayed (filtered) requests, not all captured requests
4. **`Deserialize` on InterceptedRequest** — required because Tauri command parameters must implement `Deserialize<'de>`

## Acceptance Criteria

- [x] Click "Export HAR" → downloads `.har` file with name `proxybot-YYYY-MM-DD.har`
- [x] HAR file contains all currently filtered requests
- [x] Each entry has `startedDateTime`, `time`, `request`, `response`, `timings` fields
- [x] Request/response headers are parsed from `"Name: value\r\n..."` format
- [x] Response body is included in `content.text`
- [x] `cargo check` in `src-tauri/` passes with 0 errors, 0 warnings
