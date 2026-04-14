# Architect Brief — ProxyBot

## Step 1 — 项目骨架 + HTTPS MITM 代理 ✅ (完成，验收通过)

---

## Step 2 — pf 透明代理 + IP 转发（macOS） ✅ (完成)

---

## Step 3 — 内置 DNS 服务器

目标：PC 同时作为手机的 DNS 服务器，记录手机发出的所有 DNS 查询（域名 + 时间戳），供后续 App 分类用。

### Rust 后端新增

**1. DNS 服务器模块 `src-tauri/src/dns.rs`**
- 监听 UDP 0.0.0.0:53（需要 root，通过 pf setup 时一并申请，或单独申请）
- 收到查询后：
  1. 记录查询域名 + 时间戳到内存（`Arc<Mutex<VecDeque<DnsEntry>>>`，最多保留 10000 条）
  2. 转发查询到上游 DNS（`8.8.8.8:53`），等待响应
  3. 将响应原样返回给手机
- 结构体：`DnsEntry { domain: String, timestamp: u64 /* ms */ }`
- 上游转发用 tokio UDP socket，超时 3 秒
- Flag: 不依赖任何第三方 DNS 库，用 tokio::net::UdpSocket 手动处理 UDP 数据包，DNS 报文直接透传（不解析，只记录域名用于分类）
- Flag: 需要解析 DNS 报文中的 Question Section 取出查询域名，不需要解析其他字段

**2. DNS 报文解析（仅 Question Section）**
- DNS 报文格式（RFC 1035）：12 字节 header + Question Section
- Question Section：QNAME（length-prefixed labels，以 0x00 结尾）+ QTYPE(2) + QCLASS(2)
- 实现 `fn parse_dns_query(buf: &[u8]) -> Option<String>` 解析 QNAME
- 不需要解析 QTYPE/QCLASS，不需要解析 Answer Section

**3. DNS 启停**
- `start_dns_server(app_handle)` → 启动 UDP 监听，通过 `tauri::async_runtime::spawn`
- `stop_dns_server()` → 通过 AtomicBool shutdown 信号停止
- DNS 服务器与 pf setup 联动：`setup_pf` 时同时启动 DNS，`teardown_pf` 时同时停止 DNS
- 监听端口 53 需要 root：在 osascript 脚本里加 `sudo -n` 或通过 socket 权限解决
  - 推荐方案：用 `launchctl` 或在 osascript 中用 sudo 起一个小的 UDP 转发进程监听 53，再转给 ProxyBot 监听的高位端口（如 5300）
  - 简单方案（优先）：直接在 osascript 特权 shell 里加 `sysctl` 允许低端口，或用 macOS `com.apple.security.network.server` entitlement
  - **最简方案（推荐）**：在 pf anchor 里加一条 `rdr` 把 UDP 53 重定向到本地 5300 端口，ProxyBot 监听 5300，无需 root 绑定 53

**4. Tauri command**
- `get_dns_log() -> Vec<DnsEntry>`：返回最近 DNS 查询记录
- DNS 新查询时通过 Tauri event `dns-query` 推送到前端

**5. pf.rs 修改**
- 在 `setup_pf` 的 anchor 规则里增加：
  `rdr on {interface} proto udp from any to any port 53 -> 127.0.0.1 port 5300`
- `teardown_pf` 不需要改（规则随 anchor 一起清除）

### UI 新增

- Setup 面板增加 DNS 状态指示（监听中 / 未启动）
- 新增「DNS 查询」标签页（或折叠面板），展示最近 50 条查询：时间 + 域名
- 实时更新（listen Tauri event `dns-query`）

### 不做

- DNS 缓存
- DNS 解析（自己响应查询）
- DNSSEC 验证
- App 分类（Step 4）

### 验收标准

1. 启用透明代理后，手机 Wi-Fi DNS 设为 PC IP
2. 手机访问任意网页
3. ProxyBot「DNS 查询」面板出现对应域名记录

目标：手机将 PC 设为网关后，手机所有 TCP 80/443 流量自动被 ProxyBot 拦截，无需在手机上配置代理。

### Rust 后端新增

**1. 网络接口检测**
- 新增 Tauri command `get_network_info() -> NetworkInfo`
- 返回：PC 当前局域网 IP（en0 或其他活跃接口）、接口名称
- 用于在 UI 上告知用户「将手机网关/DNS 设为 X.X.X.X」

**2. 透明代理模式支持**
- 代理监听端口保持 8080
- 新增处理透明代理连接的逻辑：从 TCP 连接的原始目标地址（SO_ORIGINAL_DST / getsockopt）还原真实目标 host:port
- macOS 上通过 `getsockopt(SO_ORIGINAL_DST)` 或读取 pf state table 获取原始目标
- Flag: 必须用 `nix` crate 做 socket 操作，不调用外部命令获取目标地址

**3. pf 规则管理**
- 新增 Tauri command `setup_pf(interface: String) -> Result<String, String>`
- 执行以下操作（需要 sudo，通过 macOS AuthorizationExecuteWithPrivileges 或提示用户输入密码）：
  - `sysctl -w net.inet.ip.forwarding=1`（启用 IP 转发）
  - 写入 pf anchor 规则到 `/etc/pf.anchors/proxybot`：
    ```
    rdr pass on <interface> proto tcp from any to any port 80 -> 127.0.0.1 port 8080
    rdr pass on <interface> proto tcp from any to any port 443 -> 127.0.0.1 port 8080
    ```
  - `pfctl -a com.proxybot -f /etc/pf.anchors/proxybot`（加载规则）
  - `pfctl -e`（启用 pf，如果未启用）
- 新增 Tauri command `teardown_pf() -> Result<(), String>`：移除规则，关闭 IP 转发
- Flag: pf 规则写入和 pfctl 命令需要 root 权限，通过 `std::process::Command` + sudo 执行，或使用 macOS Security framework
- Flag: 不要使用 iptables（Linux），只用 pf（macOS）

**4. 权限提升**
- 使用 `std::process::Command::new("sudo")` 执行 pfctl 命令
- 或使用 osascript 弹出系统密码对话框：`osascript -e 'do shell script "..." with administrator privileges'`
- 推荐 osascript 方式，用户体验更好

### UI 新增

- **Setup 面板**（新 tab 或侧边栏）：
  - 显示 PC 局域网 IP（从 `get_network_info` 获取）
  - 「启用透明代理」按钮 → 调用 `setup_pf`，成功后显示绿色状态
  - 「停止透明代理」按钮 → 调用 `teardown_pf`
  - 提示文字：「将手机 Wi-Fi 网关和 DNS 均设置为 [PC IP]」

### 不做

- DNS 服务器（Step 3）
- App 分类（Step 4）
- Windows 支持

### 验收标准

1. 点「启用透明代理」→ 系统弹出密码框 → 输入后 pf 规则加载成功
2. 手机 Wi-Fi 设置网关 = PC IP，手机访问 http://httpbin.org/get → ProxyBot UI 出现该请求
3. 手机访问 https://httpbin.org/get（需手机已安装 ProxyBot CA）→ UI 出现解密后的请求
4. 点「停止透明代理」→ 手机恢复正常上网
