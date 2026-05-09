# Graph Visualization Upgrade Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现Web可视化Graph页面，支持Waterfall/DAG/Auth三种视图切换

**Architecture:** React组件使用vis-network渲染依赖图，recharts渲染时间线，Mermaid渲染状态机。数据通过Tauri IPC从Rust获取。

**Tech Stack:** React, vis-network@9.6, recharts@2.10, mermaid@10, TypeScript

---

## File Structure

| 操作 | 文件 | 职责 |
|------|------|------|
| Create | `src/components/graph/GraphPage.tsx` | 主容器，三视图切换 |
| Create | `src/components/graph/WaterfallChart.tsx` | Recharts时间线 |
| Create | `src/components/graph/DependencyGraph.tsx` | vis-network依赖图 |
| Create | `src/components/graph/AuthStateMachine.tsx` | Mermaid状态机 |
| Create | `src-tauri/src/commands/graph.rs` | Rust IPC命令 |
| Modify | `src-tauri/src/lib.rs` | 注册graph命令 |
| Modify | `src/main.tsx` | 添加/graph路由 |

---

## Task 1: 添加前端依赖

**Files:**
- Modify: `package.json`

- [ ] **Step 1: 添加依赖**

Run: `cd /Users/doug/ai/system/proxybot && npm install vis-network recharts mermaid`

- [ ] **Step 2: 验证安装**

```bash
cat package.json | grep -E "vis-network|recharts|mermaid"
```

Expected: 三项都在package.json中

- [ ] **Step 3: Commit**

```bash
git add package.json package-lock.json
git commit -m "feat(graph): add vis-network, recharts, mermaid for visualization"
```

---

## Task 2: 创建GraphPage主容器

**Files:**
- Create: `src/components/graph/GraphPage.tsx`

- [ ] **Step 1: 创建GraphPage组件**

```tsx
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { WaterfallChart } from "./WaterfallChart";
import { DependencyGraph } from "./DependencyGraph";
import { AuthStateMachine } from "./AuthStateMachine";

type ViewType = "waterfall" | "dag" | "auth";

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
  duration_ms: number;
  timestamp: number;
  parentId?: string;
}

interface Edge {
  from: string;
  to: string;
}

export function GraphPage() {
  const [view, setView] = useState<ViewType>("waterfall");
  const [data, setData] = useState<GraphData | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    loadGraphData();
  }, []);

  async function loadGraphData() {
    try {
      setLoading(true);
      const result = await invoke<GraphData>("get_graph_data", { maxRequests: 100 });
      setData(result);
    } catch (err) {
      console.error("Failed to load graph data:", err);
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="p-6 h-screen flex flex-col">
      {/* View Selector */}
      <div className="flex gap-2 mb-4">
        <button
          onClick={() => setView("waterfall")}
          className={`px-4 py-2 rounded ${
            view === "waterfall" ? "bg-blue-600 text-white" : "bg-gray-200"
          }`}
        >
          Waterfall
        </button>
        <button
          onClick={() => setView("dag")}
          className={`px-4 py-2 rounded ${
            view === "dag" ? "bg-blue-600 text-white" : "bg-gray-200"
          }`}
        >
          Dependency
        </button>
        <button
          onClick={() => setView("auth")}
          className={`px-4 py-2 rounded ${
            view === "auth" ? "bg-blue-600 text-white" : "bg-gray-200"
          }`}
        >
          Auth Flow
        </button>
      </div>

      {/* Content */}
      <div className="flex-1 bg-white rounded-lg shadow overflow-hidden">
        {loading ? (
          <div className="flex items-center justify-center h-full">Loading...</div>
        ) : (
          <>
            {view === "waterfall" && <WaterfallChart data={data} />}
            {view === "dag" && <DependencyGraph data={data} />}
            {view === "auth" && <AuthStateMachine />}
          </>
        )}
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add src/components/graph/GraphPage.tsx
git commit -m "feat(graph): add GraphPage with view selector"
```

---

## Task 3: 创建WaterfallChart组件

**Files:**
- Create: `src/components/graph/WaterfallChart.tsx`

- [ ] **Step 1: 创建WaterfallChart**

