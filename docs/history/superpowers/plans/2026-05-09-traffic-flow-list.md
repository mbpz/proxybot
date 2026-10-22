# Traffic Flow List Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现实时流量列表页面，支持筛选、搜索、详情查看，60/40分屏布局

**Architecture:** TanStack Table实现虚拟列表，60/40分屏显示列表和详情，数据通过Tauri IPC获取，支持实时更新

**Tech Stack:** React, @tanstack/react-table@8.11, @tanstack/react-virtual@3.0, TypeScript

---

## File Structure

| 操作 | 文件 | 职责 |
|------|------|------|
| Create | `src/components/traffic/TrafficPage.tsx` | 主容器 |
| Create | `src/components/traffic/FilterBar.tsx` | 筛选栏 |
| Create | `src/components/traffic/RequestTable.tsx` | 虚拟列表表格 |
| Create | `src/components/traffic/RequestDetail.tsx` | 详情面板 |
| Create | `src/components/traffic/HeadersView.tsx` | Headers查看 |
| Create | `src/components/traffic/BodyView.tsx` | Body查看 |
| Create | `src/components/traffic/WsFramesView.tsx` | WS帧查看 |
| Modify | `src/main.tsx` | 添加路由 |
| Create | `src-tauri/src/commands/traffic.rs` | Rust IPC |

---

## Task 1: 添加前端依赖

**Files:**
- Modify: `package.json`

- [ ] **Step 1: 添加依赖**

Run: `cd /Users/doug/ai/system/proxybot && npm install @tanstack/react-table @tanstack/react-virtual`

- [ ] **Step 2: 验证安装**

```bash
cat package.json | grep -E "@tanstack/react-table|@tanstack/react-virtual"
```

- [ ] **Step 3: Commit**

```bash
git add package.json package-lock.json
git commit -m "feat(traffic): add TanStack Table and Virtual for request list"
```

---

## Task 2: 创建TrafficPage主容器

**Files:**
- Create: `src/components/traffic/TrafficPage.tsx`

- [ ] **Step 1: 创建TrafficPage**

```tsx
import { useState, useEffect, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { FilterBar } from "./FilterBar";
import { RequestTable } from "./RequestTable";
import { RequestDetail } from "./RequestDetail";

interface InterceptedRequest {
  id: string;
  method: string;
  host: string;
  path: string;
  status?: number;
  duration_ms: number;
  timestamp: number;
  app_tag?: string;
  headers: Record<string, string>;
  body?: string;
}

interface FilterState {
  method?: string;
  host?: string;
  status?: number;
  appTag?: string;
  search?: string;
}

export function TrafficPage() {
  const [requests, setRequests] = useState<InterceptedRequest[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [filters, setFilters] = useState<FilterState>({});

  useEffect(() => {
    loadRequests();

    // Subscribe to real-time updates
    const unlisten = listen<InterceptedRequest>("traffic-update", (event) => {
      setRequests((prev) => [event.payload, ...prev.slice(0, 999)]);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  async function loadRequests() {
    try {
      const result = await invoke<InterceptedRequest[]>("get_requests", {
        filter: {},
        limit: 1000,
      });
      setRequests(result);
    } catch (err) {
      console.error("Failed to load requests:", err);
    }
  }

  const filteredRequests = useMemo(() => {
    let result = requests;

    if (filters.method) {
      result = result.filter((r) => r.method === filters.method);
    }
    if (filters.host) {
      const pattern = filters.host.replace(/\*/g, ".*");
      result = result.filter((r) => new RegExp(pattern).test(r.host));
    }
    if (filters.status) {
      result = result.filter((r) => r.status === filters.status);
    }
    if (filters.search) {
      const search = filters.search.toLowerCase();
      result = result.filter(
        (r) =>
          r.path.toLowerCase().includes(search) ||
          r.host.toLowerCase().includes(search)
      );
    }

    return result;
  }, [requests, filters]);

  const selectedRequest = useMemo(
    () => requests.find((r) => r.id === selectedId),
    [requests, selectedId]
  );

  return (
    <div className="flex flex-col h-screen">
      <FilterBar filters={filters} onChange={setFilters} />

      <div className="flex flex-1 overflow-hidden">
        <div className="w-3/5 border-r overflow-hidden">
          <RequestTable
            requests={filteredRequests}
            selectedId={selectedId}
            onSelect={setSelectedId}
          />
        </div>
        <div className="w-2/5 overflow-hidden">
          {selectedRequest ? (
            <RequestDetail request={selectedRequest} />
          ) : (
            <div className="flex items-center justify-center h-full text-gray-500">
              Select a request to view details
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add src/components/traffic/TrafficPage.tsx
git commit -m "feat(traffic): add TrafficPage container"
```

