# OpenAPI/AsyncAPI Spec Generation — Design

**Date:** 2026-06-16
**Status:** Draft → Spec self-review pending
**PRD Reference:** `tasks/prd-netmind-agent.md` FR-23 + SM-4
**Author:** Three-Man Team (Arch / Bob / Richard)

---

## 1. Background & Goals

ProxyBot 已具备 LLM 推断接口语义的能力（`src-tauri/src/infer.rs`），但缺少将这些语义**落地为可复用的 API 规范**。本设计实现 **FR-23**：从已捕获的 HTTP/WebSocket/SSE 流量，自动生成 OpenAPI 3.1 与 AsyncAPI 2.x 规范，并通过倒带重放验证（SM-4）确保生成的规范可执行。

**目标**

- G-1: 用户在 AI 页面一键触发 → 获得 OpenAPI 3.1 + AsyncAPI 2.x 规范
- G-2: 生成的规范能通过 mock + 重放验证（≥ 80% 通过率，SM-4）
- G-3: 规范覆盖所有 LLM 推断出的接口 + 启发式发现的接口（无 LLM 时也可用降级路径）
- G-4: 持久化生成结果，便于后续 Replay/Mock/FastAPI 复刻（Phase 4 复用）

**非目标**

- N-1: 不实现 OAuth2/OpenID Connect 完整方案（只描述 flow，不实现 provider）
- N-2: 不生成 SDK 代码（cURL/Python/Go 已由 `GenPage.tsx` 覆盖）
- N-3: 不做增量式 live spec（边界留到 Phase 3 后续迭代）

---

## 2. Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    AI 页面 (SpecGenPanel)                │
│  ┌──────────────────┐  ┌──────────────────┐             │
│  │ 推断结果          │  │ 生成的 OpenAPI   │             │
│  │ (inferred.json)  │  │ /AsyncAPI YAML   │             │
│  └──────────────────┘  └──────────────────┘             │
│         ↑                       ↑                        │
│    [生成 spec] 按钮    [复制][下载][重放验证] 按钮         │
└─────────────────┬───────────────────────────────────────┘
                  │ Tauri commands
┌─────────────────▼───────────────────────────────────────┐
│  proxybot-core/src/specgen/ 模块                         │
│  ┌─────────┐  ┌─────────┐  ┌──────────┐  ┌──────────┐  │
│  │ Extract │→ │ DeepSeek│→ │ Validate │→ │  Render  │  │
│  │         │  │  (LLM)  │  │ (Schema) │  │ (YAML)   │  │
│  └─────────┘  └─────────┘  └──────────┘  └──────────┘  │
│                                              ↓          │
│                                       ┌──────────┐      │
│                                       │  Replay  │      │
│                                       │ Validate │      │
│                                       └──────────┘      │
└─────────────────────────────────────────────────────────┘
```

**数据流**

1. 用户在 `SpecGenPanel` 选 session，点"生成"
2. Tauri command `generate_spec(session_id)` 调用 `specgen::build_spec()`
3. `extract` 从 SQLite 取 `TrafficRecord`（FR-19）+ 路径模板化
4. `llm` 调用 DeepSeek V3，传入 prompt + JSON schema 约束
5. `validate` 用 schema 校验返回，失败重试（最多 2 次）
6. `render` 输出 OpenAPI 3.1 / AsyncAPI 2.x YAML
7. 持久化到 `~/.proxybot/specs/<session_id>.json`
8. UI 拉取结果展示，Path 列表 + 详情面板
9. 用户点"重放验证" → `replay` 启动 mock + 回放请求 + 报告 pass_rate

---

## 3. Module Layout

新增 `proxybot-core/src/specgen/`：

```rust
// 核心类型
pub struct SpecRequest {
    pub session_id: String,
    pub traffic_records: Vec<TrafficRecord>,   // FR-19
    pub inferred: Option<InferredSemantics>,   // infer.rs 输出
}

pub enum SpecOutput {
    OpenApi(String),    // OpenAPI 3.1 YAML
    AsyncApi(String),   // AsyncAPI 2.x YAML
}

