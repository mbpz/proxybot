# ProxyBot

A macOS HTTPS MITM proxy tool for developers. Phone and PC on the same LAN — phone sets gateway/DNS to PC IP, and ProxyBot captures + decrypts all HTTPS/WSS traffic. Traffic is classified by app (WeChat, Douyin, Alipay), then by domain within each app.

## Features

- **Transparent HTTPS/WSS interception** with MITM SSL
- **App classification** by DNS correlation + domain rules (WeChat, Douyin, Alipay)
- **Built-in DNS server** to log phone's DNS queries
- **macOS pf integration** for transparent proxy routing
- **9-tab full-featured TUI** for terminal-based monitoring and control

## TUI Tabs

The Terminal UI provides 9 functional tabs:

| Tab | Features |
|-----|----------|
| **Traffic** | Filter by method/host/status/app_tag, regex search, split pane with request detail, pf/DNS controls |
| **Rules** | Rule table with action badges (DIRECT/PROXY/REJECT), inline modal editor, hot-reload status |
| **Devices** | Device table with MAC/last_seen/bytes up-down, per-device rule override |
| **Certs** | CA fingerprint/expiry/serial, regenerate CA, export PEM to `~/.proxybot/ca.crt` |
| **DNS** | Upstream selector (Plain/DoH), blocklist toggle, hosts entries, live query log |
| **Alerts** | Anomaly alerts with severity badges (SEV1/2/3), ACK/clear controls, baseline stats |
| **Replay** | Replay targets table, start/stop replay, HAR export, diff view |
| **Graph** | ASCII DAG visualization, auth state machine detection |
| **Gen** | Mock API / frontend scaffold / Docker bundle generation |

### TUI Keyboard Shortcuts

```
Navigation: Tab / h,l / ←,→  (switch tabs)
             j,k / ↑,↓        (navigate lists)
General:     q / Esc           (quit)
             r                  (start proxy)
             S                  (stop proxy)
             c                  (clear)

Traffic Tab: p  (toggle pf transparent proxy)
             n  (toggle DNS server)
             /  (focus search)
             x  (clear filters)
             Enter (load request detail)
             1/2/3  (switch detail sub-tab: Headers/Body/WS Frames)
             m  (filter by method)
             f  (filter by host)
             o  (filter by status)
             a  (filter by app_tag)

Rules Tab:   a  (add rule)
             e  (edit rule)
             d  (delete rule)
             s  (save rule)
             Alt+↑/↓  (reorder rule)

Certs Tab:   r  (regenerate CA)
             e  (export PEM)

DNS Tab:     s  (toggle DNS server)
             b  (toggle blocklist)
             u  (cycle upstream)

Alerts Tab:  a  (acknowledge selected alert)
             c  (clear all acknowledged alerts)

Replay Tab:  s  (start replay)
             x  (stop replay)
             e  (export HAR)
             d  (show diff)

Graph Tab:   g / a  (toggle DAG/Auth state machine view)
             r  (refresh graph)

Gen Tab:     m  (generate mock API)
             f  (generate frontend scaffold)
             d  (generate Docker bundle)
             o  (open output folder)
```

## Installation

### Option A: Desktop GUI App

#### Step 1: Download and Install GUI

1. Download `proxybot_0.1.0_aarch64.dmg` from the release page
2. Double-click the `.dmg` file to mount it
3. Drag `ProxyBot.app` to your Applications folder

#### Step 2: Allow the App to Run

Since the app is not signed with a Developer ID, you may need to bypass the gatekeeper:

```bash
xattr -cr /Applications/ProxyBot.app
```

Or right-click the app and select "Open" > "Open" to confirm you want to run it.

#### Step 3: Install iOS CA Certificate

ProxyBot uses a self-signed root CA to decrypt HTTPS traffic. You must install this CA on your iOS device:

1. Open ProxyBot on your Mac
2. Go to **Settings** and click **Export CA Certificate**
3. AirDrop the certificate file to your iPhone, or email it to yourself
4. On your iPhone:
   - Save the attachment
   - Go to **Settings > General > About > Certificate Trust Settings**
   - Enable full trust for "ProxyBot Root CA"
5. For Safari to work properly with HTTPS interception, you may need to visit `http://proxybot.ca` once to download and install the CA profile if prompted

#### Step 4: Configure Your iPhone

Your iPhone must be on the same LAN as your Mac and use your Mac as the network gateway:

1. Go to **Settings > Wi-Fi > Your Network > Configure Proxy**
2. Select **Manual**
3. Set **Server** to your Mac's IP address (shown in ProxyBot's status panel)
4. Set **Port** to `8088`
5. Go to **Settings > Wi-Fi > Your Network > Configure DNS**
6. Set DNS to your Mac's IP address (same as the proxy server)

> **Tip:** Find your Mac's IP address by running `ipconfig getifaddr en0` in Terminal, or look at the top of ProxyBot's main window.

