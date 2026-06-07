# Network Topology Page Design

**Date:** 2026-06-07
**Author:** Claude

---

## 1. Concept & Vision

A dedicated **environment topology** view for ProxyBot that lets developers see, at a glance, the full network picture: which phones are connected, which apps they hit, which remote servers respond, and where anomalies live. It complements the existing flat Traffic list and per-request Dependency Graph by adding the "horizontal" / cross-cutting perspective that neither of those surfaces.

**Target user:** Mobile developers (debugging phone APIs), security researchers (analyzing traffic patterns), QA engineers (performance analysis).

**Why this exists:**
- The Traffic page is a flat request list — strong for drill-down, weak for environment overview.
- The Graph page (DAG / AuthState / Waterfall) is per-request — strong for one request's dependency chain, weak for cross-device / cross-app relationships.
- When something breaks, the first question is usually "is it the phone, the proxy, or the remote?" Today users have to guess by reading logs; the topology page answers it visually.

**Core principles:**
- Cyberpunk theme consistency — `#00d4ff` cyan, `#a855f7` purple, dark surfaces.
- Three views, one dataset — Radial (overview), Layered (diagnosis), Grouped (performance).
- Drill-down via side drawer — never navigate away mid-investigation.
- Manual refresh, not real-time animation — keeps layout stable.
- Independent filtering with optional global sync.

---

## 2. Architecture

### 2.1 High-level architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    ProxyBot Desktop (Tauri)                      │
│                                                                  │
│  ┌──────────────────┐    ┌──────────────────┐                   │
│  │  Rust backend     │    │  React frontend   │                   │
│  │  (src-tauri/)    │◄──►│  (src/)           │                   │
│  │                  │    │                  │                   │
│  │ • SQLite traffic  │    │ • TopologyPage   │                   │
│  │ • devices table   │    │ • TopologyCanvas │                   │
│  │ • DNS log         │    │ • DetailDrawer   │                   │
│  │   ↓               │    │ • TopologyFilter │                   │
│  │ • New Tauri cmd   │    │   ↓              │                   │
│  │   build_topology  │    │ • 3 view comps   │                   │
│  │   _graph()        │    │   RadialView     │                   │
│  └──────────────────┘    │   LayeredView    │                   │
│           │              │   GroupedView    │                   │
│           ▼              │ • vis-network     │                   │
│   SQLite aggregate       └──────────────────┘                   │
└─────────────────────────────────────────────────────────────────┘
```

### 2.2 Key architectural decisions

1. **Backend aggregates, frontend renders.** Rust collapses SQLite traffic logs + devices table + DNS log into a single `{nodes, edges}` payload. Frontend never runs SQL. Payload is < 1KB typical, < 50KB worst case (500-node cap).
2. **New Tauri command `build_topology_graph(filter, time_window)`** in `src-tauri/src/topology/mod.rs`. Reuses existing `AppState` (DB pool) and `safeInvoke` on the frontend.
3. **Node = (device, app_tag, host) tuple, plus a single Proxy node.** Edges are direct device → host connections; the App node sits as a decorative middle node for the Layered view.
4. **Refresh rebuilds the whole vis-network DataSet** rather than incremental updates. Avoids animation jitter and keeps the layout stable.

### 2.3 File structure

```
src-tauri/src/topology/
  mod.rs              # module entry + tauri::command registration
  types.rs            # TopologyGraph / Node / Edge / Filter types
  builder.rs          # SQLite → TopologyGraph aggregation logic
  tests.rs            # aggregation unit tests

src/components/topology/
  TopologyPage.tsx        # route component, composes children
  TopologyCanvas.tsx      # vis-network container + tab switcher
  TopologyFilter.tsx      # top filter bar
  TopologyDetail.tsx      # right-side detail drawer
  views/
    RadialView.tsx        # vis-network hierarchical config
    LayeredView.tsx       # vis-network hierarchical LR
    GroupedView.tsx       # vis-network physics + cluster
  hooks/
    useTopologyGraph.ts   # invoke + state management
  types.ts
  index.ts