---

## Task 3: 创建FilterBar组件

**Files:**
- Create: `src/components/traffic/FilterBar.tsx`

- [ ] **Step 1: 创建FilterBar**

```tsx
interface FilterState {
  method?: string;
  host?: string;
  status?: number;
  appTag?: string;
  search?: string;
}

interface FilterBarProps {
  filters: FilterState;
  onChange: (filters: FilterState) => void;
}

export function FilterBar({ filters, onChange }: FilterBarProps) {
  return (
    <div className="flex gap-2 p-2 bg-gray-100 border-b">
      <select
        value={filters.method || ""}
        onChange={(e) => onChange({ ...filters, method: e.target.value || undefined })}
        className="px-2 py-1 border rounded"
      >
        <option value="">All Methods</option>
        <option value="GET">GET</option>
        <option value="POST">POST</option>
        <option value="PUT">PUT</option>
        <option value="DELETE">DELETE</option>
        <option value="PATCH">PATCH</option>
      </select>

      <input
        type="text"
        placeholder="host:*.example.com"
        value={filters.host || ""}
        onChange={(e) => onChange({ ...filters, host: e.target.value || undefined })}
        className="px-2 py-1 border rounded flex-1"
      />

      <input
        type="text"
        placeholder="Search..."
        value={filters.search || ""}
        onChange={(e) => onChange({ ...filters, search: e.target.value || undefined })}
        className="px-2 py-1 border rounded flex-1"
      />

      <button
        onClick={() => onChange({})}
        className="px-3 py-1 bg-gray-200 rounded hover:bg-gray-300"
      >
        Clear
      </button>
    </div>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add src/components/traffic/FilterBar.tsx
git commit -m "feat(traffic): add FilterBar component"
```

---

## Task 4: 创建RequestTable组件

**Files:**
- Create: `src/components/traffic/RequestTable.tsx`

- [ ] **Step 1: 创建RequestTable**

```tsx
import { useRef } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";

interface InterceptedRequest {
  id: string;
  method: string;
  host: string;
  path: string;
  status?: number;
  duration_ms: number;
  timestamp: number;
  app_tag?: string;
}

interface RequestTableProps {
  requests: InterceptedRequest[];
  selectedId: string | null;
  onSelect: (id: string) => void;
}

function getStatusColor(status?: number): string {
  if (!status) return "text-gray-500";
  if (status >= 200 && status < 300) return "text-green-600";
  if (status >= 400 && status < 500) return "text-orange-600";
  if (status >= 500) return "text-red-600";
  return "text-gray-600";
}

function formatTime(timestamp: number): string {
  const d = new Date(timestamp * 1000);
  return `${d.getHours().toString().padStart(2, "0")}:${d.getMinutes().toString().padStart(2, "0")}:${d.getSeconds().toString().padStart(2, "0")}`;
}

export function RequestTable({ requests, selectedId, onSelect }: RequestTableProps) {
  const parentRef = useRef<HTMLDivElement>(null);

  const rowVirtualizer = useVirtualizer({
    count: requests.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 48,
  });

  if (requests.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500">
        No requests captured yet
      </div>
    );
  }

  return (
    <div ref={parentRef} className="h-full overflow-auto">
      <div
        style={{
          height: `${rowVirtualizer.getTotalSize()}px`,
          width: "100%",
          position: "relative",
        }}
      >
        {rowVirtualizer.getVirtualItems().map((virtualRow) => {
          const req = requests[virtualRow.index];
          return (
            <div
              key={req.id}
              onClick={() => onSelect(req.id)}
              className={`absolute top-0 left-0 w-full flex items-center px-4 border-b cursor-pointer hover:bg-gray-50 ${
                req.id === selectedId ? "bg-blue-100" : ""
              }`}
              style={{
                height: `${virtualRow.size}px`,
                transform: `translateY(${virtualRow.start}px)`,
              }}
            >
              <span className="w-16 text-sm font-mono">{req.method}</span>
              <span className="flex-1 truncate text-sm">{req.path}</span>
              <span className={`w-16 text-sm ${getStatusColor(req.status)}`}>
                {req.status || ".."}
              </span>
              <span className="w-20 text-right text-sm text-gray-500">
                {req.duration_ms}ms
              </span>
              <span className="w-20 text-right text-xs text-gray-400">
                {formatTime(req.timestamp)}
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add src/components/traffic/RequestTable.tsx
git commit -m "feat(traffic): add RequestTable with virtual scrolling"
```

