# Traffic Flow List v0.9.0 设计方案

## Status: Draft

## 1. Overview

实现GUI流量列表页面，显示实时HTTP/HTTPS请求，支持筛选、搜索、详情查看。

**当前问题：**
- TUI仅文本列表，信息密度低
- 无图形化详情面板
- 无实时更新

**目标：**
- 实时请求列表
- 多维度筛选
- 请求详情面板

---

## 2. 竞品分析

| 竞品 | 流量列表特点 |
|------|-------------|
| Proxyman | TanStack Table，支持列排序、多选 |
| mitmproxy | 虚拟列表，支持10k+请求 |
| HTTP Toolkit | 实时流，支持 WebSocket |

---

## 3. 技术方案

### 3.1 架构

```
┌─────────────────────────────────────────────────┐
│                 React Frontend                   │
│  ┌─────────────────────────────────────────────┐│
│  │         TrafficPage.tsx                      ││
│  │  - TanStack Table (虚拟列表)                 ││
│  │  - 60/40 split (列表/详情)                  ││
│  │  - 实时更新 via Tauri IPC事件               ││
│  └─────────────────────────────────────────────┘│
└─────────────────────────────────────────────────┘
```

### 3.2 布局

```
┌────────────────────────────────────────────────────────┐
│ [Filter Bar] method:GET host:*.example.com status:200  │
├────────────────────────────────────────────────────────┤
│ Request List              │ Request Detail            │
│ ┌────────────────────────┐ │ ┌────────────────────────┐│
│ │ GET /api/users 200 45ms│ │ │ Headers                ││
│ │ POST /api/login 401 .. │ │ │ Body                   ││
│ │ GET /api/items 200 ..  │ │ │ WS Frames (if any)     ││
│ │ ...                    │ │ │                        ││
│ └────────────────────────┘ │ └────────────────────────┘│
└────────────────────────────────────────────────────────┘
```

---

## 4. 组件设计

### 4.1 TrafficPage.tsx

```tsx
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

export function TrafficPage() {
  const [requests, setRequests] = useState<InterceptedRequest[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [filters, setFilters] = useState<FilterState>({});

  // Subscribe to real-time updates via IPC event

  return (
    <div className="flex h-screen">
      {/* Filter Bar */}
      <FilterBar filters={filters} onChange={setFilters} />

      {/* Split Pane */}
      <div className="flex flex-1">
        {/* Request List */}
        <div className="w-3/5 border-r">
          <RequestTable
            requests={filteredRequests}
            selectedId={selectedId}
            onSelect={setSelectedId}
          />
        </div>

        {/* Detail Panel */}
        <div className="w-2/5">
          {selectedId && (
            <RequestDetail request={getRequest(selectedId)} />
          )}
        </div>
      </div>
    </div>
  );
}
```

### 4.2 FilterBar.tsx

```tsx
interface FilterState {
  method?: string;
  host?: string;
  status?: number;
  appTag?: string;
  search?: string;
}

function FilterBar({ filters, onChange }: FilterBarProps) {
  return (
    <div className="flex gap-2 p-2 bg-gray-100">
      <select value={filters.method || ''} onChange={e => onChange({...filters, method: e.target.value})}>
        <option value="">All Methods</option>
        <option value="GET">GET</option>
        <option value="POST">POST</option>
        <option value="PUT">PUT</option>
        <option value="DELETE">DELETE</option>
      </select>

      <input
        type="text"
        placeholder="host:*.example.com"
        value={filters.host || ''}
        onChange={e => onChange({...filters, host: e.target.value})}
      />

      <input
        type="text"
        placeholder="Search..."
        value={filters.search || ''}
        onChange={e => onChange({...filters, search: e.target.value})}
      />
    </div>
  );
}
```

### 4.3 RequestTable.tsx

使用 TanStack Table + 虚拟滚动:
```tsx
import { useReactTable, getCoreRowModel, flexRender } from '@tanstack/react-table';
import { useVirtualizer } from '@tanstack/react-virtual';

function RequestTable({ requests, selectedId, onSelect }: RequestTableProps) {
  const table = useReactTable({
    data: requests,
    columns,
    getCoreRowModel: getCoreRowModel(),
  });

  const { rows } = table.getRowModel();
  const rowVirtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 48,
  });

  return (
    <div ref={parentRef} className="h-full overflow-auto">
      {rowVirtualizer.getVirtualItems().map(virtualRow => {
        const row = rows[virtualRow.index];
        return (
          <div
            key={row.id}
            className={`flex items-center px-4 border-b cursor-pointer ${
              row.original.id === selectedId ? 'bg-blue-100' : ''
            }`}
            onClick={() => onSelect(row.original.id)}
          >
            {/* Columns */}
            <div className="w-16">{row.original.method}</div>
            <div className="flex-1 truncate">{row.original.path}</div>
            <div className="w-16">{row.original.status}</div>
            <div className="w-20 text-right">{row.original.duration_ms}ms</div>
          </div>
        );
      })}
    </div>
  );
}
```

### 4.4 RequestDetail.tsx

```tsx
function RequestDetail({ request }: { request: InterceptedRequest }) {
  const [activeTab, setActiveTab] = useState<'headers' | 'body' | 'ws'>('headers');

  return (
    <div className="h-full flex flex-col">
      {/* Tabs */}
      <div className="flex border-b">
        <button onClick={() => setActiveTab('headers')} className={activeTab === 'headers' ? 'border-b-2 border-blue-500' : ''}>
          Headers
        </button>
        <button onClick={() => setActiveTab('body')}>
          Body
        </button>
        <button onClick={() => setActiveTab('ws')}>
          WS Frames
        </button>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-auto p-4">
        {activeTab === 'headers' && <HeadersView headers={request.headers} />}
        {activeTab === 'body' && <BodyView body={request.body} />}
        {activeTab === 'ws' && <WsFramesView requestId={request.id} />}
      </div>
    </div>
  );
}
```

---

## 5. Rust IPC 命令

### 5.1 现有命令

```rust
#[tauri::command]
fn get_requests(filter: RequestFilter, limit: usize) -> Result<Vec<InterceptedRequest>, String>;

#[tauri::command]
fn get_request_detail(id: String) -> Result<RequestDetail, String>;
```

### 5.2 实时更新

通过 Tauri Event 系统:
```rust
// 在 proxy.rs 中
app.emit("traffic-update", request).unwrap();
```

```tsx
// 在 React 中
useEventListener('traffic-update', (request) => {
  setRequests(prev => [request, ...prev.slice(0, 999)]);
});
```

---

## 6. 依赖

```json
{
  "@tanstack/react-table": "^8.11.0",
  "@tanstack/react-virtual": "^3.0.0"
}
```

---

## 7. 验证

```bash
# 启动代理，验证请求列表实时更新
# 测试筛选功能
# 测试详情面板切换
```
