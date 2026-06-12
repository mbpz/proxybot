# QR 一键配网 Design

**Date:** 2026-06-12
**Author:** Claude
**Status:** Approved (pending spec self-review)

---

## 1. Context

ProxyBot 是 macOS 上的 MITM 代理工具。当前移动设备接入流程：

1. 用户手动在手机上设 WiFi 代理 IP / 端口（`http://<lan_ip>:8088`）
2. 用户手动设 DNS（`lan_ip`:5300）
3. 用户打开应用内 Setup 页面，找到 CA 下载链接（`http://<lan_ip>:19876/ca.crt`）
4. 用户在手机上输入 URL 下载 CA 证书
5. 用户手动到 iOS Settings 或 Android Settings 安装 CA 证书
6. iOS 用户还需要去 Certificate Trust Settings 启用完全信任

总步骤 ~6 步，~3-5 分钟，每次接入新设备都要重做一遍。

竞品参考：[Trafexia](https://github.com/danieldev23/trafexia) 的 `IosBridgeDialog.vue` / `AndroidBridgeDialog.vue` 通过 QR 码 + 对话框把"代理 IP、端口、CA 下载"装进一个 QR —— 扫一下就完成（虽然 Trafexia 没有 mobileconfig 自动安装，ProxyBot 这块可以做得更彻底）。

本文设计把 6 步压成 1 步：**扫一个 QR 即可完成 WiFi 代理、DNS、CA 三件套的安装**。

## 2. Goals & Non-Goals

### Goals

- **iOS**：扫一个 QR → Safari 打开 → iOS 弹"安装描述文件"→ 装完即同时获得 WiFi 代理、DNS、CA 三项配置
- **Android**：扫一个 QR → 浏览器打开 → 一个自包含 HTML 页面带 4 步指引（含 CA 下载按钮）
- 两个 QR 在 Setup 页同一面板内以 tab 切换展示
- 复用现有 CertServer（端口 19876），不新增端口
- 不引入新外部依赖

### Non-goals

- **自动启用 iOS CA 完全信任** —— Apple 不允许，强制用户到 Settings 手动启用
- **解决 Android 7+ targetSdk ≥ 24 的 CA 不被应用信任问题** —— Android 系统级限制，超出工具能力
- **设备发现 / mDNS / Bonjour** —— 用户必须知道自己在连 ProxyBot 那个 WiFi
- **批量多设备管理** —— 本期一次配一台，多设备管理是后续工作
- **二维码以外的入口**（NFC、AirDrop 等）

## 3. Architecture

### 3.1 High-level

```
┌────────────────┐  generate_device_qr("ios"|"android")    ┌──────────────────────┐
│ Setup 页       │ ─────────────────────────────────────►  │ cert::mobileconfig   │
│ DeviceQrPanel  │ ◄─────────────────────────────────────  │ cert::wizard         │
└────────────────┘  SVG 字符串                              │ commands::device_    │
                                                            │   setup              │
                                                            └──────────────────────┘
                                                                      │
   手机扫码     ──► http://<lan_ip>:19876/ios.mobileconfig            │
                  http://<lan_ip>:19876/android-setup                  │
                                              │                        │
                                              ▼                        │
                              ┌──────────────────────────────────┐    │
                              │ CertServer (tiny_http:19876)      │    │
                              │  /ca.crt          → CA PEM       │    │
                              │  /ios.mobileconfig → .mobileconfig│◄───┘
                              │  /android-setup   → HTML wizard  │
                              └──────────────────────────────────┘
                                              │
                       ┌──────────────────────┴──────────────────────┐
                       ▼                                             ▼
              iOS 弹"安装描述文件"                          Android 浏览器渲染
              → 装完即获 WiFi+DNS+CA                        → 4 步指引
```

### 3.2 选定的路径

**A. 单一 CertServer 多路径路由** —— 扩展现有 `tiny_http` 的请求循环加一个 `match request.url()`，分发到 3 个 handler（CA 现有、iOS 新、Android 新）。不动端口，不动进程，不动 IPC。

被比较并否决的备选：
- **B. 拆两个 server（19877 专管 QR 配网）** —— 多了健康检查点、多了端口、不必要
- **C. Tauri asset protocol** —— 移动设备不认 `tauri://` URL，直接否决

## 4. Data Structures

### 4.1 iOS `.mobileconfig` (XML plist)

由 `cert::mobileconfig::build_ios_profile(ca_pem, proxy_ip, proxy_port, dns_port) -> String` 动态生成。

包含 **3 个 payload**：

1. **WiFi** (`com.apple.wifi.managed`)
   - `ProxyType = Manual`
   - `ProxyServer = {proxy_ip}`
   - `ProxyServerPort = {proxy_port}`
   - 不指定 SSID —— 应用所有 WiFi 连接
2. **DNS** (`com.apple.dnsSettings.managed`)
   - `DNSSettings.ServerName = {proxy_ip}`
   - `DNSSettings.ServerPort = {dns_port}`
   - `DNSSettings.SupplementalMatchDomains = [""]` (匹配所有域)
3. **Certificate** (`com.apple.security.root`)
   - `PayloadCertificateFileName = "proxybot-ca.cer"`
   - `PayloadContent = <base64(ca_pem)>`

外层 `PayloadDisplayName = "ProxyBot"`, `PayloadDescription`, `PayloadOrganization = "ProxyBot"`, `PayloadRemovalDisallowed = false`, `ConsentText.default` 给安装时的提示文本。

4 个 UUID（每个 payload 一个 + 根 profile 一个），运行时用 `uuid::Uuid::new_v4()` 生成。

完整 XML 模板见设计过程记录（已与用户确认），不重复粘贴。

### 4.2 Android HTML wizard

由 `cert::wizard::build_android_wizard(ca_pem, proxy_ip, proxy_port, dns_port) -> String` 生成。

**单文件自包含 HTML**：CSS inline，JS 无（仅静态指引）。分 4 个 step 块：

1. **WiFi Proxy** — 文字指引：Settings → WiFi → 长按当前网络 → Modify network → Advanced options → Proxy: Manual；显示 `IP: {proxy_ip}` `Port: {proxy_port}`
2. **DNS** — 文字指引：IP settings → Static；`DNS 1: {proxy_ip}` `DNS 2: 1.1.1.1` (fallback)
3. **Install CA** — 大按钮 `<a class="btn" href="/ca.crt" download>Download ProxyBot CA</a>` + 后续安装步骤文字
4. **Verify** — 提示"打开任何 HTTPS App 应该在 ProxyBot UI 看到请求"

并在 step 3 之后追加 ⚠ 提示：
> Android 7+ 默认不信任用户 CA。部分 App（targetSdk ≥ 24）会拒绝 ProxyBot 解密。这是 Android 系统限制。

### 4.3 Tauri 命令签名

```rust
// src-tauri/src/commands/device_setup.rs

#[tauri::command]
pub fn generate_device_qr(platform: String) -> Result<String, String>
// platform: "ios" | "android"
// returns: SVG 字符串（含 <svg ...>...</svg>）
// errors:
//   - "Cert server not started. Start the proxy first."
//   - "Invalid platform: {x}"
```

内部流程：
1. 从 `ProxyState.local_ip` 读 LAN IP
2. 从 `cert_server_port()` (config) 读端口
3. 拼 URL：`http://{lan_ip}:19876/ios.mobileconfig` 或 `/android-setup`
4. 用现有 `qrcode` crate 生成 SVG（`QrCode::new(url).render::<svg::Color>().build()`）

### 4.4 CertServer 路由表

| Path | Content-Type | Body | 备注 |
|---|---|---|---|
| `/ca.crt` (或 `/`, `""`) | `application/x-x509-ca-cert` | CA PEM | **现有行为，保持不变** |
| `/ios.mobileconfig` | `application/x-apple-aspen-config; charset=utf-8` | 动态生成的 .mobileconfig XML | 新增；`Content-Disposition: attachment; filename="proxybot-ios.mobileconfig"` |
| `/android-setup` | `text/html; charset=utf-8` | 动态生成的 HTML wizard | 新增 |
| 其他 | `text/plain` | `Not Found` (404) | 新增，避免 CertServer 行为漂移 |

## 5. Data Flow

### 5.1 生成 QR（应用内）

```
Setup 页打开
  → useEffect 触发 generate_device_qr("ios")
    → 后端读 local_ip + cert_server_port
    → 拼 URL = "http://{lan_ip}:19876/ios.mobileconfig"
    → QrCode::new(URL) → SVG 字符串
  → 前端 <img src={`data:image/svg+xml;utf8,${svg}`}> 渲染
用户点 Android tab
  → generate_device_qr("android") → SVG 同样方式渲染
```

### 5.2 iOS 用户扫码

```
iOS 相机 / 控制中心扫码
  → 识别 URL http://192.168.1.5:19876/ios.mobileconfig
  → Safari 打开
  → CertServer tiny_http 收到 GET /ios.mobileconfig
  → 读 ca.pem
  → build_ios_profile(ca_pem, "192.168.1.5", 8088, 5300) → XML 字符串
  → 返回 200 + Content-Type: application/x-apple-aspen-config
  → iOS 检测到该 MIME → 弹"安装描述文件"系统提示
  → 用户确认 → 描述文件装到 Settings → General → VPN & Device Management
  → 装完即同时启用：
     - WiFi 代理 192.168.1.5:8088（所有 WiFi）
     - DNS 服务器 192.168.1.5:5300
     - 根 CA 证书 "ProxyBot CA" 已安装但**默认不信任**
  → 用户需手动到 Settings → General → About → Certificate Trust Settings
     开启 ProxyBot CA 完全信任（否则 HTTPS 不解密）
```

### 5.3 Android 用户扫码

```
Android 系统扫码 (或扫码 App)
  → 识别 URL http://192.168.1.5:19876/android-setup
  → 浏览器打开
  → CertServer 收到 GET /android-setup
  → 读 ca.pem
  → build_android_wizard(ca_pem, "192.168.1.5", 8088, 5300) → HTML
  → 返回 200 + Content-Type: text/html
  → 浏览器渲染 4 步指引
  → 用户按步骤：
     Step 1: 去系统 WiFi 设置改代理
     Step 2: 改 DNS
     Step 3: 点 Download 按钮下载 CA → 系统弹"安装证书"
     Step 4: 验证
```

## 6. Implementation Notes

### 6.1 Files Changed

**新增**：
- `src-tauri/src/cert/mobileconfig.rs` — `build_ios_profile(ca_pem, proxy_ip, proxy_port, dns_port) -> String` + 单元测试
- `src-tauri/src/cert/wizard.rs` — `build_android_wizard(ca_pem, proxy_ip, proxy_port, dns_port) -> String` + 单元测试
- `src-tauri/src/commands/device_setup.rs` — `generate_device_qr(platform) -> Result<String, String>` + 单元测试
- `src/components/setup/DeviceQrPanel.tsx` — React 组件，tabs 切换 iOS/Android，渲染 QR
- `e2e/qr-onboarding.spec.ts` — Playwright 测试

**修改**：
- `src-tauri/src/cert/mod.rs` — re-export 新模块
- `src-tauri/src/cert_server.rs` — 在 `for request in server.incoming_requests()` 循环里加 `match url` 分发
- `src-tauri/src/lib.rs` — 注册 `device_setup::generate_device_qr` 到 `invoke_handler`
- `src/components/setup/SetupPage.tsx` — 在合适位置嵌入 `<DeviceQrPanel />`
- `src/components/setup/DeviceQrPanel.tsx` — 调用 `invoke("generate_device_qr", { platform: "ios" | "android" })`
- `Cargo.toml` —— 不需新依赖（uuid + qrcode + base64 已在）

### 6.2 No DB / no schema change

纯新增模块 + 扩展 CertServer 路径。`http_requests` 表不变，UI 现有页不变（Setup 页只多一个 panel）。

### 6.3 State management

- `ProxyState.local_ip: Mutex<Option<String>>` —— 现有
- `SERVER_RUNNING: AtomicBool` in `cert_server.rs` —— 现有
- 两者在 `generate_device_qr` 内检查，缺一即返回 Err

### 6.4 URL 协议

统一用 `http://`：
- CertServer 是 `tiny_http`（明文）
- iOS `.mobileconfig` 允许 `http://` 触发安装
- Android HTML 同样
- 链路在用户自家 WiFi 内，物理安全可接受

不在 URL 里加 token / signature —— 在用户可控 LAN 上无意义，反而增加 QR 长度（影响可扫性）。

## 7. Error Handling

| 场景 | 检测 | 处理 |
|---|---|---|
| CertServer 未启动 | `SERVER_RUNNING.load(SeqCst) == false` | 返回 `Err("Cert server not started. Start the proxy first.")`，前端显示 disabled 状态 + tooltip |
| LAN IP 未知 | `ProxyState.local_ip == None` | 返回 `Err("Network info not set. Start the proxy first.")`，同上前端处理 |
| Invalid platform | `platform != "ios" && != "android"` | 返回 `Err("Invalid platform: {x}")` |
| CA 文件不存在 | `std::fs::read_to_string(cert_path)` 失败 | CertServer `/ios.mobileconfig` 路径返回 500 + HTML 错误页："CA cert not found at {path}. Reinstall ProxyBot to regenerate." |
| iOS mobileconfig 安装失败 | 用户操作 | 不归 ProxyBot 管。Setup 页 QR 下方加折叠提示：安装后必须去 Certificate Trust Settings 启用 CA |
| Android 7+ targetSdk ≥ 24 不信任用户 CA | Android 系统 | Wizard HTML 步骤 3 之后追加 ⚠ 提示。接受现实 |
| CertServer 端口被占用 | 启动时 bind 失败 | 现有逻辑：`log::error!` + `SERVER_RUNNING.store(false)` + 命令返回 Err |
| 手机不在 ProxyBot WiFi 扫码 | URL 不可达 | QR 面板上方提示文字："**请确保手机已连接 ProxyBot 所在的 WiFi**" |
| iOS mobileconfig 不指定 SSID 的副作用 | 永远 | 设计接受：用户离开 ProxyBot WiFi 时也会强制走 proxy（断网是预期）。无防御代码 |

## 8. Testing

### 8.1 单元测试（Rust）

`cert::mobileconfig` (`tests` 内):
- `test_build_ios_profile_contains_wifi_proxy` — XML 含 `ProxyServer` + `ProxyServerPort`
- `test_build_ios_profile_contains_dns_payload` — XML 含 `DNSSettings.ServerName` + `DNSSettings.ServerPort`
- `test_build_ios_profile_contains_ca_payload` — XML 含 `PayloadCertificateFileName` + base64(ca_pem) 可解码回原 PEM
- `test_build_ios_profile_payload_count` — `PayloadContent` 数组恰好 3 个 dict
- `test_build_ios_profile_uuids_are_unique` — 4 个 UUID 互不相同
- `test_build_ios_profile_escapes_ampersand_in_proxy_ip` — IP 注入场景（虽然 IP 数字点，但保险起见）
- `test_build_ios_profile_consent_text_present` — 含 `ConsentText.default`

`cert::wizard`:
- `test_build_android_wizard_contains_proxy_ip` — `{proxy_ip}` 替换正确
- `test_build_android_wizard_contains_ca_download_link` — `<a href="/ca.crt" download>` 存在
- `test_build_android_wizard_contains_dns_step` — 含 "DNS 1" + DNS 1 提示
- `test_build_android_wizard_self_contained` — 无 `<link rel="stylesheet" href="http..."` 等外部依赖
- `test_build_android_wizard_contains_android7_warning` — 含 Android 7+ 警告文字
- `test_build_android_wizard_content_type_is_html` — 隐式（HTML doctype 检查）

`commands::device_setup`:
- `test_generate_device_qr_returns_svg_for_ios` — 返回字符串以 `<svg` 开头
- `test_generate_device_qr_returns_svg_for_android` — 同上
- `test_generate_device_qr_svg_contains_correct_url` — SVG path data 反查 URL 困难；改为：抽出一个 `build_qr_url(platform, lan_ip, port) -> String` 纯函数，单独测（iOS 路径、android 路径）
- `test_generate_device_qr_errors_on_unknown_platform` — `platform = "windows"` → Err
- `test_generate_device_qr_errors_when_cert_server_down` —— 难以在单元测试里模拟 SERVER_RUNNING；跳过，留 E2E 覆盖

### 8.2 Playwright E2E

`e2e/qr-onboarding.spec.ts`:
- `device_qr_panel_renders_ios_qr` — 打开 Setup 页，验证 iOS tab 显示 QR（`<svg>` 存在）
- `device_qr_panel_renders_android_qr` — 切到 Android tab，验证 QR 存在
- `device_qr_panel_tabs_switch` — tab 切换无错
- `device_qr_panel_shows_warning_when_proxy_off` — 代理未启动时显示 disabled 状态（需 mock 状态）
- `cert_server_serves_ios_mobileconfig` — `fetch('http://localhost:19876/ios.mobileconfig')` → 200, Content-Type 含 `x-apple-aspen-config`
- `cert_server_serves_android_setup` — 同上 /android-setup → 200, Content-Type `text/html`
- `cert_server_serves_ca_unchanged` — /ca.crt 仍返回原 PEM（回归）

### 8.3 手动真机测试（用户执行，不在 CI）

- **iOS 完整流程**：手机连 ProxyBot WiFi → 扫 iOS QR → Safari 打开 → 装描述文件 → 设置 → 通用 → 关于 → 证书信任设置 启用 ProxyBot CA → 打开微信 → ProxyBot UI 流量列表出现 `💬 WeChat` 标签请求
- **Android 完整流程**：连 ProxyBot WiFi → 扫 Android QR → 按 4 步设置 → 装 CA → 打开微信（部分 App 会被拒，预期）
- **错误路径测试**：代理未启动时打开 Setup 页 → QR 面板 disabled → tooltip 显示提示

## 9. References

- 现有 QR 基础设施：`src-tauri/src/cert/qr.rs` (QrGenerator, 已能生成 CA 的 QR)
- 现有 CertServer：`src-tauri/src/cert_server.rs` (port 19876, 当前仅 `/ca.crt`)
- 现有网络信息：`src-tauri/src/network/mod.rs:47` (`NetworkInfo { lan_ip, interface }`)
- 现有客户端指引：`src-tauri/src/commands/client_setup.rs` (文本指引, 本次不直接复用, 改用 QR)
- 竞品参考：https://github.com/danieldev23/trafexia (IosBridgeDialog.vue, AndroidBridgeDialog.vue)
- Apple 官方 mobileconfig 文档：https://developer.apple.com/documentation/devicemanagement
- Android 用户 CA 限制：https://developer.android.com/training/articles/security-config
