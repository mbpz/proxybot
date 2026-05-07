# 功能对比

ProxyBot 与其他流行的 MITM 代理工具对比。

## 功能对比

| 功能 | ProxyBot | mitmproxy | Proxyman |
|--------|:--------:|:---------:|:--------:|
| TUI | ratatui | mitmweb | - |
| pf 透明代理 | macOS pf | - | - |
| 应用分类 | DNS 关联 | - | - |
| Tauri GUI (Rust) | Yew WASM | - | - |
| ADB 反向隧道 | Android USB | - | - |
| macOS 支持 | ✓ | ✓ | ✓ |
| Windows 支持 | - | ✓ | ✓ |
| Linux 支持 | - | ✓ | ✓ |
| 规则引擎 | ✓ | ✓ | ✓ |
| 断点调试 | ✓ | ✓ | ✓ |

## ProxyBot 优势

1. **透明代理** — 无需逐应用配置代理。只需将手机的网关设置为 Mac，即可捕获所有流量。

2. **应用分类** — 通过将 DNS 查询与观察到的流量进行关联，按应用（微信、抖音、支付宝等）对请求进行分组。

3. **基于 Rust 的 GUI** — Yew WASM GUI 无需 Node.js 或 npm。

## mitmproxy 优势

1. **跨平台** — 支持 macOS、Windows 和 Linux。
2. **更加成熟** — 更长的开发历史，更多文档。
3. **可脚本化** — Python 脚本支持复杂用例。

## Proxyman 优势

1. **原生 macOS 应用** — 简洁的原生 UI。
2. **CA 安装便捷** — 一键在 iOS 上安装证书。
3. **本地映射** — 便于开发过程中模拟 API 响应。