---

## Task 5: 创建RequestDetail组件

**Files:**
- Create: `src/components/traffic/RequestDetail.tsx`

- [ ] **Step 1: 创建RequestDetail**

```tsx
import { useState } from "react";
import { HeadersView } from "./HeadersView";
import { BodyView } from "./BodyView";
import { WsFramesView } from "./WsFramesView";

interface InterceptedRequest {
  id: string;
  method: string;
  host: string;
  path: string;
  status?: number;
  duration_ms: number;
  headers: Record<string, string>;
  body?: string;
}

interface RequestDetailProps {
  request: InterceptedRequest;
}

type TabType = "headers" | "body" | "ws";

export function RequestDetail({ request }: RequestDetailProps) {
  const [activeTab, setActiveTab] = useState<TabType>("headers");

  const tabs: { key: TabType; label: string }[] = [
    { key: "headers", label: "Headers" },
    { key: "body", label: "Body" },
    { key: "ws", label: "WS Frames" },
  ];

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="p-4 border-b bg-gray-50">
        <div className="text-sm text-gray-500">
          {request.method} {request.host}{request.path}
        </div>
        <div className="text-sm mt-1">
          Status: <span className={request.status && request.status >= 400 ? "text-red-600" : "text-green-600"}>
            {request.status || ".."}
          </span>
          {" | "}
          Duration: {request.duration_ms}ms
        </div>
      </div>

      {/* Tabs */}
      <div className="flex border-b">
        {tabs.map((tab) => (
          <button
            key={tab.key}
            onClick={() => setActiveTab(tab.key)}
            className={`px-4 py-2 text-sm ${
              activeTab === tab.key
                ? "border-b-2 border-blue-500 text-blue-600"
                : "text-gray-600"
            }`}
          >
            {tab.label}
          </button>
        ))}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-auto">
        {activeTab === "headers" && <HeadersView headers={request.headers} />}
        {activeTab === "body" && <BodyView body={request.body} />}
        {activeTab === "ws" && <WsFramesView requestId={request.id} />}
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add src/components/traffic/RequestDetail.tsx
git commit -m "feat(traffic): add RequestDetail component"
```

---

## Task 6: 创建HeadersView和BodyView

**Files:**
- Create: `src/components/traffic/HeadersView.tsx`
- Create: `src/components/traffic/BodyView.tsx`

- [ ] **Step 1: 创建HeadersView**

```tsx
interface HeadersViewProps {
  headers: Record<string, string>;
}

export function HeadersView({ headers }: HeadersViewProps) {
  return (
    <div className="p-4">
      <table className="w-full text-sm">
        <tbody>
          {Object.entries(headers).map(([key, value]) => (
            <tr key={key} className="border-b">
              <td className="font-mono text-gray-600 pr-4 py-1 w-1/3">{key}</td>
              <td className="font-mono py-1 break-all">{value}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
```

