# ProxyBot — Build Log

## Step 5 — WSS (WebSocket over HTTPS) 拦截 ✅ (完成)
**日期:** 2026-04-15
**状态:** 构建完成，cargo check 0 errors, npm run build 成功

### 完成内容
- `proxy.rs`:
  - `WssMessage` 结构体，字段：id, timestamp, host, direction, size, content, app_name, app_icon
  - `is_websocket_upgrade()` 函数：解析 HTTP 请求检测 Upgrade: websocket 和 Connection: Upgrade 头
  - `compute_ws_accept_key()` 函数：RFC 6455 SHA1+base64 计算 Sec-WebSocket-Accept
  - 修改 `handle_https_connect`：TLS 握手后读取 HTTP 请求检测 WebSocket upgrade
  - 检测到 WebSocket 时：发送 101 Switching Protocols，使用 `tokio-tungstenite::WebSocketStream::from_raw_socket` 包装 TLS 流
  - `handle_websocket_relay()` 函数：双向 relay，每条 Text/Binary 帧触发 `intercepted-wss` event
  - Ping/Pong/Close 帧处理：Ping 回复 Pong 并透传，Close 转发后断开
- `Cargo.toml` 新增依赖：`tokio-tungstenite = "0.26"`, `tungstenite = "0.22"`, `sha1 = "0.10"`, `futures-util = "0.3"`
- `App.tsx`:
  - `WssMessage` 接口
  - `wssMessages` state，最多 200 条
  - 监听 `intercepted-wss` event
  - WSS Tab 界面：Time | Direction | Host | Size | Content Preview

### 依赖变更
- 新增 `tokio-tungstenite`, `tungstenite`, `sha1`, `futures-util` crates
- `proxy.rs` 新增 import

### Known Gaps（后续步骤处理）
- WSS 消息持久化（Step 6）
- WSS 连接按 Host 分组

### 验收标准
待验证：手机打开微信（iOS），ProxyBot WSS Tab 出现 WebSocket 消息

---

## Step 4 — App 分类规则库 ✅ (完成)
**日期:** 2026-04-14
**状态:** 通过 Richard 二次 review，cargo test 4 passed

### 完成内容
- `app_rules.rs`：WeChat/Douyin/Alipay 域名规则库，精确匹配 + 子域名边界匹配
- 防止 `qq.com.evil.com` 等 look-alike 域名攻击
- 4 个单元测试覆盖正常匹配和误匹配场景
- `proxy.rs`：HTTPS CONNECT + 透明 HTTP 两条路径都调用 `classify_host()`
- App.tsx：Tab 过滤（All/WeChat/Douyin/Alipay/Unknown），请求表 App 列

---

## Step 3 — 内置 DNS 服务器 ✅
**日期:** 2026-04-14
**状态:** 构建完成，待 Richard review，cargo check 0 错误

### 完成内容
- DNS 服务器模块 `src-tauri/src/dns.rs`（新建）
- UDP 监听 5300 端口（pf 转发 53->5300，无需 root）
- RFC 1035 QNAME 解析（DNS Question Section 手动解析）
- 转发 DNS 查询到 8.8.8.8:53，3秒超时，原样回传响应
- `DnsEntry { domain, timestamp_ms }` 存储在 `Arc<Mutex<VecDeque>>`，最多 10000 条
- Tauri event `dns-query` 实时推送查询到前端
- `get_dns_log` Tauri command 返回最近 50 条记录
- pf anchor 新增 UDP 53 -> 127.0.0.1:5300 重定向规则
- DNS 启停与 pf setup/teardown 联动
- UI Setup 面板增加 DNS 状态指示器
- UI 新增「DNS 查询」区域，显示最近 50 条（时间 + 域名），实时更新

### Known Gaps（后续步骤处理）
- DNS 查询按 App 分类（Step 4）
- DNS 缓存
- DNSSEC 验证

### 验收标准
待验证：手机 Wi-Fi DNS 设为 PC IP，手机访问任意网页，ProxyBot「DNS 查询」面板出现对应域名记录

---

## Step 2 — pf 透明代理 + IP 转发 ✅
**日期:** 2026-04-14
**状态:** 构建完成，通过 Richard 二次 review，cargo check 0 错误 0 警告

### 完成内容
- macOS pf anchor 规则（rdr + pass 分离），端口 80/443 重定向到 8080
- DIOCNATLOOK ioctl 恢复原始目标地址（macOS pf NAT 表查询）
- `sysctl net.inet.ip.forwarding` 启用/关闭 IP 转发
- osascript 权限提升（系统密码弹框）
- peek() 检测 TLS ClientHello，不消耗字节
- 接口名称注入防护（仅允许 alphanumeric）
- UI Setup 面板：显示 PC LAN IP，启用/停止透明代理按钮
- teardown_pf 完整清理：移除 pf 规则 + 关闭 IP 转发

### Known Risks（需运行时验证）
- DIOCNATLOOK direction 字段：当前 PF_OUT(2)，若 NAT 查询失败改试 PF_IN(1)
- pfioc_natlook 结构体布局无法 100% 验证（macOS 内核头文件未公开）

### 验收结果 ✅
点「启用透明代理」→ 系统密码弹框 → 输入后 pf 规则加载成功

---

## Step 1 — Tauri 骨架 + HTTPS MITM 代理 ✅
**日期:** 2026-04-14
**状态:** 构建完成，通过 Richard 二次 review，cargo check 0 错误

### 完成内容
- Tauri v2 + React + TypeScript 项目骨架
- Rust HTTPS MITM 代理，监听 8080 端口（HTTP CONNECT 显式代理）
- `rcgen` 生成根 CA（~/.proxybot/ca.crt + ca.key），每 host 动态签发叶子证书
- `rustls` 双向 TLS（TlsAcceptor 对浏览器，TlsConnector 对上游）
- AtomicBool 防止重复启动代理
- Tauri event 实时推送拦截请求到 React 前端
- 极简请求列表 UI（host / path / status / 耗时）

### Known Gaps（后续步骤处理）
- Box::leak() SNI hostname，每连接少量内存泄漏
- cert 生成失败时降级为盲转发（非崩溃）
- 尚无 pf 透明代理（手机流量路由）
- 尚无 App 分类（WeChat/Douyin/Alipay 规则库）
- 尚无 DNS 服务器

### 验收结果 ✅
curl -x http://127.0.0.1:8080 https://httpbin.org/get -k → UI 出现 GET httpbin.org /get 200 1281ms
