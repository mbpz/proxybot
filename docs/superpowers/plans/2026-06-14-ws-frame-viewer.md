# WS Frame Viewer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a WebSocket frame viewer to the RequestDetail panel: Tauri command for historical frames, real-time Tauri event stream, and 3 React components (frame list, frame detail, hex dump).

**Architecture:** Builds on existing infrastructure — `WsFrame` struct in `proxy/mod.rs`, `ws_frames` DB table in `db.rs`, `record_ws_frame` call in `proxy/forward.rs`. New work: add `opcode` + `truncated` fields to `WsFrame`, add `get_ws_frames` Tauri command, emit `ws-frame:new` event after `record_ws_frame`, build 3 React components (WsFramesView / FrameDetail / HexDump) and wire into RequestDetail.

**Tech Stack:** Rust (Tauri 2), rusqlite, React 18 + TypeScript + existing shadcn/ui-style custom classes, Playwright for E2E.

**Working directory:** This plan assumes the implementer is at the repo root on a feature branch off `main`. All file paths are relative to the repo root.

---

## File Structure

| File | Responsibility | Status |
|---|---|---|
| `src-tauri/src/proxy/mod.rs` | Add `opcode` + `truncated` to `WsFrame`; add `get_opcode_name` | **Modify** |
| `src-tauri/src/db.rs` | Add `get_ws_frames` query | **Modify** |
| `src-tauri/src/proxy/commands.rs` | Add `get_ws_frames` Tauri command | **Modify** |
| `src-tauri/src/proxy/forward.rs` | Emit `ws-frame:new` after `record_ws_frame` | **Modify** |
| `src-tauri/src/ws_frames/mod.rs` | `MAX_PAYLOAD_SIZE` / `PREVIEW_SIZE` constants + truncation helper | **New** |
| `src-tauri/src/lib.rs` | Register `get_ws_frames` command | **Modify** |
| `src/components/ws-frames/types.ts` | TS WsFrame + `getOpcodeName` | **New** |
| `src/components/ws-frames/HexDump.tsx` | Hex dump view | **New** |
| `src/components/ws-frames/FrameDetail.tsx` | Metadata + text/hex toggle | **New** |
| `src/components/ws-frames/WsFramesView.tsx` | List + detail split + real-time subscribe | **New** |
| `src/components/.../RequestDetail.tsx` | New "WebSocket Frames" tab | **Modify** |
| `e2e/ws-frames.spec.ts` | Playwright tests | **New** |

No new dependencies. No DB schema change (the `ws_frames` table already has all needed columns).

---

## Task 1: Add WsFrame fields + opcode helper

**Files:**
- Modify: `src-tauri/src/proxy/mod.rs` (extend `WsFrame` struct, add `get_opcode_name`)
- Modify: `src-tauri/src/db.rs` (the existing `record_ws_frame` is called from forward.rs — make sure callers pass opcode)

- [ ] **Step 1: Write the failing test**