```

---

## 3. Components & Views

### 3.1 Component tree

```
<TopologyPage>                       ← route: /topology
├── <TopologyFilter>                 ← top filter bar
│   ├── DeviceSelect                 ← multi-select (from list_devices)
│   ├── AppTagSelect                 ← multi-select (WeChat/Douyin/Alipay/Other)
│   ├── HostSearchInput              ← keyword input
│   ├── SyncGlobalToggle             ← sync-global filter switch
│   └── RefreshButton                ← manual refresh
├── <TopologyCanvas>                 ← main canvas area
│   ├── <TabSwitcher>                ← Radial / Layered / Grouped
│   └── <visNetworkContainer>        ← single vis-network instance
│       ├── <RadialView>             ← Tab 1
│       ├── <LayeredView>            ← Tab 2
│       └── <GroupedView>            ← Tab 3
└── <TopologyDetail>                 ← right-side detail drawer (hidden by default)
    ├── <DetailHeader>               ← node name + close button
    ├── <MetricsSummary>             ← composite metrics
    ├── <RecentRequestsList>         ← related requests (virtual scroll)
    ├── <StatusBreakdown>            ← 2xx/4xx/5xx bar chart
    └── <JumpToTrafficButton>        ← navigate to Traffic with prefilled filter
```

### 3.2 View differences

| View | vis-network layout | Node grouping | Edge order | Best for |
|------|--------------------|---------------|------------|----------|
| **Radial** | `hierarchical.direction: 'UD'` + center pin | Devices in center, Apps around | by duration desc | Environment overview |
| **Layered** | `hierarchical.direction: 'LR'` + sortMethod | Device → Proxy → App → Host | by error rate | Anomaly diagnosis |
| **Grouped** | `physics.solver: 'forceAtlas2Based'` + cluster | Auto-cluster by `app_tag` | by total bytes | Performance analysis |

### 3.3 Interaction details

**Node color rules (consistent across views):**
- Proxy node: `#a855f7` (purple)
- App node: `#22c55e` (green)
- Device node: `#00d4ff` (cyan)
- Host node: neutral border, text `#ffffff`
- Error rate ≥ 10%: node border switches to `#ff4d4d` with glow

**Node size:** `baseSize + log2(requestCount) * scale` — keeps both 1-request and 10000-request nodes readable.

**Edge visuals:**
- Default color: `#8888aa`; anomalous edges: `#ff4d4d` (dashed)
- Width: proportional to `total_bytes`
- Animation: hover-highlight only (no idle animation)

**Click behavior:**
- Node → open `TopologyDetail` drawer
- Edge → small popover with the host + 5 most recent requests
- Background click → close drawer

**Keyboard shortcuts:**
- `Cmd/Ctrl + R` → refresh topology
- `1` / `2` / `3` → switch tabs
- `Esc` → close drawer
- `/` → focus HostSearchInput

---

## 4. Data Model

### 4.1 Rust types (`src-tauri/src/topology/types.rs`)

```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TopologyGraph {
    pub nodes: Vec<TopologyNode>,
    pub edges: Vec<TopologyEdge>,
    pub meta: TopologyMeta,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TopologyNode {
    pub id: String,                    // e.g. "device:abc123" / "app:wechat" / "host:api.weixin.qq.com" / "proxy:main"
    pub kind: NodeKind,                // Device | App | Host | Proxy
    pub label: String,
    pub app_tag: Option<String>,
    pub device_id: Option<String>,
    pub request_count: u64,
    pub total_bytes: u64,
    pub avg_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub error_count: u64,
    pub error_rate: f64,               // 0.0 - 1.0
    pub last_seen: i64,                // unix ms
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TopologyEdge {
    pub id: String,
    pub from: String,                  // node id
    pub to: String,
    pub request_count: u64,
    pub total_bytes: u64,
    pub avg_latency_ms: f64,
    pub error_rate: f64,
    pub is_anomalous: bool,            // error_rate > 0.10 or p95 > threshold
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TopologyMeta {
    pub total_requests: u64,
    pub total_bytes: u64,
    pub device_count: u32,
    pub app_count: u32,
    pub host_count: u32,
    pub time_range: (i64, i64),        // (start, end) unix ms
    pub built_at: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    Device,
    App,
    Host,
    Proxy,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct TopologyFilter {
    pub device_ids: Option<Vec<String>>,
    pub app_tags: Option<Vec<String>>,
    pub host_contains: Option<String>,
    pub time_window: Option<TimeWindow>,
    pub sync_global: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TimeWindow {
    Last5Min,
    Last1Hour,
    Session,
    Custom { start: i64, end: i64 },
}
```