- [ ] **Step 2: 创建BodyView**

```tsx
interface BodyViewProps {
  body?: string;
}

export function BodyView({ body }: BodyViewProps) {
  if (!body) {
    return <div className="p-4 text-gray-500">No body content</div>;
  }

  let formattedBody = body;
  try {
    const parsed = JSON.parse(body);
    formattedBody = JSON.stringify(parsed, null, 2);
  } catch {
    // Not JSON, use as-is
  }

  return (
    <pre className="p-4 text-sm font-mono overflow-auto whitespace-pre-wrap">
      {formattedBody}
    </pre>
  );
}
```

- [ ] **Step 3: Commit**

```bash
git add src/components/traffic/HeadersView.tsx src/components/traffic/BodyView.tsx
git commit -m "feat(traffic): add HeadersView and BodyView"
```

---

## Task 7: 创建WsFramesView

**Files:**
- Create: `src/components/traffic/WsFramesView.tsx`

- [ ] **Step 1: 创建WsFramesView**

```tsx
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

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

export function WsFramesView({ requestId }: WsFramesViewProps) {
  const [frames, setFrames] = useState<WsFrame[]>([]);
  const [selectedFrame, setSelectedFrame] = useState<WsFrame | null>(null);

  useEffect(() => {
    loadFrames();
  }, [requestId]);

  async function loadFrames() {
    try {
      const result = await invoke<WsFrame[]>("get_ws_frames", { requestId });
      setFrames(result);
    } catch (err) {
      console.error("Failed to load WS frames:", err);
    }
  }

  return (
    <div className="flex h-full">
      <div className="w-1/2 border-r overflow-auto">
        {frames.length === 0 ? (
          <div className="p-4 text-gray-500">No WebSocket frames</div>
        ) : (
          frames.map((frame) => (
            <div
              key={frame.id}
              onClick={() => setSelectedFrame(frame)}
              className={`flex items-center px-3 py-2 border-b cursor-pointer ${
                frame.direction === "incoming" ? "text-green-600" : "text-blue-600"
              } ${selectedFrame?.id === frame.id ? "bg-gray-100" : ""}`}
            >
              <span className="w-4">{frame.direction === "incoming" ? "←" : "→"}</span>
              <span className="w-12 font-mono text-xs">{getOpcodeName(frame.opcode)}</span>
              <span className="flex-1 truncate text-sm">{frame.payload.slice(0, 30)}</span>
            </div>
          ))
        )}
      </div>
      <div className="w-1/2 p-4 overflow-auto">
        {selectedFrame ? (
          <pre className="text-sm font-mono whitespace-pre-wrap">{selectedFrame.payload}</pre>
        ) : (
          <div className="text-gray-500">Select a frame to view</div>
        )}
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add src/components/traffic/WsFramesView.tsx
git commit -m "feat(traffic): add WsFramesView component"
```

---

## Task 8: 配置路由

**Files:**
- Modify: `src/main.tsx`

- [ ] **Step 1: 添加路由**

```tsx
import { TrafficPage } from "./components/traffic/TrafficPage";

// 在Routes中添加
<Route path="/" element={<TrafficPage />} />
```

- [ ] **Step 2: Commit**

```bash
git add src/main.tsx
git commit -m "feat(traffic): add / route for TrafficPage"
```

---

## Task 9: 编译验证

- [ ] **Step 1: 运行编译**

```bash
npm run build 2>&1 | tail -30
```

- [ ] **Step 2: Commit**

```bash
git add -A
git commit -m "feat(traffic): complete Traffic page implementation"
```

---

## 验证清单

- [ ] 请求列表显示
- [ ] 虚拟滚动支持大数据量
- [ ] 筛选功能正常
- [ ] 详情面板切换
- [ ] Headers/Body/WS tabs
- [ ] 编译通过