Open `src-tauri/src/proxy/mod.rs` and find the existing `WsFrame` struct. Add a test module at the bottom of the file (if not present) with:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_get_opcode_name() {
        assert_eq!(get_opcode_name(0x01), "Text");
        assert_eq!(get_opcode_name(0x02), "Binary");
        assert_eq!(get_opcode_name(0x08), "Close");
        assert_eq!(get_opcode_name(0x09), "Ping");
        assert_eq!(get_opcode_name(0x0A), "Pong");
        assert_eq!(get_opcode_name(0x00), "Unknown");
    }
}
```

- [ ] **Step 2: Run the test — it should fail to compile (function doesn't exist)**

```bash
cargo test -p proxybot --lib proxy::tests::test_get_opcode_name
```

Expected: compile error or "cannot find function".

- [ ] **Step 3: Extend the WsFrame struct and add the helper**

In `src-tauri/src/proxy/mod.rs`, change the `WsFrame` struct from:

```rust
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct WsFrame {
    pub direction: String,
    pub timestamp: String,
    pub payload: String,
    pub size: usize,
}
```

to:

```rust
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct WsFrame {
    pub direction: String,
    pub timestamp: String,
    pub payload: String,
    pub size: usize,
    pub opcode: u8,
    pub truncated: bool,
}
```

And add this function (in the same file, above or below the struct):

```rust
/// Map a WebSocket frame opcode to a human-readable name.
pub fn get_opcode_name(opcode: u8) -> &'static str {
    match opcode {
        0x01 => "Text",
        0x02 => "Binary",
        0x08 => "Close",
        0x09 => "Ping",
        0x0A => "Pong",
        _ => "Unknown",
    }
}
```

- [ ] **Step 4: Run the test — it should pass**

```bash
cargo test -p proxybot --lib proxy::tests::test_get_opcode_name
```

Expected: 1 passed.

- [ ] **Step 5: Find all the call sites that construct WsFrame**

```bash
grep -rn "WsFrame {" /Users/doug/ai/system/proxybot/src-tauri/src/ | head -10
```

These are the places that need to be updated to pass the new fields. Update each one to include `opcode: 0` and `truncated: false` (defaults — will be set properly when reading from DB or emitting events).

- [ ] **Step 6: Compile-check and run all lib tests**

```bash
cargo check -p proxybot --lib
cargo test -p proxybot --lib
```

Expected: 0 errors. Existing tests may need updates for the new fields (if they construct WsFrame directly).

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/proxy/mod.rs
git commit -m "feat(proxy): add opcode and truncated fields to WsFrame

Extends the WsFrame struct with opcode (Text/Binary/Close/Ping/
Pong) and truncated (for payload > 64KB) fields. Adds
get_opcode_name helper. Existing call sites get opcode=0 and
truncated=false defaults — DB and event paths fill them in
correctly in subsequent tasks."
```

---

## Task 2: Add truncation helper

**Files:**
- Create: `src-tauri/src/ws_frames/mod.rs` (constants + truncation logic)

- [ ] **Step 1: Create the file with tests**

Create `src-tauri/src/ws_frames/mod.rs`:

```rust
//! WebSocket frame payload truncation.

/// Payloads above this size are truncated to PREVIEW_SIZE bytes.
pub const MAX_PAYLOAD_SIZE: usize = 64 * 1024;

/// When truncating, keep this many bytes of preview.
pub const PREVIEW_SIZE: usize = 1024;

/// Truncate a payload to fit within MAX_PAYLOAD_SIZE. Returns
/// (preview_string, was_truncated). Binary payloads are passed
/// through String::from_utf8_lossy; the hex view in the frontend
/// can use base64 if lossless rendering is needed.
pub fn truncate_payload(payload: &[u8]) -> (String, bool) {
    if payload.len() <= MAX_PAYLOAD_SIZE {
        (String::from_utf8_lossy(payload).to_string(), false)
    } else {
        (String::from_utf8_lossy(&payload[..PREVIEW_SIZE]).to_string(), true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_small_payload_not_truncated() {
        let payload = b"hello world";
        let (s, truncated) = truncate_payload(payload);
        assert_eq!(s, "hello world");
        assert!(!truncated);
    }

    #[test]
    fn test_truncate_exact_limit_not_truncated() {
        let payload = vec![b'x'; MAX_PAYLOAD_SIZE];
        let (s, truncated) = truncate_payload(&payload);
        assert_eq!(s.len(), MAX_PAYLOAD_SIZE);
        assert!(!truncated);
    }

    #[test]
    fn test_truncate_oversize_truncated() {
        let payload = vec![b'y'; MAX_PAYLOAD_SIZE + 1];
        let (s, truncated) = truncate_payload(&payload);
        assert_eq!(s.len(), PREVIEW_SIZE);
        assert!(truncated);
    }

    #[test]
    fn test_truncate_way_oversize() {
        let payload = vec![b'z'; 1024 * 1024]; // 1MB
        let (_, truncated) = truncate_payload(&payload);
        assert!(truncated);
    }
}
```

- [ ] **Step 2: Wire into lib.rs**

In `src-tauri/src/lib.rs`, add `pub mod ws_frames;` (alphabetically, near the other modules).

- [ ] **Step 3: Run the tests**

```bash
cargo test -p proxybot --lib ws_frames
```