```tsx
import { useMemo } from "react";
import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
} from "recharts";

interface RequestNode {
  id: string;
  host: string;
  path: string;
  method: string;
  status?: number;
  duration_ms: number;
  timestamp: number;
}

interface GraphData {
  requests: RequestNode[];
  edges: any[];
}

interface WaterfallChartProps {
  data: GraphData | null;
}

function getStatusColor(status?: number): string {
  if (!status) return "#6b7280";
  if (status >= 200 && status < 300) return "#10b981";
  if (status >= 300 && status < 400) return "#3b82f6";
  if (status >= 400 && status < 500) return "#f59e0b";
  if (status >= 500) return "#ef4444";
  return "#6b7280";
}

function formatTime(timestamp: number): string {
  const d = new Date(timestamp * 1000);
  return `${d.getHours().toString().padStart(2, "0")}:${d.getMinutes().toString().padStart(2, "0")}:${d.getSeconds().toString().padStart(2, "0")}`;
}

export function WaterfallChart({ data }: WaterfallChartProps) {
  const chartData = useMemo(() => {
    if (!data?.requests) return [];
    return data.requests.slice(0, 50).map((req) => ({
      id: req.id,
      name: `${req.method} ${req.path.slice(0, 20)}`,
      duration: req.duration_ms,
      timestamp: req.timestamp,
      status: req.status,
      color: getStatusColor(req.status),
    }));
  }, [data]);

  if (!data?.requests?.length) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500">
        No request data available
      </div>
    );
  }

  return (
    <div className="w-full h-full p-4">
      <ResponsiveContainer width="100%" height="100%">
        <BarChart data={chartData} layout="vertical">
          <XAxis type="number" label="Duration (ms)" />
          <YAxis
            type="category"
            dataKey="name"
            width={150}
            fontSize={10}
          />
          <Tooltip
            formatter={(value, name, props) => [
              `${props.payload.duration}ms`,
              "Duration",
            ]}
            labelFormatter={(label, payload) => {
              if (payload?.[0]) {
                return `${payload[0].payload.name}`;
              }
              return label;
            }}
          />
          <Bar dataKey="duration" fill="#3b82f6" />
        </BarChart>
      </ResponsiveContainer>
    </div>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add src/components/graph/WaterfallChart.tsx
git commit -m "feat(graph): add WaterfallChart with recharts"
```

---

## Task 4: 创建DependencyGraph组件

**Files:**
- Create: `src/components/graph/DependencyGraph.tsx`

- [ ] **Step 1: 创建DependencyGraph**

```tsx
import { useEffect, useRef, useMemo } from "react";
import { Network, DataSet } from "vis-network";

interface RequestNode {
  id: string;
  host: string;
  path: string;
  method: string;
  status?: number;
  duration_ms: number;
  timestamp: number;
}

interface Edge {
  from: string;
  to: string;
}

interface GraphData {
  requests: RequestNode[];
  edges: Edge[];
}

interface DependencyGraphProps {
  data: GraphData | null;
}

function getStatusColor(status?: number): string {
  if (!status) return "#6b7280";
  if (status >= 200 && status < 300) return "#10b981";
  if (status >= 400 && status < 500) return "#f59e0b";
  if (status >= 500) return "#ef4444";
  return "#3b82f6";
}

export function DependencyGraph({ data }: DependencyGraphProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const networkRef = useRef<Network | null>(null);

  const { nodes, edges } = useMemo(() => {
    if (!data?.requests) {
      return { nodes: new DataSet([]), edges: new DataSet([]) };
    }

    const nodesData = data.requests.slice(0, 30).map((req) => ({
      id: req.id,
      label: `${req.method} ${req.path.slice(0, 15)}`,
      color: getStatusColor(req.status),
      title: `${req.host}${req.path}\n${req.duration_ms}ms`,
    }));

    const edgesData = data.edges.slice(0, 50).map((e, i) => ({
      id: i,
      from: e.from,
      to: e.to,
    }));

    return {
      nodes: new DataSet(nodesData),
      edges: new DataSet(edgesData),
    };
  }, [data]);

  useEffect(() => {
    if (!containerRef.current) return;

    const options = {
      physics: {
        enabled: true,
        solver: "forceAtlas2Based",
        forceAtlas2Based: {
          theta: 0.5,
          gravitationalConstant: -50,
        },
      },
      edges: {
        arrows: "to",
        color: { color: "#94a3b8", highlight: "#3b82f6" },
      },
      nodes: {
        shape: "box",
        font: { size: 12 },
      },
    };

    networkRef.current = new Network(containerRef.current, { nodes, edges }, options);

    return () => {
      networkRef.current?.destroy();
    };
  }, [nodes, edges]);

  if (!data?.requests?.length) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500">
        No dependency data available
      </div>
    );
  }

  return <div ref={containerRef} className="w-full h-full" />;
}
```

- [ ] **Step 2: Commit**

```bash
git add src/components/graph/DependencyGraph.tsx
git commit -m "feat(graph): add DependencyGraph with vis-network"
```

---

## Task 5: 创建AuthStateMachine组件

**Files:**
- Create: `src/components/graph/AuthStateMachine.tsx`

- [ ] **Step 1: 创建AuthStateMachine**

