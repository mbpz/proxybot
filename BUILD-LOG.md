# ProxyBot — Build Log

## Step 3 — 内置 DNS 服务器 🚧
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
