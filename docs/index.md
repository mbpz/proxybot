# ProxyBot

**macOS HTTPS MITM proxy for developers**

<div class="badges" style="text-align: center; margin: 30px 0;">
[![macOS](https://img.shields.io/badge/macOS-✓-blue?style=for-the-badge)](https://github.com/mbpz/proxybot)
[![Rust](https://img.shields.io/badge/Rust-✓-orange?style=for-the-badge)](https://github.com/mbpz/proxybot)
[![TUI + GUI](https://img.shields.io/badge/TUI+GUI-✓-green?style=for-the-badge)](https://github.com/mbpz/proxybot)
</div>

## Features

<div class="grid cards" style="grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: 20px;">

- **Transparent Proxy** — No per-app proxy configuration. macOS `pf` redirects all traffic from your phone automatically.

- **App Classification** — Traffic automatically grouped by app: WeChat, Douyin, Alipay, and more.

- **Full HTTPS Decryption** — MITM with dynamically generated certificates. See encrypted traffic in plain text.

- **Keyboard-Driven TUI** — Developer-friendly terminal UI. All 9 tabs accessible without a mouse.

- **Yew GUI (Beta)** — New desktop GUI built with Rust + WebAssembly. No Node.js dependency.

- **Rule Engine** — Direct, Proxy, Reject, MapRemote, MapLocal — full control over traffic.

</div>

## Installation

```bash
brew install mbpz/proxybot/proxybot-tui
```

### Setup Steps

1. Connect your phone to the same WiFi network as your Mac
2. Set your phone's gateway and DNS to your Mac's IP address
3. Export CA certificate from the **Certs** tab → AirDrop to your phone
4. Trust the CA in **Settings → General → About → Certificate Trust Settings**
5. Run `proxybot-tui` and press `r` to start proxying

Find your Mac's IP address:

```bash
ipconfig getifaddr en0
```

## Architecture

```
Phone --[WiFi]--> Mac (pf redirect :80/:443) --> ProxyBot (MITM) --> Internet
                                                            |
                                                            +--> DNS Server (log queries, correlate with apps)
```

## Comparison

| Feature | ProxyBot | mitmproxy | Proxyman |
|---------|:--------:|:---------:|:--------:|
| TUI | ratatui | mitmweb | - |
| pf Transparent Proxy | macOS pf | - | - |
| App Classification | DNS correlation | - | - |
| Tauri GUI (Rust) | Yew WASM | - | - |
| ADB Reverse Tunnel | Android USB | - | - |

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `r` | Start/Stop Proxy |
| `p` | Toggle pf |
| `n` | Toggle DNS |
| `/` | Search |
| `x` | Clear filters |
| `q` | Quit |

## Download

[Download for macOS](https://github.com/mbpz/proxybot/releases){ .md-button .md-button--primary }