```tsx
import { useEffect, useRef, useMemo } from "react";
import mermaid from "mermaid";

interface GraphData {
  requests: any[];
  edges: any[];
}

interface AuthStateMachineProps {
  data?: GraphData | null;
}

mermaid.initialize({
  startOnLoad: false,
  theme: "neutral",
});

const authKeywords = ["login", "auth", "token", "oauth", "signin", "password", "session"];

function buildMermaidDiagram(data?: GraphData | null): string {
  if (!data?.requests) {
    return "stateDiagram-v2\n  [*] --> NoAuthFlow\n  NoAuthFlow --> [*]";
  }

  const authStates: string[] = [];
  let currentState = "Initial";

  for (const req of data.requests.slice(0, 20)) {
    const combined = `${req.host} ${req.path}`.toLowerCase();
    const isAuth = authKeywords.some((kw) => combined.includes(kw));

    if (isAuth) {
      let newState = "Auth";
      if (combined.includes("login")) newState = "Login";
      else if (combined.includes("token")) newState = "Token";
      else if (combined.includes("logout")) newState = "Logout";

      if (!authStates.includes(newState) || authStates[authStates.length - 1] !== newState) {
        authStates.push(newState);
      }
      currentState = newState;
    } else if (currentState !== "API" && currentState !== "Initial") {
      if (authStates[authStates.length - 1] !== "API") {
        authStates.push("API");
      }
    }
  }

  if (authStates.length === 0) {
    return "stateDiagram-v2\n  [*] --> NoAuthFlow\n  NoAuthFlow --> [*]";
  }

  const transitions = authStates.map((state, i) => {
    if (i === 0) return `  [*] --> ${state}`;
    return `  ${authStates[i - 1]} --> ${state}`;
  });

  transitions.push(`  ${authStates[authStates.length - 1]} --> [*]`);

  return `stateDiagram-v2\n  ${transitions.join("\n  ")}`;
}

export function AuthStateMachine({ data }: AuthStateMachineProps) {
  const containerRef = useRef<HTMLDivElement>(null);

  const diagram = useMemo(() => buildMermaidDiagram(data), [data]);

  useEffect(() => {
    if (!containerRef.current) return;

    mermaid.render("auth-graph", diagram).then(({ svg }) => {
      if (containerRef.current) {
        containerRef.current.innerHTML = svg;
      }
    });

    return () => {
      if (containerRef.current) {
        containerRef.current.innerHTML = "";
      }
    };
  }, [diagram]);

  return (
    <div className="w-full h-full overflow-auto p-4">
      <div ref={containerRef} className="flex justify-center" />
    </div>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add src/components/graph/AuthStateMachine.tsx
git commit -m "feat(graph): add AuthStateMachine with mermaid"
```

---

## Task 6: 创建Rust IPC命令

**Files:**
- Create: `src-tauri/src/commands/graph.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 创建graph.rs**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestNode {
    pub id: String,
    pub host: String,
    pub path: String,
    pub method: String,
    pub status: Option<u16>,
    #[serde(rename = "duration_ms")]
    pub duration_ms: u64,
    pub timestamp: i64,
    #[serde(rename = "parentId")]
    pub parent_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphData {
    pub requests: Vec<RequestNode>,
    pub edges: Vec<Edge>,
}

#[tauri::command]
pub fn get_graph_data(max_requests: usize) -> Result<GraphData, String> {
    // 从 traffic state 获取请求
    // 构建节点和边
    // 返回 GraphData
    Ok(GraphData {
        requests: vec![],
        edges: vec![],
    })
}
```

- [ ] **Step 2: 注册命令到lib.rs**

在 `src-tauri/src/lib.rs` 中添加:
```rust
mod commands;
mod commands::graph;

pub use commands::graph::*;
```

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/commands/graph.rs src-tauri/src/lib.rs
git commit -m "feat(graph): add Rust IPC command for graph data"
```

---

## Task 7: 配置路由

**Files:**
- Modify: `src/main.tsx`

- [ ] **Step 1: 添加/graph路由**

```tsx
import { GraphPage } from "./components/graph/GraphPage";

// 在Routes中添加
<Route path="graph" element={<GraphPage />} />
```

- [ ] **Step 2: Commit**

```bash
git add src/main.tsx
git commit -m "feat(graph): add /graph route"
```

---

## Task 8: 编译验证

**Files:**
- Modify: `src/components/graph/GraphPage.tsx` (if needed)

- [ ] **Step 1: 运行编译**

```bash
npm run build 2>&1 | tail -30
```

Expected: 无TypeScript错误

- [ ] **Step 2: 检查Tauri构建**

```bash
cd src-tauri && cargo build --bin proxybot-gui 2>&1 | tail -20
```

Expected: 编译成功

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat(graph): complete Graph visualization upgrade"
```

---

## 验证清单

- [ ] Waterfall视图显示请求时间线
- [ ] DAG视图显示请求依赖关系
- [ ] Auth视图显示Mermaid状态机
- [ ] 三视图可切换
- [ ] 编译通过
