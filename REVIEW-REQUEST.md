# Step 11 Review: History Persistence

**Ready for Review: YES**

## Summary

Added persistent history storage so intercepted requests survive app restarts. Uses `serde_json` to read/write `~/.proxybot/history.json`.

## Files Changed

### `src-tauri/src/history.rs` (NEW)

```rust
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::proxy::InterceptedRequest;

const HISTORY_FILE: &str = "history.json";
const MAX_STORED: usize = 1000;

pub struct HistoryStore {
    path: PathBuf,
}

impl HistoryStore {
    pub fn new() -> Self { /* creates ~/.proxybot directory */ }

    pub fn load(&self) -> Vec<InterceptedRequest> {
        // Opens file, parses JSON, returns Vec, ignores errors gracefully
        // No external time crate: SystemTime::now().duration_since(UNIX_EPOCH).as_secs()
    }

    pub fn save(&self, requests: &[InterceptedRequest]) -> Result<(), String> {
        // Writes { version, last_updated: "<unix_timestamp>", requests }
        // Limits to MAX_STORED (1000) entries
    }
}
```

### `src-tauri/src/lib.rs`

- Added `mod history;`
- Added `proxy::load_history`, `proxy::save_history` to invoke handler

### `src-tauri/src/proxy.rs`

```rust
#[tauri::command]
pub fn load_history() -> Vec<InterceptedRequest> {
    crate::history::HistoryStore::new().load()
}

#[tauri::command]
pub fn save_history(requests: Vec<InterceptedRequest>) -> Result<(), String> {
    crate::history::HistoryStore::new().save(&requests)
}
```

### `src/App.tsx`

```typescript
// useEffect on mount:
invoke<InterceptedRequest[]>("load_history")
  .then((hist) => { if (hist.length > 0) setRequests(hist); })
  .catch((e) => console.error("Failed to load history:", e));

// New handlers:
const saveHistory = async () => {
  try { await invoke("save_history", { requests }); }
  catch (e) { setError(String(e)); }
};

const clearHistory = async () => {
  if (!confirm("确定清空所有历史记录？")) return;
  try {
    await invoke("save_history", { requests: [] });
    setRequests([]);
  } catch (e) { setError(String(e)); }
};
```

Filter bar buttons (next to Export HAR):
```tsx
<button className="btn-save" onClick={saveHistory}>💾 保存</button>
<button className="btn-clear-history" onClick={clearHistory}>🗑️ 清空</button>
```

### `src/App.css`

- `.btn-save`: white bg, green hover tint
- `.btn-clear-history`: white bg, red hover tint for destructive action

## Key Design Decisions

1. **No external time crate** — used `SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()` formatted as Unix timestamp string
2. **Graceful degradation** — `load()` returns empty `Vec` on any parse/file error
3. **Max 1000 entries** — `save()` truncates to `MAX_STORED` before writing

## Acceptance Criteria

- [ ] Restart App -> history requests auto-restore
- [ ] Click "Save" -> manual save triggered
- [ ] Click "Clear" -> confirm dialog -> all records cleared, file emptied

## Verification

```
cd src-tauri && cargo check
cargo build (1 crates compiled)
Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.15s
0 errors, 0 warnings
```