### 4.2 SQL aggregation

**Device nodes:**
```sql
SELECT id, name, last_seen FROM devices WHERE last_seen >= ?;
```

**Aggregated request nodes (by device × app_tag × host):**
```sql
SELECT
  device_id,
  COALESCE(app_tag, 'unknown') AS app_tag,
  host,
  COUNT(*) AS request_count,
  SUM(body_size) AS total_bytes,
  AVG(duration_ms) AS avg_latency_ms,
  SUM(CASE WHEN status >= 400 THEN 1 ELSE 0 END) AS error_count
FROM requests
WHERE timestamp >= ? AND timestamp <= ?
GROUP BY device_id, app_tag, host;
```

**Edges:** `device → host` direct connection. The App node sits as a visual middle node for the Layered view but is not part of the edge in the data model (one logical device-to-host relationship, three visual layers).

### 4.3 Tauri command interface

```rust
#[tauri::command]
pub async fn build_topology_graph(
    filter: TopologyFilter,
    state: tauri::State<'_, AppState>,
) -> Result<TopologyGraph, String>;

#[tauri::command]
pub async fn get_topology_node_detail(
    node_id: String,
    filter: TopologyFilter,
    state: tauri::State<'_, AppState>,
) -> Result<NodeDetail, String>;
// NodeDetail = { node, recent_requests: Vec<InterceptedRequest>, status_breakdown: HashMap<u16, u64> }
```

### 4.4 Frontend state

```typescript
interface UseTopologyGraphState {
  graph: TopologyGraph | null;
  loading: boolean;
  error: string | null;
  lastBuiltAt: number | null;
}

// Calls build_topology_graph, debounced 300ms on filter change.
```

---

## 5. Error Handling

| Error type | Trigger | User-facing | Recovery |
|-----------|---------|-------------|----------|
| **Empty data** | No traffic or filter too narrow | Centered empty state: "还没有流量数据" + "清除过滤器" button | One-click clear |
| **SQLite error** | DB lock, disk full, schema mismatch | Top red banner: "无法读取数据：{error}" + "重试" button | Click retry |
| **Node over limit** | > 500 nodes (performance threshold) | Toast warning: "当前显示前 500 个节点（共 1200 个），请缩小过滤范围" | Auto-dismiss after 5s |
| **Build timeout** | Aggregation > 5s on large dataset | Loading spinner continues + cancel button | Cancel restores last successful result |
| **Node detail failure** | Drawer open but invoke fails | In-drawer red banner: "无法加载详情" + "重试" | Click retry |
| **vis-network render error** | Canvas context loss, etc. | Full-screen fallback: "拓扑渲染失败：{error}" + "重新构建" | destroy + rebuild |
| **Filter has no match** | Filter narrows to 0 nodes | Centered: "没有匹配的节点" + "清除过滤器" | One-click clear |

**Input validation:**
- `host_contains` length ≤ 100 chars
- `device_ids` / `app_tags` count ≤ 50
- `TimeWindow::Custom` range ≤ 30 days

**Frontend error capture pattern:**

```typescript
try {
  const result = await safeInvoke<TopologyGraph>("build_topology_graph", { filter });
  if (result.nodes.length > 500) {
    toast.warning(`显示前 500 个节点（共 ${result.nodes.length} 个）`);
  }
  setGraph(result);
} catch (err) {
  if (err instanceof TimeoutError) {
    setError("构建超时，请缩小时间范围");
  } else {
    setError(`无法读取数据: ${err.message}`);
  }
}
```

