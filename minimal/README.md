# ProxyBot minimal (CLI + local web UI)

A second, deliberately small architecture for ProxyBot: **one binary**, one
local web UI, and two capture modes. This directory is independent from the
legacy Tauri app at the repository root (it has its own Cargo workspace).

```
phone --Wi-Fi HTTP proxy--> Mac:8088 ─┐
phone --Wi-Fi gateway-----> Mac:8089 ─┴--> single binary --> upstream
                                              ├─ MITM (hudsucker + rustls + rcgen)
                                              ├─ app classification (SNI + YAML rules)
                                              ├─ SQLite history
                                              └─ web UI  http://127.0.0.1:8899
```

## Two capture modes (user selects)

### Mode 1 — HTTP proxy (default, no admin)

```
iOS/Android ── HTTP / CONNECT ──▶ Mac :8088 (ProxyBot) ──▶ router ──▶ internet
   Wi-Fi HTTP proxy = <Mac IP>:8088                    │
                                                       └─ local Web UI 127.0.0.1:8899
```

- Set the device Wi-Fi HTTP proxy to `<Mac LAN IP>:8088`.
- No pf, no NAT, no IP forwarding, no administrator rights.
- Mac failure only affects proxied apps; the device can still browse normally.
- Covers HTTP/HTTPS from apps that honour the system proxy.

### Mode 2 — Side-router / gateway (admin, opt-in)

```
iOS/Android ── all IP traffic ──▶ Mac (gateway) ──▶ router ──▶ internet
   Wi-Fi gateway = <Mac IP>          │
   DNS stays on router               ├─ :80/:443  pf rdr ──▶ ProxyBot :8089 (MITM)
                                     └─ other traffic NAT ──▶ router
```

- Run `proxybot start --gateway` (asks for admin; rules are removed on Ctrl+C).
- Set the device Wi-Fi **gateway** to the Mac's LAN IP; keep DNS on the router.
- Enables IP forwarding, pf redirect for :80/:443, and NAT for everything else.
- Covers nearly all IP traffic; the Mac is a single point of failure while active.

| | Mode 1 · HTTP proxy | Mode 2 · Side-router |
| --- | --- | --- |
| Device setting | Wi-Fi HTTP proxy | Wi-Fi gateway |
| Mac privileges | none | admin (pf + sysctl) |
| Coverage | HTTP/HTTPS via system proxy | all IP traffic |
| Failure impact | only proxied apps | all device traffic |
| Best for | app dev, QA, API debugging | reverse engineering, full capture |

Both modes share the same CA, MITM engine, SQLite history, app rules, web UI and
HAR export; only the listener and the device-side setting differ.

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/mbpz/proxybot/main/minimal/install.sh | sh
```

The script downloads the matching macOS arm64/x64 release archive, verifies its
SHA-256, and installs `proxybot` to `~/.local/bin` (override with
`PROXYBOT_PREFIX=/usr/local/bin`). No Homebrew, Node, Python, or Rust toolchain
is required at install time.

Releases are built by the manual **Minimal CLI release** workflow
(`.github/workflows/minimal-release.yml`).

## Use

```bash
proxybot start              # explicit proxy (:8088) + local web UI (:8899), opens browser
proxybot start --gateway    # also installs the pf gateway (asks for admin, auto-removes on Ctrl+C)
proxybot gateway status
proxybot gateway uninstall
proxybot rules show
```

- **Explicit proxy** — set the device Wi-Fi HTTP proxy to `<Mac LAN IP>:8088`.
- **Transparent gateway** — with `--gateway`, set the device Wi-Fi **gateway** to
  the Mac's LAN IP. pf redirects :80/:443 into the local proxy, other traffic is
  NATed, and DNS can stay on the router.
- On the device, open the CA page printed by the CLI (default
  `http://<Mac LAN IP>:8090/`), install the CA, and trust it (iOS requires the
  Certificate Trust Settings step).

The web UI has three views:

- **Live** — request list with app filter chips, search, method/status filters,
  pause, clear, HAR export; click a row for headers/bodies/cURL.
- **WebSocket** — frames grouped by host, both directions, expandable payloads.
- **Setup** — explicit-proxy and gateway instructions, LAN values, QR code that
  opens the device CA page.
- **Rules** — edit the YAML app classification rules in place; changes apply
  immediately.

## Current status

Implemented and locally verified (2026-10-23):

- `proxybot start` with explicit HTTPS capture, CA download/trust, SSE live UI,
  detail view, filters, HAR export, SQLite history, YAML app rules.
- Transparent adapter end-to-end: direct TLS with SNI and plain HTTP both reach
  the MITM engine and appear in the UI (the pf installation itself is not
  exercised by CI; use `proxybot gateway install --dry-run` to inspect).
- `proxybot gateway status` and `gateway uninstall` (privileged operations are
  untested in CI).
- Response bodies are streamed through a tee wrapper, so chunked / unknown-length
  responses are captured (up to 1 MB, truncated flag) without breaking downloads.
- WebSocket frames are captured in both directions and shown in the UI
  (text as UTF-8, binary as hex). permessage-deflate is not supported: clients
  that negotiate compression see a protocol error through the proxy.

Known MVP gaps: HTTP/2 is disabled (clients fall back to HTTP/1.1); transparent
HTTP only rewrites the first request on a connection (it forces
`Connection: close`); WebSocket compression is unsupported.

## What is intentionally missing

No Tauri, no React build chain, no MCP, no AI/generation/deploy, no Graph/Topology,
no workspace/alerts/anomalies, no device registry. HTTP/2 is not enabled in the
MVP; HTTP/1.1 and HTTPS are. WebSocket frame capture and transparent HTTP
keep-alive are follow-ups. Certificate-pinned apps are expected to refuse
interception.

## Development

```bash
cd minimal
cargo run -- start --no-open   # use --data-dir ./tmp-data for a scratch profile
cargo test
```

Data lives in `~/Library/Application Support/ProxyBot/`:

| File | Purpose |
| --- | --- |
| `ca.pem`, `ca.key` | local root CA (key mode 0600) |
| `history.sqlite` | captured requests (5,000-row cap) |
| `rules/apps.yaml` | app classification rules |
| `gateway.json` | previous forwarding state used for cleanup |

## Gateway details

`proxybot gateway install` loads a dedicated pf anchor
(`com.apple/proxybot`) with:

```text
rdr pass on <iface> inet proto tcp from any to any port {80, 443} -> <lan_ip> port <transparent_port>
pass in quick on <iface> inet from <lan_cidr> to any keep state
pass out quick on <iface> inet from any to any keep state
nat on <iface> inet from <lan_cidr> to any -> (<iface>)
```

and enables `net.inet.ip.forwarding`. `proxybot gateway uninstall` flushes the
anchor and restores the previous forwarding value. Use `--dry-run` to inspect
the rules/commands without changing the system.

The gateway path is macOS/pf-specific and cannot be exercised in CI; explicit
proxy mode has no privileged dependencies and is the primary tested path.
