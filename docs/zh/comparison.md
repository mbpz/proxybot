# 功能对比

ProxyBot 与其他流行的 MITM 代理工具对比。

## 功能对比

| 功能 | ProxyBot | mitmproxy | Proxyman | proxelar | anything-analyzer |
|--------|:--------:|:---------:|:--------:|:--------:|:-----------------:|
| TUI | - | mitmweb | - | ratatui | - |
| GUI | Tauri+React | mitmweb | macOS 原生 | Web (axum) | Electron |
| pf 透明代理 | macOS pf | - | - | - | - |
| 应用分类 | DNS 关联 | - | - | - | - |
| DNS 服务器 | 内置 DoH/UDP | - | - | - | - |
| 脚本引擎 | Rhai | Python | Scripting | Lua | JS Hook |
| AI 分析 | Gen tab | - | - | - | 5模式 AI |
| MCP Server | - | - | - | - | Client+Server |
| macOS 支持 | ✓ | ✓ | ✓ | ✓ | ✓ |
| Windows 支持 | - | ✓ | ✓ | ✓ | ✓ |
| Linux 支持 | - | ✓ | ✓ | ✓ | ✓ |
| 规则引擎 | ✓ (5动作) | ✓ | ✓ | Lua only | - |
| 断点调试 | ✓ | ✓ | ✓ | Lua | - |
| HAR 导出 | ✓ | ✓ | ✓ | - | - |
| Mock 生成 | ✓ Gen tab | - | Map Local | Lua | - |
| 过滤 DSL | ✓ AND/OR/NOT | ✓ Flow | ✓ | column:value | 域名过滤 |
| 代码导出 | ✓ cURL/fetch/Python/Go | ✓ | ✓ | - | - |

## ProxyBot 优势

1. **透明代理** — 无需逐应用配置代理。只需将手机的网关设置为 Mac，即可捕获所有流量。

2. **应用分类** — 通过将 DNS 查询与观察到的流量进行关联，按应用（微信、抖音、支付宝等）对请求进行分组。

3. **Rust + Tauri GUI 双界面** — 终端效率 + 桌面体验，竞品中独一无二。

4. **内置 DNS 服务器** — DoH + UDP，支持 hosts、阻止列表和查询日志用于 App 关联。

## proxelar (966 ⭐) — 最接近的 Rust 竞品

proxelar 是架构最相似的开源项目（Rust + MITM + 脚本）。

**proxelar 优势:**
1. **Lua 脚本** — 更成熟的生态，更多可用脚本
2. **列作用域过滤** — `method:POST host:api status:200` 语法直观
3. **三界面** — Terminal + TUI + Web GUI 单二进制
4. **跨平台** — macOS, Linux, Windows
5. **简易 CA 安装** — 通过代理访问 `http://proxel.ar`

**ProxyBot 相对 proxelar 的优势:**
1. **pf 透明代理** — proxelar 需手动配置代理
2. **应用分类** — DNS 关联 + SNI + 域名规则
3. **内置 DNS 服务器** — DoH + UDP 带日志
4. **规则引擎** — YAML 5 动作规则 vs 仅 Lua
5. **Gen tab** — Mock API / 前端脚手架 / Docker 生成
6. **DAG 分析** — 请求依赖图和认证状态机
7. **Tauri GUI** — 原生桌面体验 vs Web GUI
8. **设备管理** — 逐设备追踪和策略覆盖

## anything-analyzer (2,366 ⭐) — AI 优先竞品

anything-analyzer 是首个将 AI 分析作为一等公民的抓包工具。

**anything-analyzer 优势:**
1. **AI 自动分析** — 5 种分析模式（API 逆向、安全审计、性能、JS 加密）
2. **MCP Server** — 将代理能力暴露为 MCP 工具供 AI Agent 使用
3. **JS Hook 注入** — crypto.subtle、CryptoJS、SM2/3/4 拦截
4. **双通道捕获** — 浏览器 CDP + MITM 代理统一会话
5. **内嵌浏览器** — 多标签 Chromium

**ProxyBot 相对 anything-analyzer 的优势:**
1. **Rust 性能** — Electron 内存/CPU 开销大
2. **pf 透明代理** — 零配置移动端抓包
3. **应用分类** — 自动 App 识别
4. **原生桌面 GUI** — Tauri 更轻量
5. **断点编辑** — 请求/响应修改（anything-analyzer 只读）
6. **Rhai 脚本** — 服务端流量转换
7. **Gen tab** — Mock 生成 + Docker 打包

## mitmproxy 优势

1. **跨平台** — 支持 macOS、Windows 和 Linux。
2. **更加成熟** — 更长的开发历史，更多文档。
3. **可脚本化** — Python 脚本支持复杂用例。
4. **插件生态** — 最大的插件社区。

## Proxyman 优势

1. **原生 macOS 应用** — 简洁的原生 UI。
2. **CA 安装便捷** — 一键在 iOS 上安装证书。
3. **本地映射** — 便于开发过程中模拟 API 响应。
4. **Atlantis** — iOS VPN 无代理抓包。

## InterceptSuite 优势

1. **传输层聚焦** — TCP/UDP/DTLS/TLS，不仅是 HTTP
2. **IoT/Thick Client** — 非 Web 协议专用
3. **Python 扩展** — 自定义协议解析器
4. **PCAP 导出** — 兼容 Wireshark 分析