**Backend error handling:**
- All errors return `Result<_, String>` with human-readable messages.
- Aggregation runs in `tokio::task::spawn_blocking` to avoid blocking the UI thread.
- 5-second timeout via `tokio::time::timeout`; cancels the query and returns a timeout error.

**Error visual style:** `var(--accent-red)` + existing `error-banner` class. Loading state: centered spinner + "构建拓扑中..." text. Empty state: centered icon + guidance.

---

## 6. Testing

### 6.1 Rust unit tests (`src-tauri/src/topology/tests.rs`)

Pure-function aggregation tests, no Tauri runtime:

- `test_aggregate_empty_db` — empty DB → 0 nodes, 0 edges
- `test_aggregate_single_device_single_host` — basic tuple aggregation
- `test_aggregate_metrics_accuracy` — count/bytes/latency/error_rate numeric correctness
- `test_aggregate_filter_by_device` — device filter effective
- `test_aggregate_filter_by_app_tag` — app_tag filter effective
- `test_aggregate_filter_by_host_contains` — keyword fuzzy match
- `test_aggregate_time_window_session` — Session window contains all data
- `test_aggregate_time_window_last_hour` — time boundary correctness
- `test_aggregate_groups_by_app_tag` — same host with different app_tags produces distinct nodes
- `test_aggregate_error_rate_calculation` — 4xx/5xx rate correctness
- `test_aggregate_p95_latency` — P95 latency algorithm
- `test_node_limit_500` — over-limit returns first 500
- `test_anomalous_edge_detection` — error_rate > 10% marks `is_anomalous`

### 6.2 Tauri command integration tests (`src-tauri/tests/topology_integration.rs`)

- `test_command_returns_graph` — `build_topology_graph` returns the correct structure
- `test_command_respects_filter` — filter params correctly forwarded
- `test_command_handles_db_error` — injected DB error returns `Err`

### 6.3 Frontend unit tests (`src/components/topology/__tests__/`)

- `useTopologyGraph.test.ts` — state machine, loading/error/data transitions, debounce
- `RadialView.test.ts` — vis-network options configured correctly
- `LayeredView.test.ts` — same
- `GroupedView.test.ts` — physics options correct
- `TopologyFilter.test.ts` — filter interactions, sync-global toggle
- `TopologyDetail.test.ts` — node click opens drawer, metrics render
- `node_color.test.ts` — color rules (device/app/host/proxy/error rate)

### 6.4 End-to-end tests (`e2e/topology.spec.ts`)

- `e2e: open topology page` — `/topology` route opens cleanly
- `e2e: switch tabs` — 1/2/3 keys switch Radial/Layered/Grouped
- `e2e: click node opens drawer` — clicking a node opens the detail panel
- `e2e: filter narrows graph` — filter reduces node count
- `e2e: refresh button` — refresh button triggers rebuild
- `e2e: empty state on no traffic` — empty data shows empty state

### 6.5 Coverage targets

- Rust aggregation logic: ≥ 90% line coverage
- Frontend components: ≥ 80% line coverage
- E2E: all 5 core user paths covered

### 6.6 Performance benchmarks

- 10,000-request dataset: aggregation build ≤ 500ms
- 100 nodes + 200 edges: vis-network first render ≤ 1s
- Pan/zoom: 60fps (Playwright performance API sampling)

---

## 7. Out of Scope (deferred to future iterations)

- 3D globe / geo map of remote servers
- Replay-in-topology (replay a request and watch edge animate)
- Topology snapshots / save-load topology state
- Diff between two topology snapshots (regression detection)
- Time-slider scrubbing through historical topology states
- Export topology as PNG / SVG
- Custom user-defined node grouping rules

These are explicitly out of scope to keep the first iteration focused and shippable in a reasonable window.

---

## 8. Approval

This design is ready for implementation. Proceed to writing the implementation plan.
