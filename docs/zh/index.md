# ProxyBot

**面向开发者的 macOS HTTPS MITM 代理**

<div class="badges" style="text-align: center; margin: 30px 0;">
[![macOS](https://img.shields.io/badge/macOS-✓-blue?style=for-the-badge)](https://github.com/mbpz/proxybot)
[![Rust](https://img.shields.io/badge/Rust-✓-orange?style=for-the-badge)](https://github.com/mbpz/proxybot)
[![Native GUI](https://img.shields.io/badge/Native GUI-✓-green?style=for-the-badge)](https://github.com/mbpz/proxybot)
</div>

## 功能特点

<div class="grid cards" style="grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: 20px;">

- **透明代理** — 无需逐应用配置代理。macOS `pf` 自动将手机所有流量重定向。

- **应用分类** — 流量按应用自动分组：微信、抖音、支付宝等。

- **完整 HTTPS 解密** — MITM + 动态生成证书。加密流量明文可见。

- **原生 macOS GUI** — Rust + Tauri 构建的桌面应用。无需 Node.js 依赖。

- **规则引擎** — 直连、代理、拒绝、远程映射、本地映射 — 完全控制流量。

</div>

## 安装

```bash
brew install --cask mbpz/tap/proxybot
```

### 设置步骤

1. 将手机连接到与 Mac 相同的 WiFi 网络
2. 将手机的网关和 DNS 设置为 Mac 的 IP 地址
3. 从 **证书** 标签页导出 CA 证书 → AirDrop 到手机
4. 在 **设置 → 通用 → 关于 → 证书信任设置** 中信任 CA
5. 启动 ProxyBot 并点击 **启动代理**

查看 Mac 的 IP 地址：

```bash
ipconfig getifaddr en0
```

## 架构

```
手机 --[WiFi]--> Mac (pf 重定向 :80/:443) --> ProxyBot (MITM) --> 互联网
                                                             |
                                                             +--> DNS 服务器（记录查询，关联应用）
```

## 对比

| 功能 | ProxyBot | mitmproxy | Proxyman |
|------|:--------:|:---------:|:--------:|
| TUI | ratatui | mitmweb | - |
| pf 透明代理 | macOS pf | - | - |
| 应用分类 | DNS 关联 | - | - |
| Tauri GUI (Rust) | Yew WASM | - | - |
| ADB 反向隧道 | Android USB | - | - |

## 键盘快捷键

| 快捷键 | 操作 |
|--------|------|
| `r` | 启动/停止代理 |
| `p` | 切换 pf |
| `n` | 切换 DNS |
| `/` | 搜索 |
| `x` | 清除筛选 |
| `q` | 退出 |

## 下载

[下载 macOS 版本](https://github.com/mbpz/proxybot/releases){ .md-button .md-button--primary }