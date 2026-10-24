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
  pf redirects to the Mac's **LAN IP** (not loopback), so in gateway mode the
  transparent adapter listens on `0.0.0.0:<transparent_port>`.
- Covers every TCP :80/:443 connection from the device regardless of whether the
  app honours the system proxy; `--block-quic` forces QUIC apps back to TCP.
  UDP/DNS/ICMP are not HTTP and are not inspected (pf NATs them).
- The Mac is a single point of failure while active.

| | Mode 1 · HTTP proxy | Mode 2 · Side-router |
| --- | --- | --- |
| Device setting | Wi-Fi HTTP proxy | Wi-Fi gateway |
| Mac privileges | none | admin (pf + sysctl) |
| Coverage | HTTP/HTTPS via system proxy | all IP traffic |
| Failure impact | only proxied apps | all device traffic |
| Best for | app dev, QA, API debugging | reverse engineering, full capture |

> Many apps do not honour the iOS/Android system HTTP proxy for all traffic
> (Douyin/ByteDance apps use their own networking stack and QUIC). Use the
> side-router mode (`--gateway --block-quic`) when an app produces no rows in
> HTTP proxy mode. Pinned apps can still reject MITM after the connection is
> redirected; that is an application boundary, not a proxy failure.

Both modes share the same CA, MITM engine, SQLite history, app rules, web UI and
HAR export; only the listener and the device-side setting differ.

## Install

Recommended (works even when `raw.githubusercontent.com` is blocked, because the
installer is attached to the GitHub release):

```bash
curl -fsSL https://github.com/mbpz/proxybot/releases/latest/download/install.sh | sh
```

Other options:

```bash
# From a repository checkout
sh minimal/install.sh

# Pin a release
PROXYBOT_VERSION=0.1.0 sh minimal/install.sh

# Install an already-built local binary (offline)
PROXYBOT_LOCAL_BINARY=minimal/target/release/proxybot sh minimal/install.sh

# Mirror that serves the same proxybot-min-*.tar.gz assets
PROXYBOT_DOWNLOAD_BASE=https://your-mirror.example/proxybot sh minimal/install.sh

# Custom prefix (default ~/.local/bin)
PROXYBOT_PREFIX=/usr/local/bin sh minimal/install.sh
```

The script downloads the matching macOS arm64/x64 release archive, verifies its
SHA-256, and installs `proxybot` to `~/.local/bin` (override with
`PROXYBOT_PREFIX`). No Homebrew, Node, Python, or Rust toolchain is required at
install time.

### Local development loop (no GitHub download)

For fast local debugging, build from this checkout instead of downloading a
release:

```bash
sh minimal/dev-run.sh start --gateway --block-quic   # build + run, Ctrl+C stops
sh minimal/dev-run.sh list                           # any proxybot command
sh minimal/dev-install.sh                            # build + install ~/.local/bin

# Or produce the same archive layout locally, then install it offline:
sh minimal/package.sh
PROXYBOT_LOCAL_ARCHIVE=minimal/dist/proxybot-min-$(uname -m | sed 's/arm64/aarch64-apple-darwin/;s/x86_64/x86_64-apple-darwin/').tar.gz \
  sh minimal/install.sh
```

`dev-run.sh` and `dev-install.sh` reuse the repository-local `.cache-cargo`
when present and build offline by default, so the loop is typically a few
seconds after the first build.

Releases are built by the manual **Minimal CLI release** workflow
(`.github/workflows/minimal-release.yml`). The first release,
`minimal-v0.1.0` (2026-10-24), ships arm64/x64 archives with SHA-256 checksums;
both `PROXYBOT_VERSION=0.1.0` and the default `latest` install paths were
verified locally.

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
  NATed, and DNS can stay on the router. Apps that ignore the iOS/Android system
  proxy (many Chinese apps use their own network stack) are only visible in this
  mode. Add `--block-quic` to drop QUIC/UDP 80/443 so those apps fall back to
  TCP/TLS; certificate-pinned apps may still refuse decryption.
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

### Certificate pinning and TLS attempts

Apps that pin certificates reject the ProxyBot CA during the TLS handshake
(`CertificateUnknown` / handshake EOF in the terminal). Those handshakes cannot
be decrypted. To make the failure visible and keep the app usable:

- The **TLS** tab lists every observed SNI host, its attempt count, and whether
  it was intercepted or passed through.
- Add the host suffix to
  `~/Library/Application Support/ProxyBot/rules/passthrough.txt` (one per line,
  subdomains match automatically) and restart ProxyBot. Matching hosts are then
  forwarded as raw TLS without interception: the app works normally, the host
  still appears in the TLS tab, but HTTP bodies are not captured.

### Body decoding and encrypted app protocols

- gzip/deflate/brotli HTTP bodies are decompressed before storage and marked as
  decoded in the detail view.
- WeChat MMTLS (`/mmtls/...`, `Upgrade: mmtls`) and ByteDance/TTNet binary frames
  are **application-layer encrypted**; TLS MITM cannot turn them into plaintext.
  The detail view labels them explicitly and shows a hex preview instead of a
  misleading base64 blob.
- JSON/text/image bodies are shown as text or binary metadata as appropriate.

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
# only with --block-quic:
block drop quick on <iface> inet proto udp from <lan_cidr> to any port {80, 443}
```

and enables `net.inet.ip.forwarding`. `proxybot gateway uninstall` flushes the
anchor and restores the previous forwarding value. Use `--dry-run` to inspect
the rules/commands without changing the system.

The gateway path is macOS/pf-specific and cannot be exercised in CI; explicit
proxy mode has no privileged dependencies and is the primary tested path.
