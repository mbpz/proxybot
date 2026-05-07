# Comparison

ProxyBot vs other popular MITM proxy tools.

## Feature Comparison

| Feature | ProxyBot | mitmproxy | Proxyman |
|---------|:--------:|:---------:|:--------:|
| TUI | ratatui | mitmweb | - |
| pf Transparent Proxy | macOS pf | - | - |
| App Classification | DNS correlation | - | - |
| Tauri GUI (Rust) | Yew WASM | - | - |
| ADB Reverse Tunnel | Android USB | - | - |
| macOS Support | ✓ | ✓ | ✓ |
| Windows Support | - | ✓ | ✓ |
| Linux Support | - | ✓ | ✓ |
| Rule Engine | ✓ | ✓ | ✓ |
| Breakpoint Debugging | ✓ | ✓ | ✓ |

## ProxyBot Advantages

1. **Transparent proxy** — No per-app configuration needed. Just set the phone's gateway to your Mac and all traffic is captured.

2. **App classification** — Correlates DNS queries with observed traffic to group requests by app (WeChat, Douyin, Alipay, etc.).

3. **Rust-based GUI** — Yew WASM GUI requires no Node.js or npm.

## mitmproxy Advantages

1. **Cross-platform** — Works on macOS, Windows, and Linux.
2. **More mature** — Longer development history, more documentation.
3. **Scriptable** — Python scripting for complex use cases.

## Proxyman Advantages

1. **Native macOS app** — Clean native UI.
2. **Easy CA install** — One-click certificate installation on iOS.
3. **Map Local** — Useful for mocking API responses during development.