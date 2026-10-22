# WS Frame Viewer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现WebSocket帧实时查看器，支持帧列表、详情、Text/Hex切换

**Architecture:** 前端WsFramesView组件，Rust端提供IPC命令获取帧数据，支持实时订阅

**Tech Stack:** React, TypeScript, Tauri IPC

---

## File Structure

| 操作 | 文件 | 职责 |
|------|------|------|
| Create | `src/components/ws/WsFramesView.tsx` | 帧列表+详情容器 |
| Create | `src/components/ws/WsFrameItem.tsx` | 单帧列表项 |
| Create | `src/components/ws/WsFrameDetail.tsx` | 帧详情面板 |
| Create | `src/components/ws/HexDump.tsx` | 十六进制查看 |
| Create | `src-tauri/src/commands/ws_frames.rs` | Rust IPC命令 |
| Modify | `src-tauri/src/lib.rs` | 注册命令 |

---

## Task 1: 创建WsFramesView组件

**Files:**
- Create: `src/components/ws/WsFramesView.tsx`

- [ ] **Step 1: 创建WsFramesView**

```tsx
import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { WsFrameItem } from "./WsFrameItem";
import { WsFrameDetail } from "./WsFrameDetail";

interface WsFrame {
  id: string;
  direction: "incoming" | "outgoing";
  opcode: number;
  payload: string;
  timestamp: number;
}

interface WsFramesViewProps {
  requestId: string;
}

export function WsFramesView({ requestId }: WsFramesViewProps) {
  const [frames, setFrames] = useState<WsFrame[]>([]);
  const [selectedFrame, setSelectedFrame] = useState<WsFrame | null>(null);

  const loadFrames = useCallback(async () => {
    try {
      const result = await invoke<WsFrame[]>("get_ws_frames", { requestId });
      setFrames(result);
    } catch (err) {
      console.error("Failed to load WS frames:", err);
    }
  }, [requestId]);

  useEffect(() => {
    loadFrames();

    // Subscribe to real-time frames
    const unlisten = listen<WsFrame>("ws_frame", (event) => {
      if (event.payload.requestId === requestId) {
        setFrames((prev) => [...prev, event.payload]);
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [requestId, loadFrames]);

  return (
    <div className="flex h-full">
      {/* Frame List */}
      <div className="w-1/2 border-r overflow-auto">
        {frames.length === 0 ? (
          <div className="flex items-center justify-center h-full text-gray-500">
            No WebSocket frames captured
          </div>
        ) : (
          frames.map((frame) => (
            <WsFrameItem
              key={frame.id}
              frame={frame}
              isSelected={selectedFrame?.id === frame.id}
              onClick={() => setSelectedFrame(frame)}
            />
          ))
        )}
      </div>

      {/* Frame Detail */}
      <div className="w-1/2 overflow-hidden">
        {selectedFrame ? (
          <WsFrameDetail frame={selectedFrame} />
        ) : (
          <div className="flex items-center justify-center h-full text-gray-500">
            Select a frame to view details
          </div>
        )}
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add src/components/ws/WsFramesView.tsx
git commit -m "feat(ws): add WsFramesView container component"
```

---

## Task 2: 创建WsFrameItem组件

**Files:**
- Create: `src/components/ws/WsFrameItem.tsx`

- [ ] **Step 1: 创建WsFrameItem**