Expected: 4 passed.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/ws_frames/mod.rs src-tauri/src/lib.rs
git commit -m "feat(ws_frames): add payload truncation helper

MAX_PAYLOAD_SIZE=64KB, PREVIEW_SIZE=1KB. Binary payloads go
through String::from_utf8_lossy for text display; the frontend
can use base64 for lossless hex rendering."
```

---

## Task 3: Add `get_ws_frames` DB query

**Files:**
- Modify: `src-tauri/src/db.rs` (add `get_ws_frames`)

- [ ] **Step 1: Write the failing test**

Open `src-tauri/src/db.rs` and find the existing test module. Add:

```rust
    #[test]
    fn test_get_ws_frames_returns_in_timestamp_order() {
        let conn = open_test_db();
        let req_id = "req-frames-1";
        record_ws_frame(&conn, req_id, "outgoing", 0x01, "first", None, 5, &Timestamp::now()).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        record_ws_frame(&conn, req_id, "incoming", 0x01, "second", None, 6, &Timestamp::now()).unwrap();
        
        let frames = get_ws_frames(&conn, req_id).unwrap();
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].payload, "first");
        assert_eq!(frames[0].direction, "outgoing");
        assert_eq!(frames[0].opcode, 0x01);
        assert_eq!(frames[1].payload, "second");
        assert_eq!(frames[1].direction, "incoming");
    }
    
    #[test]
    fn test_get_ws_frames_empty_for_unknown_request() {
        let conn = open_test_db();
        let frames = get_ws_frames(&conn, "nonexistent").unwrap();
        assert!(frames.is_empty());
    }
```

(Adapt to the project's existing test pattern — `open_test_db` and `Timestamp` may be named differently. Read the existing `test_record_ws_frame_persists` to see the pattern.)

- [ ] **Step 2: Run the test — it should fail to compile**

```bash
cargo test -p proxybot --lib db::tests::test_get_ws_frames_returns_in_timestamp_order
```

Expected: compile error (function doesn't exist).

- [ ] **Step 3: Add the function**

In `src-tauri/src/db.rs`, add this function (near the existing `record_ws_frame`):

```rust
pub fn get_ws_frames(conn: &Connection, request_id: &str) -> Result<Vec<crate::proxy::WsFrame>, String> {
    let mut stmt = conn.prepare(
        "SELECT direction, opcode, payload, size, timestamp
         FROM ws_frames WHERE request_id = ?1 ORDER BY timestamp ASC"
    ).map_err(|e| e.to_string())?;
    
    let rows = stmt.query_map([request_id], |row| {
        let opcode: i32 = row.get(1)?;
        let payload: String = row.get(2)?;
        let size: i64 = row.get(3)?;
        let timestamp: String = row.get(4)?;
        let truncated = (size as usize) > crate::ws_frames::MAX_PAYLOAD_SIZE;
        Ok(crate::proxy::WsFrame {
            direction: row.get(0)?,
            opcode: opcode as u8,
            payload,
            size: size as usize,
            timestamp,
            truncated,
        })
    }).map_err(|e| e.to_string())?;
    
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}
```

(Adapt the import paths to match the project's actual module structure — `crate::proxy::WsFrame` and `crate::ws_frames::MAX_PAYLOAD_SIZE` may differ. Read the existing imports in db.rs to get them right.)

- [ ] **Step 4: Run the tests**

```bash
cargo test -p proxybot --lib db::tests::test_get_ws_frames
```

Expected: 2 passed.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/db.rs
git commit -m "feat(db): add get_ws_frames query

Returns Vec<WsFrame> for a given request_id, ordered by
timestamp. Sets the truncated flag based on stored size vs
MAX_PAYLOAD_SIZE."
```

---

## Task 4: Add `get_ws_frames` Tauri command

**Files:**
- Modify: `src-tauri/src/proxy/commands.rs` (add command)
- Modify: `src-tauri/src/lib.rs` (register in `invoke_handler!`)

- [ ] **Step 1: Read the existing commands.rs to understand the pattern**

```bash
head -30 src-tauri/src/proxy/commands.rs
```

- [ ] **Step 2: Add the command**

