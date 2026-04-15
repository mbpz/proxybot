# Architect Brief — ProxyBot

## Step 5 — WSS (WebSocket over HTTPS) 拦截 ✅ (完成)

---

## Step 6 — 请求详情面板

目标：点击任意 HTTP 请求，弹出详情面板查看请求 Header / Response Body / 时间线。

### Rust 后端修改

**1. 扩展 InterceptedRequest**

```rust
struct InterceptedRequest {
    pub id: String,
    pub timestamp: String,
    pub method: String,
    pub host: String,
    pub path: String,
    pub status: u16,
    pub latency_ms: u64,
    pub app_name: Option<String>,
    pub app_icon: Option<String>,
    pub request_headers: Option<String>,    // "Header-Name: value\r\n..."
    pub response_headers: Option<String>,
    pub response_body: Option<String>,       // 最多 10KB，超出截断
    pub request_body: Option<String>,
}
```

**2. Response Body 读取**

在 `handle_http` 里，读上游响应时：
1. 解析 HTTP Status Line 和 Headers
2. 读 body 到 buffer（最多 10KB）
3. UTF-8 解码：成功存 `response_body`，失败存 `[Binary N bytes]`

**3. Request Header 读取**

读客户端请求时：
1. 解析 Request Line + Headers
2. 存 `request_headers`

**4. Tauri Command**

```rust
#[tauri::command]
pub fn get_request_detail(id: String) -> Option<InterceptedRequest>;
```

前端按 ID 查找并返回完整信息。

### UI 修改（App.tsx）

**1. 请求列表点击**

- 选中行高亮（`bg-blue-100 dark:bg-blue-900`）
- 右侧滑出详情面板

**2. 详情面板布局（右侧 40%）**

三个 Tab：
- **General**: Method, URL, Status, Latency, App, Time
- **Headers**: Request Headers (key-value) + Response Headers (key-value)
- **Body**: Request Body + Response Body，超长截断显示

**3. 关闭**

点空白区域或 X 按钮关闭面板。

### 不做

- 流式响应实时展示
- 请求重放
- Binary body hex dump
- WebSocket 帧详情

### 验收标准

1. 点请求 → 详情面板弹出，右侧展示
2. Headers Tab 有完整的 Req + Resp headers
3. Body Tab 有 Response Body（10KB 内）
4. 点 X 或空白 → 面板关闭
