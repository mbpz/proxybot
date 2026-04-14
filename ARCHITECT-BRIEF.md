# Architect Brief — ProxyBot

## Step 1 — 项目骨架 + HTTPS MITM 代理 ✅ (完成，验收通过)

---

## Step 2 — pf 透明代理 + IP 转发（macOS）

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
