# Architect Brief — ProxyBot

## Pending Items — v0.2

---

## Item 1 — 后台常驻进程开关

目标：ProxyBot 可以作为后台常驻进程运行，关闭窗口后不退出。

### 实现

**Rust 端：**
- `tauri.conf.json`: `"windows": [{ "visible": false, "skipTaskbar": true }]` — 无窗口模式
- 在 Setup 或设置里加 Toggle：「后台运行」开关

**UI Toggle：**
```tsx
<div className="setting-row">
  <span>后台运行</span>
  <Toggle
    checked={keepRunning}
    onChange={setKeepRunning}
  />
</div>
```

- `keepRunning` 为 `true` 时：关闭窗口只隐藏到 dock，不退出进程
- `keepRunning` 为 `false` 时：关闭窗口 → `tauri::window::Window::close()` → App 退出
- 用 `tauri::window::Window::hide()` 代替 `close()`

**Tauri Command：**
```rust
#[tauri::command]
pub fn set_keep_running(app_handle: AppHandle, keep: bool) {
    // Store preference in app state
    app_handle.managed_state::<KeepRunningState>().set(keep);
}

#[tauri::command]
pub fn hide_window(app_handle: AppHandle) -> Result<(), String> {
    app_handle.get_webview_window("main")
        .ok_or("no window")?
        .hide()
        .map_err(|e| e.to_string())
}
```

**窗口关闭拦截：**
在 App.tsx 的 window event listener 里：
```tsx
window.addEventListener('beforeunload', (e) => {
  if (keepRunning) {
    e.preventDefault();
    invoke("hide_window");
  }
});
```

---

## Item 2 — 设置面板（右上角入口）

目标：把配置移到设置菜单，窗口右上角放齿轮图标。

### UI 布局

```
┌─────────────────────────────────────────────┐
│ ⚙️ ProxyBot              [🌙 Dark] [☀️ Light] │  ← 顶栏
├─────────────────────────────────────────────┤
│ [HTTP Requests] [WSS Messages] [DNS Queries]│  ← 三个Tab切换
├─────────────────────────────────────────────┤
│                                             │
│            主内容区                           │
│                                             │
└─────────────────────────────────────────────┘

点击 ⚙️ → 右侧滑出设置面板：
┌──────────────────────────┐
│ ⚙️ 设置                   │
│ ────────────────────     │
│ ▶ 透明代理                │ ← 折叠/展开
│   - 启用透明代理  [开关]   │
│   - 当前状态：已启用       │
│ ▶ CA 证书                 │
│   - 下载证书  [按钮]       │
│ ▶ 后台运行                │
│   - 常驻进程  [开关]        │
│ ▶ 清除历史                │
│   - [清空所有记录]         │
└──────────────────────────┘
```

### 实现

- 顶栏右边加齿轮按钮 `<button className="settings-btn">⚙️</button>`
- 点击打开右侧设置面板（和详情面板一样从右侧滑出）
- 设置面板内：透明代理区、CA证书区、后台运行开关、清除历史
- 面板外侧点 overlay 关闭

---

## Item 3 — 三个 Tab 切换

目标：顶部 Tab 栏，三个 Tab：「HTTP 请求」「WSS 消息」「DNS 查询」。

```
┌────────────────────────────────────────────────────┐
│  HTTP Requests    WSS Messages    DNS Queries      │
├────────────────────────────────────────────────────┤
│  搜索 + 过滤栏                                      │
│  ──────────────────────────────────────────────    │
│  请求表格                                           │
│                                                     │
└─────────────────────────────────────────────────────┘
```

每个 Tab 内容：
- **HTTP Requests**：现有请求列表 + 搜索/过滤/导出按钮
- **WSS Messages**：WSS 消息列表
- **DNS Queries**：DNS 查询记录

### 实现

```tsx
const [activeTab, setActiveTab] = useState<'http' | 'wss' | 'dns'>('http');

<div className="top-tabs">
  <button className={activeTab === 'http' ? 'active' : ''} onClick={() => setActiveTab('http')}>
    HTTP Requests ({requests.length})
  </button>
  <button className={activeTab === 'wss' ? 'active' : ''} onClick={() => setActiveTab('wss')}>
    WSS Messages ({wssMessages.length})
  </button>
  <button className={activeTab === 'dns' ? 'active' : ''} onClick={() => setActiveTab('dns')}>
    DNS Queries ({dnsQueries.length})
  </button>
</div>

{activeTab === 'http' && <HttpPanel ... />}
{activeTab === 'wss' && <WssPanel ... />}
{activeTab === 'dns' && <DnsPanel ... />}
```

---

## Item 4 — HTTP 请求行操作菜单

目标：每个请求行右侧加操作按钮：Replay / Copy as cURL。

### 操作按钮

```
[Time] [Method] [Host] [Path] [Status] [Latency] [App] [⧉] [📋]
                                                     ↑Replay  ↑Copy
```

### Copy as cURL

```tsx
const copyAsCurl = (req: InterceptedRequest) => {
  const headers = parseHeaders(req.request_headers || '');
  const headerArgs = headers.map((h: any) => `-H "${h.name}: ${h.value}"`).join(' ');
  const curl = `curl ${headerArgs} ${req.method === 'POST' ? '-d "' + (req.request_body || '') + '"' : ''} "https://${req.host}${req.path}"`;
  navigator.clipboard.writeText(curl);
};
```

### Replay（重放请求）

通过 Rust 后端发起请求：

```rust
#[tauri::command]
pub fn replay_request(id: String) -> Result<(), String> {
    let req = REQUEST_STORE.get(&id).ok_or("not found")?;
    // 使用 req 的 method/host/path/headers 重新发起请求
    // 复用现有 proxy 连接逻辑，只是不走 MITM，直接发到目标
}
```

Replay 后结果以新请求形式出现在列表顶部。

### 实现

- 每行末尾加两个小按钮：⧉ (Replay) 和 📋 (Copy cURL)
- Hover 行时显示按钮
- Click 📋 → clipboard，写入格式化的 cURL 命令
- Click ⧉ → 调用 `invoke("replay_request", { id: req.id })`
