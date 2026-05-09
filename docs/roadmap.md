# ProxyBot Roadmap

## 1. Hero Section

**ProxyBot — macOS HTTPS MITM proxy for developers**

ProxyBot captures and decrypts all HTTPS/WSS traffic from your phone via macOS pf transparent proxy. Set your phone's gateway and DNS to your Mac's IP, install the CA certificate once, and watch every request flow through — classified by app (WeChat, Douyin, Alipay) and domain.

**Demo concept:** Phone on the left, Mac running the TUI on the right. Traffic appears in real-time as you use apps on the phone.

---

## 2. What's Shipped (v0.4.x)

Nine functional tabs, all accessible from the keyboard-driven TUI:

### Traffic

Real-time request list with method/host/status/app filter. Regex search. 60/40 split between request list and detail panel. pf/DNS toggle controls.

**Shortcut:** `p` toggle pf, `n` toggle DNS, `/` focus search, `x` clear filters, `Enter` load detail

### Rules

Five action types: **Direct** (bypass proxy), **Proxy** (forward to upstream), **Reject** (drop connection), **MapRemote** (forward to custom remote), **MapLocal** (serve from local file/mock). Hot-reload on file change. Rule table with inline modal editor.

**Shortcut:** `a` add, `e` edit, `d` delete, `s` save

### Devices

Per-device table showing MAC address, last seen, and bytes up/down. Per-device rule override — enter edit mode on any device to assign a custom rule action. WeChat/Douyin/Alipay classification badge per device.

**Shortcut:** `e` edit rule override, `Enter` confirm, `Esc` cancel

### Certs

One-click CA certificate export to `~/.proxybot/ca.crt`. Shows fingerprint, expiry, and serial number. Regenerate CA with fresh key pair. AirDrop or email the certificate to your phone.

**Shortcut:** `r` regenerate CA, `e` export PEM

### DNS

Upstream resolver selector: plain UDP or DoH (DNS-over-HTTPS). Blocklist toggle. Hosts file entries with lock. Live query log showing recent lookups with response latency.

**Shortcut:** `s` toggle DNS server, `b` toggle blocklist, `u` cycle upstream

### Alerts

SEV1 (critical), SEV2 (warning), SEV3 (info) anomaly detection with baseline profiling. Alert table with source, description, severity badge. ACK/clear controls. Baseline stats show normal traffic patterns.

**Shortcut:** `a` acknowledge, `c` clear acknowledged

### Replay

Replay targets table with start/stop controls. HAR export of captured traffic. Diff view comparing replayed response against original — highlights header and body differences.

**Shortcut:** `s` start replay, `x` stop replay, `e` export HAR, `d` show diff

### Graph

ASCII DAG visualization of request dependency graph (domains, timing, status codes). Auth state machine detection — identifies login sequences and session token flows. Toggle between DAG and auth views.

**Shortcut:** `g` DAG view, `a` auth state machine, `r` refresh

### Gen

Mock API generation from captured traffic. Frontend scaffold generator (React + TypeScript boilerplate). Docker bundle generator — packages mock backend with Dockerfile and docker-compose. Open output folder directly.

**Shortcut:** `m` generate mock API, `f` generate frontend scaffold, `d` generate Docker, `o` open output

---

## 3. Competitive Comparison

| | ProxyBot | mitmproxy | Proxyman | HTTP Toolkit |
|--|--|--|--|--|
| TUI | macOS-native ratatui | NCurses/mitmweb | Mac GUI only | CLI + web UI |
| App classification | WeChat/Douyin/Alipay | — | — | — |
| pf transparent proxy | macOS pf integration | Manual proxy config | Mac GUI proxy | — |
| Breakpoint | ✅ Full | Full | Full | Full |
| Auto CA install | Wizard guide | Manual | One-click | One-click |
| Tauri GUI | ✅ Alpha | — | Mac GUI | — |
| System tray | ✅ With notifications | — | ✅ | — |
| ADB reverse tunnel | ✅ Android USB | — | — | — |

**ProxyBot's edge:** pf transparent proxy means no per-app proxy configuration on the phone. App classification groups traffic automatically. TUI is first-class on macOS.

---

