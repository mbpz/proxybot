# WebSocket Frame Viewer Design

**Date:** 2026-06-14
**Author:** Claude
**Status:** Approved (pending spec self-review)

---

## 1. Context

ProxyBot 当前已经 capture WebSocket frames（`proxy/forward.rs:195,214` 调用 `record_ws_frame`），并把 frames 持久化到 `ws_frames` 表。但前端没有 viewer：用户在 Traffic 列表选中一个 WS request 后，只能看到 request/response 头，没有 frame 级可视化。

竞品参考：Proxyman 提供完整帧列表 + 二进制 hex 查看；Charles 支持 text/binary 切换；mitmproxy 的 mitmweb 提供 WS 消息流。

本文设计补完 WS Frame Viewer 的 v1 pipeline：backend query command + real-time event stream + 3 个 React 组件（帧列表、帧详情、hex dump）。

## 2. Goals & Non-Goals

### Goals

- 在 RequestDetail 面板里新增 "WebSocket Frames" tab，显示该 request 的所有 frames
- 实时推送新 frame（通过 Tauri event `ws-frame:new`）
- Frame 列表显示 direction（incoming/outgoing）+ opcode（Text/Binary/Close/Ping/Pong）+ payload 预览 + timestamp
- Frame 详情面板支持 text / hex 切换
- 大 payload 截断保护（> 64KB 只存前 1KB + truncated flag）
- WsFrame struct 补 opcode 字段

### Non-Goals

- Frame replay（重发单个 frame 到服务器）—— v2
- Frame 搜索/filter（按 opcode、按 direction、按 payload 关键字）—— v2
- WebSocket close handshake 分析（close code、reason）—— v2
- 二进制 frame 的自动格式化（protobuf、msgpack、JSON 自动识别）—— v2
- Frame 导出（保存为 .har 或 .jsonl）—— v2

## 3. Architecture

### 3.1 High-level

```
┌─────────────────┐  invoke("get_ws_frames", {requestId})  ┌──────────────┐
│ WsFramesView    │ ──────────────────────────────────────► │ proxy/      │
│ (React)         │ ◄────────────────────────────────────── │ commands.rs │
└─────────────────┘  Vec<WsFrame>                            └──────────────┘
         │                                                       │
         │  listen("ws-frame:new", (frame) => append)            │ (proxy emits
         ▼                                                       │  on capture)
┌─────────────────┐                                       ┌──────────────┐
│ WsFramesView    │ ◄────────────────────────────────────── │ proxy/      │
│ (auto-append)   │  WsFrame                              │ forward.rs  │
└─────────────────┘                                       └──────────────┘
         │
         ▼  user selects frame
┌─────────────────┐
│ FrameDetail     │  text/hex toggle, payload render
│ (React)         │  metadata grid (direction, opcode, size, time)
└─────────────────┘
```

### 3.2 数据结构

```rust
// proxy/mod.rs — 扩展现有 WsFrame
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WsFrame {
    pub direction: String,        // "incoming" | "outgoing"
    pub timestamp: String,
    pub payload: String,          // UTF-8 string (大 payload 截断)
    pub size: usize,              // 原始 size（截断前）
    pub opcode: u8,               // 新增：0x01=Text, 0x02=Binary, 0x08=Close, 0x09=Ping, 0x0A=Pong
    pub truncated: bool,          // 新增：true if payload was truncated
}
```

### 3.3 Tauri 命令

```rust
// proxy/commands.rs
#[tauri::command]
fn get_ws_frames(request_id: String) -> Result<Vec<WsFrame>, String>;

// 实时事件由 forward.rs 在 record_ws_frame 时 emit
app_handle.emit("ws-frame:new", &frame);
```

### 3.4 Truncation 逻辑

```rust
const MAX_PAYLOAD_SIZE: usize = 64 * 1024; // 64KB
const PREVIEW_SIZE: usize = 1024;          // 1KB preview

fn truncate_payload(payload: Vec<u8>) -> (String, bool) {
    if payload.len() <= MAX_PAYLOAD_SIZE {
        (String::from_utf8_lossy(&payload).to_string(), false)
    } else {
        let mut bytes = payload;
        bytes.truncate(PREVIEW_SIZE);
        (String::from_utf8_lossy(&bytes).to_string(), true)
    }
}
```

Binary frames 走 `String::from_utf8_lossy`（无效 UTF-8 字节替换为 U+FFFD），hex 视图用 base64 编码 raw bytes。

### 3.5 Frontend 组件

```
RequestDetail/
├── ...existing tabs...
└── WsFramesView (new tab)
    ├── FrameList (left half)
    │   └── frame row: ← / →, opcode name, payload preview, timestamp
    └── FrameDetail (right half, when frame selected)
        ├── MetadataGrid
        ├── ViewModeToggle (text/hex)
        └── PayloadRenderer
            ├── TextView (if mode=text)
            └── HexDump (if mode=hex)
```