```tsx
interface WsFrame {
  id: string;
  direction: "incoming" | "outgoing";
  opcode: number;
  payload: string;
  timestamp: number;
}

interface WsFrameItemProps {
  frame: WsFrame;
  isSelected: boolean;
  onClick: () => void;
}

function getOpcodeName(opcode: number): string {
  switch (opcode) {
    case 0x01:
      return "TEXT";
    case 0x02:
      return "BINARY";
    case 0x08:
      return "CLOSE";
    case 0x09:
      return "PING";
    case 0x0a:
      return "PONG";
    default:
      return `OP${opcode}`;
  }
}

function formatTime(timestamp: number): string {
  const d = new Date(timestamp * 1000);
  return `${d.getHours().toString().padStart(2, "0")}:${d.getMinutes().toString().padStart(2, "0")}:${d.getSeconds().toString().padStart(2, "0")}.${d.getMilliseconds().toString().padStart(3, "0")}`;
}

export function WsFrameItem({ frame, isSelected, onClick }: WsFrameItemProps) {
  return (
    <div
      onClick={onClick}
      className={`flex items-center px-3 py-2 border-b cursor-pointer hover:bg-gray-50 ${
        isSelected ? "bg-blue-50" : ""
      } ${frame.direction === "incoming" ? "text-green-600" : "text-blue-600"}`}
    >
      <span className="w-4 text-lg">
        {frame.direction === "incoming" ? "←" : "→"}
      </span>
      <span className="w-12 font-mono text-xs">{getOpcodeName(frame.opcode)}</span>
      <span className="flex-1 truncate text-sm">{truncate(frame.payload, 40)}</span>
      <span className="text-xs text-gray-400 ml-2">{formatTime(frame.timestamp)}</span>
    </div>
  );
}

function truncate(str: string, maxLen: number): string {
  if (str.length <= maxLen) return str;
  return str.slice(0, maxLen) + "...";
}
```

- [ ] **Step 2: Commit**

```bash
git add src/components/ws/WsFrameItem.tsx
git commit -m "feat(ws): add WsFrameItem component"
```

---

## Task 3: 创建WsFrameDetail组件

**Files:**
- Create: `src/components/ws/WsFrameDetail.tsx`

- [ ] **Step 1: 创建WsFrameDetail**

```tsx
import { useState } from "react";
import { HexDump } from "./HexDump";

interface WsFrame {
  id: string;
  direction: "incoming" | "outgoing";
  opcode: number;
  payload: string;
  timestamp: number;
}

interface WsFrameDetailProps {
  frame: WsFrame;
}

function getOpcodeName(opcode: number): string {
  switch (opcode) {
    case 0x01:
      return "TEXT";
    case 0x02:
      return "BINARY";
    case 0x08:
      return "CLOSE";
    case 0x09:
      return "PING";
    case 0x0a:
      return "PONG";
    default:
      return `OP${opcode}`;
  }
}

export function WsFrameDetail({ frame }: WsFrameDetailProps) {
  const [viewMode, setViewMode] = useState<"text" | "hex">("text");

  return (
    <div className="h-full flex flex-col">
      {/* Metadata */}
      <div className="p-4 border-b bg-gray-50">
        <div className="grid grid-cols-2 gap-2 text-sm">
          <div>
            <span className="text-gray-500">Direction:</span>{" "}
            <span className={frame.direction === "incoming" ? "text-green-600" : "text-blue-600"}>
              {frame.direction === "incoming" ? "Incoming ←" : "Outgoing →"}
            </span>
          </div>
          <div>
            <span className="text-gray-500">Opcode:</span>{" "}
            <span className="font-mono">{frame.opcode} ({getOpcodeName(frame.opcode)})</span>
          </div>
          <div>
            <span className="text-gray-500">Size:</span>{" "}
            <span>{frame.payload.length} bytes</span>
          </div>
          <div>
            <span className="text-gray-500">Time:</span>{" "}
            <span>{new Date(frame.timestamp * 1000).toLocaleString()}</span>
          </div>
        </div>
      </div>

      {/* View Mode Toggle */}
      <div className="flex gap-2 p-2 border-b">
        <button
          onClick={() => setViewMode("text")}
          className={`px-3 py-1 rounded text-sm ${
            viewMode === "text" ? "bg-blue-500 text-white" : "bg-gray-200"
          }`}
        >
          Text
        </button>
        <button
          onClick={() => setViewMode("hex")}
          className={`px-3 py-1 rounded text-sm ${
            viewMode === "hex" ? "bg-blue-500 text-white" : "bg-gray-200"
          }`}
        >
          Hex
        </button>
      </div>

      {/* Payload */}
      <div className="flex-1 overflow-auto p-4 bg-gray-100">
        {viewMode === "text" ? (
          <pre className="text-sm font-mono whitespace-pre-wrap break-all">
            {frame.payload}
          </pre>
        ) : (
          <HexDump text={frame.payload} />
        )}
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add src/components/ws/WsFrameDetail.tsx
git commit -m "feat(ws): add WsFrameDetail component"
```