## 4. Roadmap (Milestones)

| Version | Focus | Features |
|---------|-------|----------|
| **v0.4.x (DONE)** | TUI complete | All 9 tabs shipped, pf + DNS, basic breakpoint intercept |
| **v0.5.0 (DONE)** | Breakpoint Editing | Full TUI breakpoint UI — pause, edit request/response, continue. Android adb reverse support via USB |
| **v0.6.0 (DONE)** | Tauri GUI Alpha | React UI traffic panel, proxybot-gui binary, CA wizard, system tray with notifications |
| **v0.7.0 (DONE)** | Rules Engine | MapRemote/MapLocal/Respond rules integrated into handle_http pipeline. apply_request_rule() sync rule engine with hot-reload |
| **v0.8.0 (DONE)** | Tauri GUI Complete | Full GUI: Rules editor, Devices management, Certs UI, complete parity with TUI |
| **v0.9.0 (DONE)** | Advanced Features | Filter DSL (AND/OR/NOT/glob), WS frame viewer (text/hex), Replay engine (reqwest), TLS fingerprint classifier (6 apps), Dependency graph (DAG/waterfall/auth), Traffic list (virtual scroll) |
| **v0.10.0 (NEXT)** | Quick Wins | Code export (cURL/fetch/Python/Go), Request Composer, JSON/XML syntax highlight, One-click client setup |
| **v1.0.0** | Phase 2 Complete | Plugin system, Rhai scripting, gRPC/Protobuf, Network conditions, iOS VPN, Team collaboration |

## 3.1 Competitive Deep-Dive (May 2026)

Researched 6 comparable projects for architecture, interaction, and product insights.