In `src-tauri/src/proxy/commands.rs`, add (near other simple query commands):

```rust
#[tauri::command]
pub fn get_ws_frames(
    request_id: String,
) -> Result<Vec<crate::proxy::WsFrame>, String> {
    use crate::db;
    let conn = db::get_connection().map_err(|e| e.to_string())?;
    db::get_ws_frames(&conn, &request_id)
}
```

(Adapt the connection-acquisition pattern to match the existing commands — they may use `State<DbState>` or similar.)

- [ ] **Step 3: Register in lib.rs**

In `src-tauri/src/lib.rs`, find the `tauri::generate_handler!` macro and add `commands::get_ws_frames` to the list (or `proxy::commands::get_ws_frames` depending on the module structure).

- [ ] **Step 4: Compile-check**

```bash
cargo check -p proxybot --lib
```

Expected: 0 errors.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/proxy/commands.rs src-tauri/src/lib.rs
git commit -m "feat(commands): add get_ws_frames Tauri command

Wraps db::get_ws_frames for the frontend."
```

---

## Task 5: Emit `ws-frame:new` event after record_ws_frame

**Files:**
- Modify: `src-tauri/src/proxy/forward.rs` (emit event)

- [ ] **Step 1: Read the current call sites**

```bash
grep -n "record_ws_frame" /Users/doug/ai/system/proxybot/src-tauri/src/proxy/forward.rs
```

Two call sites: line 195 (outgoing) and line 214 (incoming). After each `record_ws_frame` call, we need to emit a Tauri event.

- [ ] **Step 2: Determine how to get the AppHandle**

Check if `forward.rs` already has access to a Tauri AppHandle, or if `ProxyContext` (which is the parent context) holds one. The simplest path is to use the `app_handle()` method on the runtime via `tauri::Manager` — but this needs the runtime to be passed in. The cleanest option for now:

- After each `record_ws_frame`, push a `(WsFrame, request_id)` tuple to a broadcast channel
- The `forward.rs` function spawns a small task that listens to that channel and emits the Tauri event
- OR: pass the AppHandle through the function signature

**The simplest workable approach**: Add a `broadcast::Sender<WsFrame>` to the existing `ProxyContext` (which already has `event_tx: broadcast::Sender<InterceptedRequest>`), and have a separate task in `lib.rs` that listens to it and emits Tauri events.

- [ ] **Step 3: Implement the simplest version**

Open `src-tauri/src/proxy/forward.rs`. After each `record_ws_frame` call (line 195 and 214), add code to build a `WsFrame` and emit it. The actual emission mechanism depends on what access is available. If `AppHandle` is not in scope, use the broadcast channel pattern:

```rust
// After record_ws_frame for outgoing (around line 195):
let frame = WsFrame {
    direction: "outgoing".to_string(),
    timestamp: ts.to_string(),
    payload: text,
    size: header.payload_len,
    opcode: header.opcode,
    truncated: header.payload_len > crate::ws_frames::MAX_PAYLOAD_SIZE,
};
let _ = ctx.ws_frame_tx.send((request_id.clone(), frame));
```

And similarly for incoming.

If `ProxyContext` doesn't have `ws_frame_tx`, add it:
- In `proxy/mod.rs`, add `pub(super) ws_frame_tx: broadcast::Sender<(String, WsFrame)>` to `ProxyContext`
- In `proxy/mod.rs::ProxyState::new()`, create it
- In `lib.rs`, spawn a task that listens to it and emits the Tauri event:

```rust
let ws_frame_rx = proxy_state.ws_frame_tx.subscribe();
tokio::spawn(async move {
    while let Ok((request_id, frame)) = ws_frame_rx.recv().await {
        let _ = app_handle.emit("ws-frame:new", WsFrameEvent { request_id, frame });
    }
});
```

(Adapt to the actual project structure — the exact wiring depends on what `ProxyContext` looks like and where it's instantiated.)

- [ ] **Step 4: Run the existing lib tests to confirm no regressions**

```bash
cargo test -p proxybot --lib
```

Expected: existing tests pass.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/proxy/forward.rs src-tauri/src/proxy/mod.rs src-tauri/src/lib.rs
git commit -m "feat(proxy): emit ws-frame:new event after record_ws_frame

Adds a broadcast channel from ProxyContext to a Tauri event
emitter. Every recorded WS frame now flows to the frontend
as a ws-frame:new event in addition to being persisted to the
ws_frames table."
```