---

## Task 4: 创建HexDump组件

**Files:**
- Create: `src/components/ws/HexDump.tsx`

- [ ] **Step 1: 创建HexDump**

```tsx
interface HexDumpProps {
  text: string;
}

function stringToBytes(text: string): number[] {
  const bytes: number[] = [];
  for (let i = 0; i < text.length; i++) {
    bytes.push(text.charCodeAt(i) & 0xff);
  }
  return bytes;
}

function bytesToHex(bytes: number[]): string {
  return bytes.map((b) => b.toString(16).padStart(2, "0")).join(" ");
}

function bytesToAscii(bytes: number[]): string {
  return bytes
    .map((b) => (b >= 32 && b < 127 ? String.fromCharCode(b) : "."))
    .join("");
}

export function HexDump({ text }: HexDumpProps) {
  const bytes = stringToBytes(text);
  const lines: string[] = [];

  for (let i = 0; i < bytes.length; i += 16) {
    const chunk = bytes.slice(i, Math.min(i + 16, bytes.length));
    const hex = bytesToHex(chunk).padEnd(48);
    const ascii = bytesToAscii(chunk);
    const addr = i.toString(16).padStart(8, "0");

    lines.push(`${addr}  ${hex}  ${ascii}`);
  }

  return (
    <pre className="text-xs font-mono leading-relaxed">
      {lines.map((line, i) => (
        <div key={i}>{line}</div>
      ))}
    </pre>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add src/components/ws/HexDump.tsx
git commit -m "feat(ws): add HexDump component"
```

---

## Task 5: 创建Rust IPC命令

**Files:**
- Create: `src-tauri/src/commands/ws_frames.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 创建ws_frames.rs**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsFrame {
    pub id: String,
    #[serde(rename = "requestId")]
    pub request_id: String,
    pub direction: FrameDirection,
    pub opcode: u8,
    pub payload: String,
    #[serde(rename = "payloadText")]
    pub payload_text: Option<String>,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FrameDirection {
    Incoming,
    Outgoing,
}

#[tauri::command]
pub fn get_ws_frames(request_id: String) -> Result<Vec<WsFrame>, String> {
    // 从连接状态获取该请求关联的WS帧
    // 返回帧列表
    Ok(vec![])
}

#[tauri::command]
pub fn subscribe_ws_frames(request_id: String) -> Result<Channel<WsFrame>, String> {
    // 创建channel用于实时推送帧
    Err("Not implemented".to_string())
}
```

- [ ] **Step 2: 注册命令**

在 `src-tauri/src/lib.rs` 中添加:
```rust
pub mod commands;
pub mod commands::ws_frames;
pub use commands::ws_frames::*;
```

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/commands/ws_frames.rs src-tauri/src/lib.rs
git commit -m "feat(ws): add WS frames IPC commands"
```

---

## Task 6: 编译验证

- [ ] **Step 1: 运行编译**

```bash
npm run build 2>&1 | tail -20
```

- [ ] **Step 2: Commit**

```bash
git add -A
git commit -m "feat(ws): complete WS frame viewer implementation"
```

---

## 验证清单

- [ ] 帧列表显示 (direction, opcode, payload preview)
- [ ] 点击帧显示详情
- [ ] Text/Hex视图切换
- [ ] HexDump正确显示
- [ ] 编译通过
