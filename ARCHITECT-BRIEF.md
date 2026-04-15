# Architect Brief — ProxyBot

## Step 5 — WSS (WebSocket over HTTPS) 拦截

目标：拦截微信/抖音的 WebSocket 长连接请求，分类并展示在 UI 中。

### 背景

微信和抖音大量使用 WebSocket（wss://）进行实时通信：
- 微信：消息推送、心跳、实时状态
- 抖音：直播评论、互动、推送
- 传统 MITM 代理只能拦截 HTTP CONNECT 隧道里的 HTTP 请求，WebSocket 在 TLS 层之上直接升级为 WebSocket 帧流，现有代理逻辑未处理

### Rust 后端修改

**1. WebSocket 检测（proxy.rs）**

在 HTTPS CONNECT 隧道建立后，检测 HTTP Upgrade 请求头：
```
Upgrade: websocket
Connection: Upgrade
Sec-WebSocket-Key: ...
```

检测到后：
1. 复用现有的 TLS 连接（不关闭）
2. 完成 WebSocket 握手（101 Switching Protocols）
3. 替换 `handle_https_connect` 中的盲转发为 `tokio-tungstenite` 的 WebSocket 帧中继
4. 中继过程中：解析 WebSocket 帧（Text/Binary），记录消息内容 + 时间戳
5. 对每条消息调用 `classify_host()` 分类（用 WebSocket 握手时的 Host 头）
6. 通过 Tauri event `intercepted-wss` 推送消息到前端
7. 支持双向中继：client → server 和 server → client 都要记录

**2. 新增 Tauri event**

```rust
struct WssMessage {
    pub id: String,
    pub timestamp: String,    // "HH:MM:SS.mmm"
    pub host: String,
    pub direction: String,    // "↑" (send) 或 "↓" (recv)
    pub size: usize,         // bytes
    pub content: String,      // Text 帧内容，Binary 帧显示 "[Binary N bytes]"
    pub app_name: Option<String>,
    pub app_icon: Option<String>,
}
```

Event name: `"intercepted-wss"`

**3. WebSocket 握手处理**

```rust
// 在 TLS 隧道握手完成后，读取 HTTP Upgrade 请求
// 如果是 WebSocket 升级：
//   1. 读取 Sec-WebSocket-Key，构造 Sec-WebSocket-Accept
//   2. 发送 101 Switching Protocols 响应
//   3. 把 TCP 连接升级为 WebSocket 帧处理
//   4. 用 tokio-tungstenite 的 ws_util::util::derive_server_key 计算 accept key
```

**4. 帧解析**

```rust
// 使用 tokio-tungstenite::ws_util::frame::Frame 解析
// opcode: 0x1 = Text, 0x2 = Binary, 0x8 = Close, 0x9 = Ping, 0xA = Pong
// 只记录 Text/Binary 帧内容和方向
// Close 帧：记录后断开连接
// Ping/Pong：透传，不单独记录
```

### UI 新增

- 请求列表增加 **WSS** Tab（与 HTTP 请求列表并列）
- WSS Tab 展示消息流：时间 | 方向 | Host | App | 内容预览
- 点击消息展开详情（完整 Text 内容 或 Binary 十六进制）
- WSS 连接按 Host 分组（同一个 Host 的请求在一起）

### 不做

- WebSocket 帧修改/注入（只记录，不篡改内容）
- WSS 消息持久化（Step 6 持久化历史再做）
- 浏览器开发者工具风格的全帧调试面板

### 验收标准

1. 手机打开微信（iOS），ProxyBot WSS Tab 出现 WebSocket 消息
2. 手机打开抖音，WSS Tab 出现抖音 WebSocket 消息（需手机安装并信任 CA）
3. WSS 消息按 App 分类（WeChat/Douyin）
4. HTTP 请求列表和 WSS 消息列表分离，互不干扰