## 4. Data Structures

### 4.1 Rust 端（扩展现有 WsFrame）

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WsFrame {
    pub direction: String,        // "incoming" | "outgoing"
    pub timestamp: String,        // ISO 8601
    pub payload: String,          // 截断后的 UTF-8 字符串
    pub size: usize,              // 原始 size
    pub opcode: u8,               // 新增
    pub truncated: bool,          // 新增
}

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

### 4.2 DB 查询

```rust
// db.rs — 新增 get_ws_frames
pub fn get_ws_frames(conn: &Connection, request_id: &str) -> Result<Vec<WsFrame>, String> {
    let mut stmt = conn.prepare(
        "SELECT direction, opcode, payload, size, timestamp
         FROM ws_frames WHERE request_id = ?1 ORDER BY timestamp ASC"
    ).map_err(|e| e.to_string())?;
    
    let rows = stmt.query_map([request_id], |row| {
        let opcode: i32 = row.get(1)?;
        let payload: String = row.get(2)?;
        let size: i64 = row.get(3)?;
        let timestamp: String = row.get(4)?;
        Ok(WsFrame {
            direction: row.get(0)?,
            opcode: opcode as u8,
            payload,
            size: size as usize,
            timestamp,
            truncated: size as usize > MAX_PAYLOAD_SIZE,  // 截断标记
        })
    }).map_err(|e| e.to_string())?;
    
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}
```

### 4.3 Frontend 类型

```typescript
// src/components/ws-frames/types.ts
export interface WsFrame {
  direction: "incoming" | "outgoing";
  timestamp: string;
  payload: string;
  size: number;
  opcode: number;
  truncated: boolean;
}

export function getOpcodeName(opcode: number): string {
  switch (opcode) {
    case 0x01: return "Text";
    case 0x02: return "Binary";
    case 0x08: return "Close";
    case 0x09: return "Ping";
    case 0x0A: return "Pong";
    default: return "Unknown";
  }
}
```

## 5. IPC & Real-time

### 5.1 Tauri command

```rust
#[tauri::command]
pub fn get_ws_frames(request_id: String) -> Result<Vec<WsFrame>, String> {
    let conn = get_db_connection()?;
    db::get_ws_frames(&conn, &request_id)
}
```

### 5.2 实时事件流

在 `proxy/forward.rs:record_ws_frame` 之后，emit 一个 Tauri event：

```rust
let _ = record_ws_frame(&conn, &request_id, direction, opcode, &text, None, size, &ts);
// 新增：emit 实时事件
let frame = WsFrame { direction: direction.to_string(), opcode, payload: text, size, timestamp: ts.to_string(), truncated: size > MAX_PAYLOAD_SIZE };
let _ = app_handle.emit("ws-frame:new", &frame);
```

注意：`app_handle` 在 forward.rs 里的可用性需要检查（通过 `tauri::Manager` 的 `app_handle()` 方法）。如果 forward.rs 不能直接拿 app_handle，需要通过 Mutex<Option<AppHandle>> 共享，或者 emit 改为通过一个 broadcast::Sender。

## 6. Components

### 6.1 WsFramesView.tsx

```tsx
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export function WsFramesView({ requestId }: { requestId: string }) {
  const [frames, setFrames] = useState<WsFrame[]>([]);
  const [selectedFrame, setSelectedFrame] = useState<WsFrame | null>(null);
  
  useEffect(() => {
    // Initial fetch
    invoke<WsFrame[]>("get_ws_frames", { requestId })
      .then(setFrames)
      .catch(console.error);
    
    // Subscribe to real-time updates
    const unlisten = listen<WsFrame>("ws-frame:new", (event) => {
      if (event.payload.request_id === requestId) {
        setFrames(prev => [...prev, event.payload]);
      }
    });
    
    return () => { unlisten.then(fn => fn()); };
  }, [requestId]);
  
  // ... render frame list + detail split view
}
```

### 6.2 FrameDetail.tsx + HexDump

```tsx
function FrameDetail({ frame }: { frame: WsFrame }) {
  const [viewMode, setViewMode] = useState<'text' | 'hex'>('text');
  
  return (
    <div>
      <MetadataGrid frame={frame} />
      <ViewModeToggle mode={viewMode} onChange={setViewMode} />
      {viewMode === 'text' ? (
        <pre>{frame.payload}</pre>
      ) : (
        <HexDump payload={frame.payload} truncated={frame.truncated} />
      )}
    </div>
  );
}
```