pub struct SpecResult {
    pub openapi: Option<SpecOutput>,
    pub asyncapi: Option<SpecOutput>,
    pub coverage: CoverageReport,
    pub replay: Option<ReplayReport>,
    pub generated_at: DateTime<Utc>,
    pub source: SpecSource,  // Llm | Heuristic | Hybrid
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SpecSource {
    Llm,        // 全部由 DeepSeek 生成
    Heuristic,  // 全部由 extract 启发式生成（LLM 不可用时降级）
    Hybrid,     // LLM + extract 混合（LLM 失败或产出不完整时，extract 补偿缺失路径）
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageReport {
    pub total_requests: usize,           // 流量记录总数
    pub covered_in_openapi: usize,       // 出现在 openapi paths 中的请求数
    pub covered_in_asyncapi: usize,      // 出现在 asyncapi channels 中的请求数
    pub uncovered_paths: Vec<String>,    // 未覆盖的 path + method
    pub coverage_rate: f32,              // (openapi+asyncapi) / total
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecConfig {
    pub deepseek_api_key: Option<String>,    // None → 从 env 读
    pub max_traffic_records: usize,          // 默认 50
    pub max_retry: u32,                      // 默认 2
    pub enable_replay_validation: bool,      // 默认 true
    pub mock_port: Option<u16>,              // None → 随机
}

// 公共入口
pub async fn build_spec(req: SpecRequest, config: &SpecConfig) -> Result<SpecResult, SpecError>;

// 子模块
mod extract;   // 路径模板化、参数聚类、host 分组
mod llm;       // DeepSeek 调用，JSON schema 约束
mod validate;  // JSON Schema 验证、coverage 检查
mod render;    // 输出 OpenAPI/AsyncAPI YAML
mod replay;    // 启动 mock + 倒带验证
```

### 3.1 错误类型

```rust
#[derive(Debug, thiserror::Error)]
pub enum SpecError {
    #[error("session is empty")]
    EmptySession,
    #[error("DeepSeek call failed: {0}")]
    LlmUnavailable(String),
    #[error("LLM output failed schema validation after {0} retries")]
    SchemaValidationFailed(u32),
    #[error("render failed: {0}")]
    RenderFailed(String),
    #[error("replay failed: {0}")]
    ReplayFailed(String),
}
```

### 3.2 持久化路径

- 生成结果：`~/.proxybot/specs/<session_id>.json`（含 openapi/asyncapi/coverage/replay/generated_at/source）
- API key：`~/.proxybot/config.toml` 的 `[specgen] deepseek_api_key`
- 日志：`~/.proxybot/logs/specgen.log`（按 session 隔离）

---

## 4. DeepSeek Integration

### 4.1 Model & API

- **Model:** `deepseek-chat`（V3，支持 JSON schema constrained output / function calling）
- **Endpoint:** `https://api.deepseek.com/v1/chat/completions`
- **Auth:** API key 从 `~/.proxybot/config.toml` 读取；首次使用 UI 提示用户配置
- **Env var override:** `DEEPSEEK_API_KEY` 优先

### 4.2 System Prompt

```
你是 API 规范生成助手。根据用户提供的流量记录，输出符合 JSON Schema 的 OpenAPI 3.1 路径对象。

规则：
- 路径必须用 {param} 模板化（如 /api/user/123 → /api/user/{id}）
- 不臆造字段，只在流量中实际出现的字段才写
- 每个接口给 operationId (camelCase)、summary、tags
- 至少 1 个 example（从流量 body 取）
- 中文 summary
```

### 4.3 JSON Schema 约束

使用 `response_format: { type: "json_schema", json_schema: { strict: true, schema: {...} } }`：

```json
{
  "type": "object",
  "properties": {
    "paths": {
      "type": "object",
      "additionalProperties": { "$ref": "#/$defs/pathItem" }
    }
  },
  "required": ["paths"],
  "$defs": {
    "pathItem": {
      "type": "object",
      "properties": {
        "get":    { "$ref": "#/$defs/operation" },
        "post":   { "$ref": "#/$defs/operation" },
        "put":    { "$ref": "#/$defs/operation" },
        "delete": { "$ref": "#/$defs/operation" },
        "patch":  { "$ref": "#/$defs/operation" }
      }
    },
    "operation": {
      "type": "object",
      "required": ["operationId", "summary", "responses"],
      "properties": {
        "operationId": { "type": "string", "pattern": "^[a-z][a-zA-Z0-9]+$" },
        "summary":     { "type": "string" },
        "tags":        { "type": "array", "items": { "type": "string" } },
        "parameters":  { "type": "array" },
        "requestBody": { "type": "object" },
        "responses":   { "type": "object" }
      }
    }
  }
}
```

### 4.4 AsyncAPI 单独调用

- 单独一次 LLM 调用
- 不同的 prompt 和 schema
- 仅针对 `WebSocket` upgrade + `text/event-stream` (SSE) 帧

### 4.5 验证与重试

1. DeepSeek 返回 → 用 schema 验证
2. 失败 → 同一 session 重试（最多 2 次）
3. 仍失败 → 降级到 `extract` 启发式输出（无 LLM）
4. UI 顶部黄色横幅："LLM 输出不符合 schema，已用启发式生成（覆盖度较低）"

### 4.6 成本控制

- 流量截断到最近 50 个请求（按时间排序）
- 单次输入 ~10k tokens，输出 ~5k tokens
- 估算成本：~$0.001/spec（DeepSeek V3 当前价格）
- OpenAPI + AsyncAPI = 2 次调用/spec

---

## 5. OpenAPI/AsyncAPI 渲染

### 5.1 OpenAPI 3.1 输出

完整结构：路径 + 方法 + 参数（query/path/header）+ 请求体 schema（推断类型）+ 响应体 schema + 至少一个 example（从原始流量抽取）。

骨架示例：

```yaml
openapi: 3.1.0
info:
  title: <从 inferred.module 取，或 host>
  version: 1.0.0
  description: <generated by ProxyBot specgen at TIMESTAMP>
servers:
  - url: <从 traffic_records 抽样最常见 host>
paths:
  /api/v3/user/profile:
    get:
      operationId: getUserProfile
      summary: 获取用户资料
      tags: [user]
      parameters:
        - name: X-Auth-Token
          in: header
          required: true
          schema: { type: string }
      responses:
        '200':
          description: OK
          content:
            application/json:
              schema: { ... }
              examples:
                default:
                  value: { ... 真实流量截取 ... }
        '401':
          description: 未登录
```

### 5.2 AsyncAPI 2.x 输出

```yaml
asyncapi: 2.6.0
info:
  title: <ws host or sse endpoint>
  version: 1.0.0
servers:
  - url: <wss:// or https:// + path>
channels:
  /ws/chat:
    description: 聊天频道
    subscribe:
      message:
        payload: { ... }
        examples:
          - name: default
            payload: { ... 真实流量截取 ... }
    publish:
      message:
        payload: { ... }
```

### 5.3 渲染器

- 用 `serde_yaml` 序列化结构化数据
- 不直接用字符串模板拼接（避免注入错误）
- 每个 spec 输出前用 `openapi-schema-validator`（或自写 validator）做一次完整 schema 校验

---

## 6. Replay Validation (SM-4)

**目标**：拿生成的 OpenAPI spec 启动 mock，用原始流量回放，验证 ≥80% 请求得到与记录一致的响应。

### 6.1 步骤

1. **Mock server 启动**（复用 `mockgen.rs`）
   - 把 OpenAPI spec 喂进 mockgen
   - 启动在 `127.0.0.1:<random_port>`（默认 19999 已被占用时随机）
   - spec 中 examples 当 fixture
2. **请求重写**
   - 把原始流量的 host 改写为 `127.0.0.1:<port>`
   - 保留 path/method/headers/body
   - 跳过 host 校验（用本地 hosts 绑定或 reqwest `danger_accept_invalid_certs`）
3. **重放 + 对比**
   - 顺序回放 session 内所有请求
   - 记录：HTTP 状态码、响应 body（按 JSON 字段 diff，容差 10%）
   - 统计：`pass` (status 一致 + body diff < 10%) / `fail` / `error`
4. **报告** `ReplayReport`
   ```rust
   pub struct ReplayReport {
       pub total: usize,
       pub pass: usize,
       pub fail: usize,
       pub error: usize,
       pub pass_rate: f32,           // pass / total
       pub failures: Vec<ReplayFailure>,  // path + 错误详情
       pub started_at: DateTime<Utc>,
       pub finished_at: DateTime<Utc>,
       pub mock_port: u16,
   }

   pub struct ReplayFailure {
       pub path: String,
       pub method: String,
       pub expected_status: u16,
       pub actual_status: u16,
       pub body_diff_summary: Option<String>,  // 仅 body 不匹配时填
   }
   ```

### 6.2 Mock 实现（OpenAPI → 路由）

- 自写：spec.paths → 路由表（hyper-based）
- example 命中 body 模板，否则返回 200 + 空 `{}`
- POST/PUT：把 request body 原样回显，模拟 echo
- 路径参数 `{id}` 匹配任意非 `/` 字符

### 6.3 判定逻辑

```
status_match = expected.status == actual.status
body_match = if both have JSON: shallow_diff_rate < 0.1
             else: bytes_eq
pass = status_match && body_match
fail = !pass && actual.is_some()
error = actual.is_none()  // 网络/超时/解析失败
```

---

## 7. UI Design (sniffnet-inspired)

### 7.1 位置

`src/components/ai/SpecGenPanel.tsx`，作为 AI 页面的第 5 个标签 "Spec Gen"（非嵌套在 Inference 标签内部），与 Token Usage、API Inference、Auth Flow、Vision 平级

### 7.2 布局（sniffnet Inspect 页模式）

```
┌─ SpecGenPanel ─────────────────────────────────────────┐
│  [▶ 生成 OpenAPI/AsyncAPI]   [状态: Idle/Running/...]   │
├──────────────────┬──────────────────────────────────────┤
│  Path 列表       │  选中: GET /api/v3/user/profile      │
│  ──────          │  ──────────                          │
│  ▶ /api/v3/auth  │  operationId: getUserProfile         │
│  ▼ /api/v3/user  │  summary: 获取用户资料                │
│     GET profile  │  tags: [user]                        │
│     PUT profile  │  parameters: [id: path, X-Token: hdr] │
│  ▶ /api/v3/feed  │  responses:                          │
│  ▶ ws://chat     │    200: { profile: {id, name} }      │
│                  │    401: { error: "未登录" }           │
│  [复制 YAML]     │                                      │
│  [下载文件]      │  ─── AsyncAPI ───                    │
│  [▶ 重放验证]    │  channel: /ws/chat                   │
│                  │  messages: ...                       │
│                  │  ─── 重放验证 ───                    │
│                  │  ✓ 42/45 pass (93%)  [▶ 重跑]        │
│                  │  ✗ 3 fail: [展开]                    │
└──────────────────┴──────────────────────────────────────┘
```

### 7.3 sniffnet 风格借鉴

- 左侧固定宽度列表，路径用树形折叠（shadcn Collapsible）
- 右侧详情 + 标签页（OpenAPI / AsyncAPI / 重放报告）
- 主题色继承现有 shadcn/ui
- 状态徽章（Idle/Running/Done/Failed/Degraded）用同款 Badge
- 失败项用红色 + 可点击展开
- 数字大字号显示 pass_rate（sniffnet 风格）

### 7.4 交互

- 选 session → 点"生成" → 显示 loading → 完成后自动展开 Path 列表
- 重放验证独立按钮，进度条 + 实时计数
- "复制 YAML" → `navigator.clipboard.writeText`
- "下载文件" → Tauri command `export_spec`，让用户选保存路径

### 7.5 错误展示

- 顶部横幅黄色警告：LLM 不可用/降级/Schema 失败重试中
- 重放失败列表：可展开看每条详情
- 网络错误：toast 通知

---

## 8. Error Handling Summary

| 错误类型 | 处理 | UI 反馈 |
|---|---|---|
| LLM 调用失败（网络/超时/429） | 重试 2 次，指数退避 | "DeepSeek 调用失败，正在重试 (1/2)..." |
| Schema 验证失败 | 重试 1 次 + 调整 prompt 追加"严格按 schema" | "LLM 输出不符合 schema，重新生成" |
| 完全失败（3 次后） | 降级到 `extract` 启发式输出 | 顶部黄色横幅 |
| 流量为空 | 立即返回 `SpecError::EmptySession` | "当前 session 没有流量" |
| Mock 启动失败 | 重放验证显示 error，不影响 spec 本身 | 重放报告区显示错误 |
| 重放时网络错误 | 单条标记 fail/error，统计 error 数 | 重放报告中 `error` 列 |
| API key 缺失 | 用户首次点击"生成"按钮时检测，missing 则弹配置对话框 | 阻塞生成按钮，弹配置对话框 |

---

## 9. Testing Strategy

| 测试类型 | 范围 | 工具 |
|---|---|---|
| 单元：extract | 路径模板化、参数聚类、host 分组 | `cargo test` |
| 单元：validate | JSON schema 验证、coverage 计算 | `cargo test` |
| 单元：render | OpenAPI/AsyncAPI YAML 序列化 | `cargo test`，用 fixture 比对 |
| 单元：replay | mock 启动 + 重放判定逻辑 | `cargo test` + 本地 hyper server |
| 集成：llm | mock DeepSeek server，验证 prompt/schema 正确 | `wiremock-rs` |
| 集成：build_spec | extract → llm → validate → render 流水线 | mock DeepSeek + fixture session |
| 端到端 | Playwright：UI 上点生成，看到结果 | `e2e/spec-gen.spec.ts` |
| 性能 | 50 请求 session 端到端 < 30s | `cargo bench` 或 wall-clock 断言 |

### 9.1 Fixture 测试数据

- `proxybot-core/tests/fixtures/specgen/wechat-session.json` — 50 个微信 API 请求
- `proxybot-core/tests/fixtures/specgen/ws-chat-session.json` — 30 个 WS 帧
- `proxybot-core/tests/fixtures/specgen/sse-feed-session.json` — 20 个 SSE 事件
- `proxybot-core/tests/fixtures/specgen/expected-openapi.yaml` — golden file
- `proxybot-core/tests/fixtures/specgen/expected-asyncapi.yaml` — golden file

### 9.2 Mock DeepSeek Server

- 用 `wiremock-rs` 启动本地 HTTP server
- 接收 `/v1/chat/completions` 请求，返回 fixture JSON
- 验证请求中包含 prompt、schema 约束、API key

### 9.3 端到端 E2E

- 启动 dev server + 注入 fixture session
- Playwright 打开 AI 页面，选 session，点"生成"
- 断言 30s 内看到 Path 列表
- 点"重放验证"，断言看到 pass_rate ≥ 80%

---

## 10. Performance Budget

- 单 session 生成（50 请求）端到端 < 30s（PRD SM-3 修正）
  - extract: < 1s
  - DeepSeek OpenAPI 调用: ~15s（包含网络）
  - DeepSeek AsyncAPI 调用: ~10s
  - validate + render: < 1s
  - 持久化: < 1s
- 重放验证（50 请求） < 10s
- 内存峰值 < 100MB

---

## 11. Open Questions (Resolved)

- ~~Q-3 (PRD): LLM on-device vs API~~ → **DeepSeek API**（用户决定）
- ~~Q-1: 输出完整度~~ → **完整结构**（用户决定）
- ~~Q-2: AsyncAPI 范围~~ → **WebSocket + SSE**（用户决定）
- ~~Q-3: LLM 输出策略~~ → **Schema-constrained + 验证**（用户决定）
- ~~Q-4: UI 位置~~ → **AI 页面新增面板，sniffnet 风格**（用户决定）
- ~~Q-5: 验证方式~~ → **倒带重放验证**（用户决定）

---

## 12. Out of Scope

- OAuth2/OpenID Connect 完整方案实现
- 增量式 live spec（流式生成）
- SDK 代码生成（已有 cURL/Python/Go 导出）
- FastAPI/Hono 后端代码生成（Phase 4 范围）
- Vision-based UI 复制（Phase 4 范围）

---

## 13. Rollout Plan

1. **Step 1**: `specgen::extract` + `specgen::render`（无 LLM，启发式输出）
   - 单元测试 + fixture golden file
2. **Step 2**: `specgen::llm` 集成 DeepSeek，JSON schema 约束
   - wiremock 集成测试
3. **Step 3**: `specgen::build_spec` 端到端 + 持久化
   - 集成测试
4. **Step 4**: `specgen::replay` mock + 重放验证
   - 单元 + 集成测试
5. **Step 5**: Tauri commands 暴露
6. **Step 6**: `SpecGenPanel.tsx` UI 实现
7. **Step 7**: E2E + 性能基准

每个 Step 由 Bob 实现，Richard review，Arch 验收。