| Project | Stars | Lang | UI | Key Differentiator |
|---------|-------|------|----|--------------------|
| [mitmproxy](https://github.com/mitmproxy/mitmproxy) | 43k | Python | TUI+Web | Industry standard, addon system, Python scripting |
| [whistle](https://github.com/avwo/whistle) | 15.5k | Node.js | Web UI | Plugin ecosystem, Weinre remote debugging, Composer |
| [bandwhich](https://github.com/imsnif/bandwhich) | 11.7k | Rust | TUI | Process-level bandwidth attribution |
| [Proxyman](https://github.com/ProxymanApp/Proxyman) | 6.8k | Swift | macOS native | SwiftNIO performance, iOS app, team workspace |
| [HTTP Toolkit](https://github.com/httptoolkit/httptoolkit) | 3.5k | TS+React | Electron | Modular (mockttp lib), one-click client setup |
| [proxy.py](https://github.com/abhinavsingh/proxy.py) | 3.5k | Python | CLI | Zero-dependency, plugin framework, GROUT tunnel |

### Feature Gap Analysis

| Feature | ProxyBot | mitmproxy | Proxyman | HTTP Toolkit | whistle | proxy.py |
|---------|----------|-----------|----------|--------------|---------|----------|
| Transparent proxy (pf) | ✅ | — | — | — | — | — |
| App classification | ✅ | — | — | — | — | — |
| TUI | ✅ ratatui | ✅ NCurses | — | — | — | — |
| Tauri GUI | ✅ | — | — | — | — | — |
| ADB tunnel | ✅ | — | — | — | — | — |
| Plugin system | — | ✅ addons | — | — | ✅ plugins | ✅ plugins |
| Scripting hooks | — | ✅ Python | ✅ Scripting | — | ✅ rules | ✅ plugins |
| Request composer | — | — | ✅ Compose | ✅ Send | ✅ Composer | — |
| Diff tool | — | — | ✅ | — | — | — |
| DNS spoofing | — | — | ✅ | — | — | ✅ |
| Network throttling | — | — | ✅ | ✅ | — | — |
| Protobuf/gRPC | — | ✅ | ✅ | — | — | — |
| gRPC-web/WebSocket frame | ✅ WS | ✅ | ✅ | ✅ | ✅ | — |
| HAR export | ✅ | ✅ | ✅ | ✅ | ✅ | — |
| Mock API generation | ✅ Gen tab | — | Map Local | ✅ mockttp | ✅ | ✅ |
| Remote debugging | — | — | — | — | ✅ Weinre | — |
| Tunnel (ngrok alt) | — | — | — | — | — | ✅ GROUT |
| Team workspace | — | — | ✅ | — | — | — |
| API diff / regression | — | — | — | — | — | — |
| Code export (cURL/fetch) | — | ✅ | ✅ | ✅ | ✅ | — |
| One-click client setup | — | ✅ | ✅ | ✅ | — | — |
| iOS standalone app | — | — | ✅ | — | — | — |

### Key Improvements for ProxyBot

**Priority 1 — Quick Wins (v0.10.0)**

1. **cURL/fetch code export** — Export any captured request from detail panel
   - Formats: cURL, fetch(), Python requests, Go http
   - UI: "Copy as cURL" button + format dropdown in RequestDetail
   - Files: `src/components/shared/CodeExport.tsx`, `src-tauri/src/commands/code_export.rs`

2. **Request Composer** — Edit-and-resend single request
   - Split view: left=original request, right=modified response preview
   - Method/URL/Headers/Body form editor (reuse ReplayModal patterns)
   - Live preview of response status/duration/body
   - Files: `src/components/composer/ComposerPage.tsx`, `src/components/composer/ComposerEditor.tsx`

3. **JSON/XML syntax highlighting** — BodyView with code highlighting
   - Use shiki (same engine as VSCode) for syntax highlighting
   - Auto-detect content type: JSON, XML, HTML, JavaScript, CSS
   - Line numbers, folding, search within body
   - Files: `src/components/shared/CodeViewer.tsx` (replace BodyView)

4. **One-click client setup** — Auto-configure proxy for browsers/Node.js
   - Detect installed browsers (Chrome, Firefox, Safari, Edge)
   - Generate proxy PAC file or set system proxy flags
   - Node.js: set http_proxy/https_proxy env vars
   - Python: export REQUESTS_CA_BUNDLE pointing to ProxyBot CA
   - Files: `src-tauri/src/commands/client_setup.rs`, `src/components/setup/ClientSetup.tsx`

**Priority 2 — Architecture Upgrades (v0.11.0)**

5. **Plugin system** — Rust trait-based plugin API
   - `ProxyPlugin` trait: on_request, on_response, on_connect, on_error hooks
   - Plugin discovery: scan `~/.proxybot/plugins/` for .wasm or .so files
   - Hot-reload: watch plugin directory for changes
   - Sandbox: WASM runtime for untrusted plugins
   - Files: `src-tauri/src/plugin/` (loader, registry, sandbox, wasm_runtime)
   - API surface: read/modify headers, read body, inject response, log

6. **Scripting hooks** — Rhai scripting engine
   - Rhai (Rust-native, no unsafe) for user-defined traffic transforms
   - Editor UI with syntax highlighting and live validation
   - Hook points: request received, response received, before forward
   - Script API: `request.headers`, `request.body`, `response.status`, `ctx.log()`
   - Files: `src-tauri/src/scripting/` (engine, api, sandbox), `src/components/scripts/ScriptEditor.tsx`

7. **gRPC/Protobuf support** — Decode protobuf bodies
   - Detect `content-type: application/grpc` and `application/x-protobuf`
   - Protobuf descriptor discovery: upload .proto or auto-detect via reflection
   - Decode protobuf to JSON display, show field names and types
   - gRPC-Web support (base64 + binary frames)
   - Files: `src-tauri/src/protobuf/` (decoder, descriptor, grpc_web)

8. **Network condition simulation** — Throttle + latency + packet loss
   - Presets: 3G (1.6Mbps/768kbps/300ms), 4G (20Mbps/10Mbps/100ms), Custom
   - Per-host or global throttle rules
   - Buffered delay with configurable jitter
   - Visual indicator in status bar when throttling active
   - Files: `src-tauri/src/network_conditions.rs`, `src/components/conditions/NetworkConditions.tsx`

**Priority 3 — Product Expansion (v1.0.0+)**

9. **Process-level attribution** — Which app sent this request
   - ADB: run `ps` on device, map PID→package name
   - iOS: use NEPacketTunnel flow metadata (iOS 15+)
   - Show app icon + name in traffic list (AppBadge component already exists)
   - Per-app traffic statistics: bytes, request count, latency distribution

10. **Team collaboration** — Share proxy configurations
    - Export/import: HAR, ProxyBot project file, CA certificate bundle
    - Rule sets: share MapRemote/MapLocal configurations as YAML/JSON
    - Mock configs: export generated mocks with Docker bundle

11. **iOS VPN mode** — On-device capture
    - NEPacketTunnel provider (Swift, packaged separately)
    - Local VPN → forward to local proxy → MITM on device
    - No Mac required for basic capture
    - Certificate install via MDM profile

---

## 4.1 Enhanced Detection (from rkn-block-checker analysis)

Inspired by [rkn-block-checker](https://github.com/MayersScott/rkn-block-checker) architecture for deep censorship detection:

### Multi-Layer Diagnosis System

```
check_url() progressive detection:
├── DNS layer: system DNS vs DoH comparison → detect DNS poisoning
├── TCP layer: port connectivity → detect IP blacklist/TCP reset
├── TLS layer: SNI handshake → detect TLS DPI (SNI filtering)
└── HTTP layer: status code + body signature → detect HTTP stub pages (451, ISP inject)
```

### Verdict + Confidence System

| Verdict | Description | Confidence Calibration |
|---------|-------------|------------------------|
| `OK` | Connection normal | HIGH if all layers pass |
| `DNS_BLOCK` | System DNS fails, DoH succeeds | HIGH - dual signal confirmation |
| `TCP_RESET` | TCP RST received | MEDIUM - known censorship pattern |
| `TLS_BLOCK` | TLS handshake dropped on ClientHello | MEDIUM - SNI-based DPI signature |
| `HTTP_STUB` | HTTP 451 or ISP stub page marker | HIGH - explicit signal |
| `TIMEOUT` | Connection timeout | LOW - ambiguous |
| `DOWN` | Generic failure | LOW - multiple possible causes |

### Stub Page Detection

```python
STUB_MARKERS = [...]  # ISP signature strings for body matching

def looks_like_stub(body_snippet: str) -> bool:
    return any(marker in body_snippet for marker in STUB_MARKERS)
```

### DNS Comparison (ProxyBot already has foundation)

```rust
// System DNS vs DoH对比检测DNS污染
sys_ips = resolve_system_all(host)
doh_ips = resolve_doh_all(host)

if sys_ips.is_disjoint(doh_ips) {
    → DNS mismatch detected (transparent DNS rewriting)
}
```

### Implementation Priority

1. **Stub page signatures** - Add ISP stub markers to response analyzer
2. **Confidence scoring** - Add Verdict + Confidence to anomaly alerts
3. **DNS对比检测** - Leverage existing DNS log for poisoning detection
4. **TLS DPI detection** - Detect握手被重置的审查模式
5. **流式诊断报告** - 分层展示诊断结果

---

## 5. Installation

```bash
brew install mbpz/proxybot/proxybot-tui
```

Then connect your phone to the same WiFi network as your Mac:

1. **Set gateway:** WiFi settings > Configure Proxy > Manual — set Server to your Mac IP, Port to `8088`
2. **Set DNS:** WiFi settings > Configure DNS — set to your Mac IP
3. **Install CA:** Export cert from Certs tab, AirDrop to phone, enable full trust in Settings > General > About > Certificate Trust Settings
4. **Start capturing:** Run `proxybot-tui`, press `r` to start the proxy

Find your Mac IP with: `ipconfig getifaddr en0`

---

## 6. Architecture

```
Phone --[WiFi]--> Mac (pf redirect :80/:443) --> ProxyBot (MITM) --> Internet
                        |
                        +--> DNS Server (log queries, correlate with apps)
```

- **pf** redirects all port 80/443 traffic from the phone to ProxyBot's local proxy port
- **MITM** terminates TLS with dynamically-generated leaf certs signed by the root CA
- **DNS server** logs queries from the phone, correlated with subsequent connections for app classification
- **Classification engine** maps domains to apps: WeChat (`*.weixin.qq.com`, `*.wechat.com`), Douyin (`*.douyin.com`, `*.tiktokv.com`), Alipay (`*.alipay.com`, `*.alipayusercontent.com`)
