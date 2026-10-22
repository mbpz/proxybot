# Graph Visualization Upgrade v0.9.0 设计方案

## Status: Draft

## 1. Overview

将TUI的ASCII DAG升级为Web可视化组件，支持交互式请求依赖图、Auth状态机可视化、时间线瀑布流。

**当前问题：**
- 仅ASCII字符，DAG信息密度低
- 无法点击查看详情
- Auth状态机仅文本格式

**目标：**
- Web可视化，交互式
- 请求瀑布流 + 时序图
- Auth状态机Mermaid渲染

---

## 2. 竞品分析

| 竞品 | 可视化方案 |
|------|-----------|
| Proxyman | 完整Timeline视图，请求瀑布流 |
| mitmproxy | mitmweb提供请求timeline |
| HTTP Toolkit | D3.js请求依赖图 |
| Charles | 整体结构/序列图 |

---

## 3. 技术方案

### 3.1 架构

```
┌─────────────────────────────────────────────────┐
│                 React Frontend                   │
│  ┌─────────────────────────────────────────────┐│
│  │         GraphPage.tsx                         ││
│  │  - Recharts (timeline)                       ││
│  │  - Mermaid (state diagram)                   ││
│  │  - vis-network (dependency graph)           ││
│  └─────────────────────────────────────────────┘│
│                       │ IPC                      │
└───────────────────────┼─────────────────────────┘
                        │
┌───────────────────────┼─────────────────────────┐
│                 Rust Core                        │
│  ┌──────────────┐ ┌──────────────────────────┐ │
│  │ traffic.rs   │ │ /api/graph/requests      │ │
│  │ 提供数据     │ │ 返回结构化请求图数据     │ │
│  └──────────────┘ └──────────────────────────┘ │
└─────────────────────────────────────────────────┘
```

### 3.2 三种视图

**View A: 请求瀑布流 (Waterfall)**
- X轴: 时间
- Y轴: 请求
- 颜色: 状态码 (2xx绿, 4xx橙, 5xx红)
- 支持: 缩放、筛选、点击详情

**View B: 依赖图 (DAG)**
- 节点: 请求 (host + path pattern)
- 边: 请求顺序/依赖
- 使用 vis-network 实现
- 可拖拽、缩放、点击节点

**View C: Auth状态机**
- Mermaid格式渲染
- 检测login/token/logout流程
- 显示状态转换

---

## 4. 组件设计

### 4.1 GraphPage.tsx

```tsx
interface GraphData {
  requests: RequestNode[];
  edges: Edge[];
}

interface RequestNode {
  id: string;
  host: string;
  path: string;
  method: string;
  status?: number;
  duration: number;
  timestamp: number;
  parentId?: string;
}

export function GraphPage() {
  const [view, setView] = useState<'waterfall' | 'dag' | 'auth'>('waterfall');
  const [data, setData] = useState<GraphData | null>(null);

  return (
    <div className="p-6">
      {/* View Selector */}
      <div className="flex gap-2 mb-4">
        <button onClick={() => setView('waterfall')}>Waterfall</button>
        <button onClick={() => setView('dag')}>Dependency</button>
        <button onClick={() => setView('auth')}>Auth Flow</button>
      </div>

      {/* Content */}
      <div className="bg-white rounded-lg shadow" style={{ height: '600px' }}>
        {view === 'waterfall' && <WaterfallChart data={data} />}
        {view === 'dag' && <DependencyGraph data={data} />}
        {view === 'auth' && <AuthStateMachine />}
      </div>
    </div>
  );
}
```

### 4.2 WaterfallChart.tsx

使用 Recharts:
```tsx
function WaterfallChart({ data }: { data: GraphData }) {
  return (
    <ResponsiveContainer width="100%" height="100%">
      <BarChart data={data?.requests}>
        <XAxis dataKey="timestamp" tickFormatter={formatTime} />
        <YAxis dataKey="path" />
        <Tooltip />
        <Bar dataKey="duration" fill={getStatusColor} />
      </BarChart>
    </ResponsiveContainer>
  );
}
```

### 4.3 DependencyGraph.tsx

使用 vis-network:
```tsx
function DependencyGraph({ data }: { data: GraphData }) {
  const nodes = data?.requests.map(r => ({
    id: r.id,
    label: `${r.method} ${truncate(r.path, 20)}`,
    color: getStatusColor(r.status),
  })) || [];

  const edges = data?.edges.map(e => ({
    from: e.from,
    to: e.to,
  })) || [];

  return (
    <Network
      nodes={nodes}
      edges={edges}
      options={{ physics: { enabled: true } }}
    />
  );
}
```

### 4.4 AuthStateMachine.tsx

使用 mermaid:
```tsx
import mermaid from 'mermaid';

function AuthStateMachine() {
  const [diagram, setDiagram] = useState('');

  useEffect(() => {
    mermaid.render('auth-graph', diagram);
  }, [diagram]);

  return <div className="mermaid" ref={containerRef} />;
}
```

---

## 5. Rust IPC 命令

### 5.1 新增命令

```rust
#[tauri::command]
fn get_graph_data(max_requests: usize) -> Result<GraphData, String> {
    // 从 traffic state 获取最近请求
    // 构建节点和边
    // 返回结构化数据
}

#[tauri::command]
fn get_auth_flow() -> Result<AuthFlowData, String> {
    // 分析请求序列
    // 检测 auth 相关 endpoints
    // 返回 Mermaid 格式
}
```

### 5.2 数据结构

```rust
struct GraphData {
    requests: Vec<RequestNode>,
    edges: Vec<Edge>,
}

struct RequestNode {
    id: String,
    host: String,
    path: String,
    method: String,
    status: Option<u16>,
    duration_ms: u64,
    timestamp: i64,
    parent_id: Option<String>,
}

struct Edge {
    from: String,
    to: String,
    edge_type: String, // "temporal" | "dependency"
}
```

---

## 6. 依赖

```json
{
  "recharts": "^2.10.0",
  "vis-network": "^9.6.0",
  "mermaid": "^10.0.0"
}
```

---

## 7. 实施步骤

### Phase 1: 基础结构
1. 添加依赖包
2. 创建 GraphPage 框架
3. 实现 vis-network DAG 视图

### Phase 2: Waterfall
4. 实现 WaterfallChart
5. 绑定 Rust IPC

### Phase 3: Auth
6. 实现 Mermaid Auth 状态机
7. 完善交互

---

## 8. 验证

```bash
# 编译
npm run build
cargo build --bin proxybot-gui

# 启动GUI
./target/release/proxybot-gui

# 访问 /graph
# 切换三个视图
# 验证交互
```