---

## Task 6: Frontend types and opcode helper

**Files:**
- Create: `src/components/ws-frames/types.ts`

- [ ] **Step 1: Create the file**

Create `src/components/ws-frames/types.ts`:

```typescript
// Shared types for the WS Frame Viewer components.

export interface WsFrame {
  direction: "incoming" | "outgoing";
  timestamp: string;
  payload: string;
  size: number;
  opcode: number;
  truncated: boolean;
}

export interface WsFrameEvent {
  request_id: string;
  frame: WsFrame;
}

export function getOpcodeName(opcode: number): string {
  switch (opcode) {
    case 0x01:
      return "Text";
    case 0x02:
      return "Binary";
    case 0x08:
      return "Close";
    case 0x09:
      return "Ping";
    case 0x0a:
      return "Pong";
    default:
      return "Unknown";
  }
}
```

- [ ] **Step 2: Typecheck**

```bash
pnpm typecheck
```

Expected: 0 errors.

- [ ] **Step 3: Commit**

```bash
git add src/components/ws-frames/types.ts
git commit -m "feat(ui): add WS frame types and opcode name helper"
```

---

## Task 7: HexDump component

**Files:**
- Create: `src/components/ws-frames/HexDump.tsx`

- [ ] **Step 1: Create the file**

Create `src/components/ws-frames/HexDump.tsx`:

```tsx
interface HexDumpProps {
  payload: string;
  truncated: boolean;
}

/**
 * Render a string as a Latin-1 byte hex dump, 16 bytes per line.
 * Note: the input is already a lossy UTF-8 string (binary frames
 * were converted via String::from_utf8_lossy on the backend).
 */
export function HexDump({ payload, truncated }: HexDumpProps) {
  const lines: string[] = [];
  const bytes: number[] = [];
  for (let i = 0; i < payload.length; i++) {
    bytes.push(payload.charCodeAt(i) & 0xff);
  }
  for (let i = 0; i < bytes.length; i += 16) {
    const chunk = bytes.slice(i, i + 16);
    const hex = chunk
      .map((b) => b.toString(16).padStart(2, "0"))
      .join(" ");
    const ascii = chunk
      .map((b) => (b >= 32 && b < 127 ? String.fromCharCode(b) : "."))
      .join("");
    const offset = i.toString(16).padStart(8, "0");
    lines.push(`${offset}  ${hex.padEnd(48)}  ${ascii}`);
  }
  return (
    <div>
      {truncated && (
        <p className="text-xs text-amber-600 mb-2">
          Binary frame preview may be lossy. Hex shows first 1KB only.
        </p>
      )}
      <pre className="bg-muted/30 rounded p-3 font-mono text-xs overflow-auto">
        {lines.join("\n") || "(empty)"}
      </pre>
    </div>
  );
}
```