### Step 5: Start Capturing

1. ProxyBot requires admin privileges on first launch to set up the pf transparent proxy. Enter your Mac password when prompted.
2. Click **Start Proxy** in ProxyBot's main window
3. Traffic from your iPhone will now appear in the dashboard, classified by app and domain

### Option B: Terminal UI (TUI)

ProxyBot ships as a standalone terminal UI binary with no GUI dependency.

#### Install

```bash
brew install mbpz/proxybot/proxybot-tui
```

#### Update

```bash
brew upgrade proxybot-tui
```

#### Uninstall

```bash
brew uninstall proxybot-tui
```

> **Note:** Homebrew auto-detects your Mac's CPU architecture (arm64 / x86_64) and installs the correct binary.

### Option C: Build from Source

```bash
git clone https://github.com/mbpz/proxybot.git
cd proxybot/src-tauri
cargo build --bin proxybot-tui --release
./target/release/proxybot-tui
```

## Architecture

### TUI Module Structure

```
src-tauri/src/tui/
  mod.rs              # TuiApp state, Tab enum, TrafficState
  input.rs            # Key event handling, format_ts, fmt_duration
  render/
    mod.rs            # Tab dispatcher, tab bar, status bar
    traffic.rs        # Traffic tab: filters, list, detail pane
    rules.rs          # Rules tab: table, action badges, modal editor
    devices.rs        # Devices tab: device table with stats
    certs.rs          # Certs tab: CA info, regenerate, export
    dns.rs            # DNS tab: upstream, blocklist, query log
    alerts.rs         # Alerts tab: severity badges, ACK/clear
    replay.rs         # Replay tab: targets, HAR export, diff
    graph.rs          # Graph tab: ASCII DAG, auth state machine
    gen.rs           # Gen tab: mock/frontend/docker generation
```

### Key Dependencies

- **ratatui 0.26** — Terminal UI rendering
- **crossterm 0.26** — Cross-platform terminal handling
- **rusqlite** — Local database for request storage
- **tokio** — Async runtime for proxy and DNS servers
- **rustls** — TLS/MITM certificate handling

## FAQ

### WeChat shows "Certificate Error" or connections fail

WeChat uses certificate pinning and may reject the intercepted certificate. This is expected behavior for apps with strong SSL pinning. The traffic will still be logged at the DNS and SNI level for classification purposes, but full HTTPS content inspection may not work for these apps.

### The proxy starts but I see no traffic

1. Verify your iPhone is using your Mac as the proxy and DNS server
2. Check that both devices are on the same LAN
3. Try opening `http://example.com` in Safari on the iPhone (not HTTPS)
4. Check ProxyBot's DNS tab to see if DNS queries are being received

### How does ProxyBot classify traffic?

ProxyBot uses a multi-stage classification pipeline:
1. **DNS correlation** — When your phone makes a DNS query, ProxyBot logs it. The subsequent connection is tagged with the app that made the DNS request.
2. **SNI inspection** — The TLS ClientHello message contains the requested domain (SNI), which ProxyBot extracts before encryption.
3. **Domain rules** — Known app domains are mapped: WeChat (`*.weixin.qq.com`, `*.wechat.com`), Douyin (`*.douyin.com`, `*.tiktokv.com`), Alipay (`*.alipay.com`).

### Does ProxyBot work on Windows?

Not yet. Windows support is planned for Phase 2.

### How do I uninstall?

**GUI App:**
1. Stop the proxy in ProxyBot
2. Remove the app: `rm -rf /Applications/ProxyBot.app`
3. Optionally remove the CA from your iPhone: Settings > General > About > Certificate Trust Settings
4. Optionally remove config data: `rm -rf ~/Library/Application\ Support/com.proxybot.app/`

**TUI:**
```bash
brew uninstall proxybot-tui
```

## Development

### Running Tests

```bash
cd src-tauri
cargo test                 # 153+ unit + integration tests
cargo build --bin proxybot-tui --release
```

### TUI Architecture

The TUI is built with a state-driven architecture:

- `TuiApp` holds all subsystem handles (`db_state`, `cert_manager`, `rules_engine`, etc.) as `Arc`
- Each tab has its own state struct (`TrafficState`, `RulesState`, etc.)
- Input handling is centralized in `input.rs` via `handle_key_event()`
- Rendering is dispatched per-tab in `render/mod.rs`
- The main loop in `proxybot-tui.rs` handles input + rendering

### Key Design Patterns

- **Context-sensitive keys**: Same key does different things on different tabs (e.g., `r` = StartProxy on Traffic, RegenerateCert on Certs)
- **Split pane**: Traffic tab uses 60/40 split between request list and detail panel
- **Hot reload**: Rules engine watches rule files and reloads automatically
- **Async event channel**: `broadcast::Receiver<InterceptedRequest>` for real-time request updates