HexDump 把 string 字符当作 Latin-1 字节渲染（每行 16 字节，offset | hex | ASCII）。对 binary frame 的 `payload`（已 lossy 转换）显示警告 "Binary frame preview may be lossy"。

## 7. Error Handling

| 场景 | 处理 |
|---|---|
| `get_ws_frames` 找不到 request | 返回 `Vec::new()`（空数组），不报错 |
| 实时事件期间 connection drop | frontend `listen` 自动 reconnect |
| WS frame 录制失败 | `record_ws_frame` 现有错误处理保持不变，不影响 HTTP 代理 |
| DB 查询超时 | 5 秒后返回 `Err`，frontend 显示 toast |
| Truncated binary frame 的 hex 渲染 | 在 HexDump 顶部加警告 "Binary frame, preview truncated" |
| 0 frames | 显示 "No WebSocket frames for this request" |

## 8. Testing

### 8.1 单元测试

**db.rs:**
- `test_get_ws_frames_returns_all_frames` — 插入 3 frames，验证返回顺序
- `test_get_ws_frames_filters_by_request_id` — 插入 2 个不同 request 的 frames，验证只返回匹配的
- `test_get_ws_frames_empty_for_unknown_request` — 返回空 Vec
- `test_ws_frame_truncated_flag` — 大 payload 应该有 truncated=true

**proxy/commands.rs:**
- `test_get_ws_frames_tauri_command` — invoke command，验证返回

**proxy/mod.rs:**
- `test_get_opcode_name` — 所有 5 个 opcode name

### 8.2 E2E

`e2e/ws-frames.spec.ts`:
- `ws_frames_view_renders_empty_for_non_ws_request` — 选 HTTP request，显示 "No frames"
- `ws_frames_view_shows_frames_after_ws_conversation` — mock get_ws_frames 返回 3 frames，验证显示
- `ws_frames_view_text_hex_toggle` — 点 hex tab，验证 hex dump
- `ws_frames_view_realtime_append` — mock listen 事件触发，验证 frame 追加

### 8.3 手动

- 启动 ProxyBot
- 用手机 app 发 WS 消息
- 打开 request detail
- 验证 frames 实时出现
- 点 hex 切换验证 hex dump

## 9. Implementation Notes

### 9.1 Files Changed

**修改：**
- `src-tauri/src/proxy/mod.rs` — 给 WsFrame 加 `opcode` + `truncated` 字段
- `src-tauri/src/proxy/commands.rs` — 新增 `get_ws_frames` Tauri 命令
- `src-tauri/src/proxy/forward.rs` — 在 `record_ws_frame` 后 emit `ws-frame:new` 事件
- `src-tauri/src/db.rs` — 新增 `get_ws_frames` query + `test_record_ws_frame_persists` 已存在
- `src/components/.../RequestDetail.tsx` — 新增 "WebSocket Frames" tab

**新建：**
- `src-tauri/src/ws_frames/mod.rs` — `get_opcode_name` + truncation 逻辑
- `src/components/ws-frames/WsFramesView.tsx`
- `src/components/ws-frames/FrameDetail.tsx`
- `src/components/ws-frames/HexDump.tsx`
- `src/components/ws-frames/types.ts`
- `e2e/ws-frames.spec.ts`

### 9.2 截断阈值

`MAX_PAYLOAD_SIZE = 64KB` 和 `PREVIEW_SIZE = 1KB` 是常量。如果未来需要可配置，可以移到 `config.rs`。

### 9.3 AppHandle 获取

`forward.rs` 当前是 `pub(super) fn forward(...)`，需要 `app_handle: tauri::AppHandle` 参数。检查现有的 `ProxyContext` 是否已经持有 AppHandle。如果没有，需要修改 `ProxyContext` 共享 AppHandle（通过 `Arc<tauri::AppHandle>` 或 broadcast::Sender<WsFrame>）。

## 10. References

- 现有 spec: `docs/superpowers/specs/2026-05-09-ws-frame-viewer.md` (Draft)
- 现有 WsFrame type: `src-tauri/src/proxy/mod.rs:87`
- 现有 ws_frames DB: `src-tauri/src/db.rs:332-345`
- 现有 record_ws_frame: `src-tauri/src/db.rs:800-825`
- 现有 forward.rs WS capture: `src-tauri/src/proxy/forward.rs:195,214`
- Frida message streaming 模式 (Tauri event 订阅): `src/stores/sslBypassStore.tsx`

## 11. Self-Review Notes

- Placeholder 扫描：没有 TBD/TODO
- Internal consistency：WsFrame opcode 字段在 Rust 和 TS 两边都有定义
- Scope check：单一功能（WS frame viewer），1-2 天工作量
- Ambiguity check：truncation 阈值明确（64KB），opcode 名明确（5 个标准值）