(Adapt class names to match the project's design system — see existing `DeviceQrPanel.tsx` for the pattern.)

- [ ] **Step 2: Typecheck**

```bash
pnpm typecheck
```

Expected: 0 errors.

- [ ] **Step 3: Commit**

```bash
git add src/components/ws-frames/HexDump.tsx
git commit -m "feat(ui): add HexDump component for binary WS frame inspection"
```

---

## Task 8: FrameDetail component

**Files:**
- Create: `src/components/ws-frames/FrameDetail.tsx`

- [ ] **Step 1: Create the file**

Create `src/components/ws-frames/FrameDetail.tsx`:

```tsx
import { useState } from "react";
import { WsFrame, getOpcodeName } from "./types";
import { HexDump } from "./HexDump";

interface FrameDetailProps {
  frame: WsFrame;
}

function formatTimestamp(ts: string): string {
  // Simple formatter; adapt to project's existing time-formatting util
  const d = new Date(ts);
  return d.toLocaleTimeString();
}

export function FrameDetail({ frame }: FrameDetailProps) {
  const [viewMode, setViewMode] = useState<"text" | "hex">("text");

  return (
    <div className="space-y-4 p-4">
      {/* Metadata grid */}
      <div className="grid grid-cols-2 gap-2 text-sm">
        <div>
          <span className="text-muted-foreground">Direction: </span>
          <span
            className={
              frame.direction === "incoming" ? "text-green-600" : "text-blue-600"
            }
          >
            {frame.direction === "incoming" ? "← incoming" : "→ outgoing"}
          </span>
        </div>
        <div>
          <span className="text-muted-foreground">Opcode: </span>
          {frame.opcode} ({getOpcodeName(frame.opcode)})
        </div>
        <div>
          <span className="text-muted-foreground">Size: </span>
          {frame.size} bytes
        </div>
        <div>
          <span className="text-muted-foreground">Time: </span>
          {formatTimestamp(frame.timestamp)}
        </div>
      </div>

      {/* Text/Hex toggle */}
      <div className="flex gap-2">
        <button
          onClick={() => setViewMode("text")}
          className={`text-xs px-3 py-1 rounded ${
            viewMode === "text" ? "bg-primary text-primary-foreground" : "bg-muted"
          }`}
        >
          Text
        </button>
        <button
          onClick={() => setViewMode("hex")}
          className={`text-xs px-3 py-1 rounded ${
            viewMode === "hex" ? "bg-primary text-primary-foreground" : "bg-muted"
          }`}
        >
          Hex
        </button>
      </div>

      {/* Payload */}
      {viewMode === "text" ? (
        <pre className="bg-muted/30 rounded p-3 font-mono text-xs overflow-auto whitespace-pre-wrap break-all">
          {frame.payload || "(empty)"}
        </pre>
      ) : (
        <HexDump payload={frame.payload} truncated={frame.truncated} />
      )}
    </div>
  );
}
```

- [ ] **Step 2: Typecheck**

```bash
pnpm typecheck
```

Expected: 0 errors.

- [ ] **Step 3: Commit**

```bash
git add src/components/ws-frames/FrameDetail.tsx
git commit -m "feat(ui): add FrameDetail with text/hex toggle and metadata"
```

---

## Task 9: WsFramesView main component

**Files:**
- Create: `src/components/ws-frames/WsFramesView.tsx`

- [ ] **Step 1: Create the file**

Create `src/components/ws-frames/WsFramesView.tsx`:

```tsx
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { WsFrame, WsFrameEvent, getOpcodeName } from "./types";
import { FrameDetail } from "./FrameDetail";

interface WsFramesViewProps {
  requestId: string;
}

function truncate(s: string, n: number): string {
  if (s.length <= n) return s;
  return s.slice(0, n) + "…";
}

export function WsFramesView({ requestId }: WsFramesViewProps) {
  const [frames, setFrames] = useState<WsFrame[]>([]);
  const [selectedFrame, setSelectedFrame] = useState<WsFrame | null>(null);

  useEffect(() => {
    let unlistenFn: (() => void) | null = null;

    // Initial fetch
    invoke<WsFrame[]>("get_ws_frames", { requestId })
      .then((initial) => setFrames(initial))
      .catch(console.error);

    // Subscribe to real-time updates
    listen<WsFrameEvent>("ws-frame:new", (event) => {
      if (event.payload.request_id === requestId) {
        setFrames((prev) => [...prev, event.payload.frame]);
      }
    }).then((fn) => {
      unlistenFn = fn;
    });

    return () => {
      if (unlistenFn) unlistenFn();
    };
  }, [requestId]);

  return (
    <div className="flex h-full">
      {/* Frame list (left half) */}
      <div className="w-1/2 border-r overflow-auto" data-testid="ws-frames-list">
        {frames.length === 0 ? (
          <p className="p-4 text-sm text-muted-foreground">
            No WebSocket frames for this request.
          </p>
        ) : (
          frames.map((frame, i) => (
            <div
              key={i}
              onClick={() => setSelectedFrame(frame)}
              data-testid="ws-frame-row"
              className={`flex items-center px-3 py-2 border-b cursor-pointer ${
                selectedFrame === frame ? "bg-muted" : "hover:bg-muted/30"
              }`}
            >
              <span className="w-4 text-sm">
                {frame.direction === "incoming" ? "←" : "→"}
              </span>
              <span className="w-12 font-mono text-xs text-muted-foreground">
                {getOpcodeName(frame.opcode)}
              </span>
              <span className="flex-1 truncate text-sm font-mono">
                {truncate(frame.payload, 30)}
                {frame.truncated && (
                  <span className="ml-1 text-xs text-amber-600">(truncated)</span>
                )}
              </span>
              <span className="text-xs text-muted-foreground">
                {new Date(frame.timestamp).toLocaleTimeString()}
              </span>
            </div>
          ))
        )}
      </div>

      {/* Frame detail (right half) */}
      <div className="w-1/2 overflow-auto">
        {selectedFrame ? (
          <FrameDetail frame={selectedFrame} />
        ) : (
          <p className="p-4 text-sm text-muted-foreground">
            Select a frame to view details.
          </p>
        )}
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Typecheck**

```bash
pnpm typecheck
```

Expected: 0 errors.

- [ ] **Step 3: Commit**

```bash
git add src/components/ws-frames/WsFramesView.tsx
git commit -m "feat(ui): add WsFramesView with list, detail, and real-time subscribe

Fetches initial frames via get_ws_frames, subscribes to
ws-frame:new for streaming updates. Split view: list on left,
detail on right."
```

---

## Task 10: Wire WsFramesView into RequestDetail

**Files:**
- Modify: `src/components/.../RequestDetail.tsx` (add tab)

- [ ] **Step 1: Read RequestDetail to find the right place to add the tab**

```bash
grep -n "tab\|Tab\|WSS\|ws_frame" /Users/doug/ai/system/proxybot/src/components/.../RequestDetail.tsx 2>/dev/null | head -10
```

(Adjust the path — find the actual RequestDetail file.)

The existing `RequestDetail` likely has a tab strip (Params, Headers, Body, etc.). Add a new tab "WebSocket Frames" that shows `WsFramesView` when the request was a WS upgrade.

- [ ] **Step 2: Add the tab**

In `RequestDetail.tsx`:

```tsx
import { WsFramesView } from "@/components/ws-frames/WsFramesView";

// In the tab strip, add:
{(request.is_websocket ?? false) && (
  <button
    onClick={() => setActiveTab("ws-frames")}
    className={activeTab === "ws-frames" ? "..." : "..."}
  >
    WebSocket Frames
  </button>
)}

// In the tab content, add:
{activeTab === "ws-frames" && (request.is_websocket ?? false) && (
  <WsFramesView requestId={request.id} />
)}
```

(Adapt to the project's actual tab API — the existing pattern may use a Tabs component, a state variable, or another abstraction.)

- [ ] **Step 3: Typecheck**

```bash
pnpm typecheck
```

Expected: 0 errors.

- [ ] **Step 4: Commit**

```bash
git add src/components/.../RequestDetail.tsx
git commit -m "feat(ui): wire WsFramesView into RequestDetail as a new tab

Only shown when the request was a WebSocket upgrade (is_websocket
flag). Lazy — only loaded when the user clicks the tab."
```

---

## Task 11: E2E tests

**Files:**
- Create: `e2e/ws-frames.spec.ts`

- [ ] **Step 1: Read an existing E2E test for the pattern**

```bash
ls e2e/
head -30 e2e/qr-onboarding.spec.ts
```

- [ ] **Step 2: Create the E2E test file**

Create `e2e/ws-frames.spec.ts`:

```typescript
import { test, expect } from "@playwright/test";

test("ws_frames_view_shows_empty_for_non_ws_request", async ({ page }) => {
  await page.goto("/requests/req-1");
  // Mock get_ws_frames to return empty array
  await page.evaluate(() => {
    // @ts-ignore
    window.__TAURI_INVOKE__ = async (cmd: string, args: any) => {
      if (cmd === "get_ws_frames") return [];
      return null;
    };
  });
  await expect(page.getByText("No WebSocket frames for this request.")).toBeVisible();
});

test("ws_frames_view_shows_frames_after_ws_conversation", async ({ page }) => {
  await page.goto("/requests/req-2");
  await page.evaluate(() => {
    // @ts-ignore
    window.__TAURI_INVOKE__ = async (cmd: string, args: any) => {
      if (cmd === "get_ws_frames") {
        return [
          {
            direction: "outgoing",
            timestamp: "2026-06-14T10:00:00Z",
            payload: "hello",
            size: 5,
            opcode: 0x01,
            truncated: false,
          },
          {
            direction: "incoming",
            timestamp: "2026-06-14T10:00:01Z",
            payload: "world",
            size: 5,
            opcode: 0x01,
            truncated: false,
          },
        ];
      }
      return null;
    };
  });
  await expect(page.getByTestId("ws-frame-row")).toHaveCount(2);
  await expect(page.getByText("Text").first()).toBeVisible();
});

test("ws_frames_view_text_hex_toggle", async ({ page }) => {
  // Click a frame, click Hex, verify hex view appears
  await page.goto("/requests/req-3");
  // ... similar mocking
  await page.getByTestId("ws-frame-row").first().click();
  await page.getByRole("button", { name: "Hex" }).click();
  await expect(page.getByText(/hello/)).toBeVisible(); // hex dump contains "hello" in ASCII column
});

test("ws_frames_view_realtime_append", async ({ page }) => {
  // Mock the event listener, emit a new frame, verify it appears
  await page.goto("/requests/req-4");
  // ... verify a frame appended after a delay
});
```

(Adapt the mocking pattern to match the project's existing E2E setup — look at `e2e/ssl-bypass.spec.ts` and `e2e/qr-onboarding.spec.ts` for the actual mock pattern. The project uses `tauri-mock.ts` fixture.)

- [ ] **Step 3: Run the E2E tests**

```bash
pnpm test:e2e -- ws-frames
```

Expected: 4 tests pass (or note if E2E can't run).

- [ ] **Step 4: Commit**

```bash
git add e2e/ws-frames.spec.ts
git commit -m "test(e2e): add Playwright tests for WS Frame Viewer

Empty state, multi-frame list, text/hex toggle, real-time append."
```

---

## Task 12: Final verification

**Files:** none modified

- [ ] **Step 1: Run `cargo build`**

```bash
cargo build
```

Expected: 0 errors.

- [ ] **Step 2: Run the full test suite**

```bash
cargo test
```

Expected: all tests pass (existing + new ws_frames tests).

- [ ] **Step 3: Run `pnpm typecheck`**

```bash
pnpm typecheck
```

Expected: 0 errors.

- [ ] **Step 4: Run `pnpm test:ui`**

```bash
pnpm test:ui
```

Expected: existing tests pass.

- [ ] **Step 5: Run `cargo clippy`**

```bash
cargo clippy -p proxybot --no-deps
```

Expected: no new clippy warnings from this branch.

- [ ] **Step 6: Final commit if any cleanup needed**

```bash
git status
# If uncommitted changes:
git add -A
git commit -m "chore: post-implementation cleanup"
```

---

## Manual verification (out-of-band)

Real-device testing:
1. Start ProxyBot
2. Open an app that uses WebSocket (e.g., WeChat)
3. Select the captured WS request in the traffic list
4. Click the "WebSocket Frames" tab
5. Verify frames appear in real time as messages are exchanged
6. Click a frame, verify metadata and payload
7. Click Hex, verify hex dump renders

---

## References

- Spec: `docs/superpowers/specs/2026-06-14-ws-frame-viewer-design.md`
- Existing WsFrame type: `src-tauri/src/proxy/mod.rs:87`
- Existing ws_frames DB table: `src-tauri/src/db.rs:332-345`
- Existing record_ws_frame function: `src-tauri/src/db.rs:800`
- Existing forward.rs WS capture: `src-tauri/src/proxy/forward.rs:195,214`
- ProxyContext: `src-tauri/src/proxy/mod.rs` (after line 81)
- Frida message streaming pattern (for emit): `src-tauri/src/frida/mod.rs`
