# ProxyBot minimal (CLI + local web UI)

The active lightweight ProxyBot implementation: **one binary**, a local web UI,
an explicit HTTP-proxy mode, and a **Beta** gateway mode. It is independent from
the legacy Tauri desktop app at the repository root (separate Cargo workspace).

```
phone --Wi-Fi HTTP proxy--> Mac:8088 ─┐
phone --Wi-Fi gateway-----> Mac TUN  ─┴--> MITM / classification / SQLite / web UI
```

## Modes

| Mode | Status | Device setting | Admin |
| --- | --- | --- | --- |
| Explicit HTTP proxy | **Supported / default** | Wi-Fi HTTP proxy `<Mac>:8088` | No |
| Gateway (TUN + sing-box) | **Beta** | Wi-Fi gateway `<Mac>`, HTTP proxy off | Yes (TUN/pf) |
| Passive sniff | Experimental | none (same interface) | Yes (BPF) |
| Legacy pf `rdr` gateway | **Legacy / unsupported** | do not use | — |

The explicit proxy is the only Core capture mode. It covers apps that honour the
system proxy; apps with certificate pinning can still reject MITM.

## HTTP proxy (supported)

```bash
proxybot start
# Web UI: http://127.0.0.1:8899
```

- Set the device Wi-Fi HTTP proxy to `<Mac LAN IP>:8088`.
- Install/trust the CA from `http://<Mac LAN IP>:8090/`; iOS also needs
  Certificate Trust Settings.
- No pf, NAT, IP forwarding or admin rights.
- A stopped ProxyBot only affects proxied apps; the device can still browse.

## Gateway Beta

Recommended only for testing. Keep the device on automatic networking when the
gateway is not running; a failed gateway can leave the device without internet.

Prerequisites:

- macOS root for TUN/pf.
- ProxyBot running on `127.0.0.1:8088` as the normal user.
- User-supplied sing-box v1.14.4-compatible binary built with
  `-tags with_gvisor` (not bundled; see `docs/legal/sing-box.md`).

```bash
# terminal 1
proxybot start
# terminal 2
sudo proxybot gateway start
sudo proxybot gateway status
sudo proxybot gateway stop
```

Optional policies:

```bash
sudo proxybot gateway start --block-quic       # UDP/443 reject for fallback test
sudo proxybot gateway start --devices minimal/examples/devices.json
proxybot singbox-config --block-quic           # generate config only
```

Device setting: Wi-Fi **gateway** = Mac LAN IP, HTTP proxy off, DNS may stay on
the router.

What Gateway Beta does:

- TUN ingress, TLS SNI sniffing, TCP/443 → Hudsucker MITM.
- TCP/80 → transparent adapter → Hudsucker; HTTP/80 identity is unknown.
- UDP/DNS direct; optional UDP/443 blocking for QUIC fallback tests.
- Per-device source-IP routing with `--devices`; unregistered sources stay
  unknown rather than being assigned to a device.
- Supervisor lifecycle: exit code/signal/reason, startup mutex, stale/orphan
  recovery, log rotation.

What it does not do:

- UDP/QUIC/DNS decryption or full DNS policy.
- Exact TUN ingress-to-request connection correlation (`connection_id` is
  proxy-side diagnostic only).
- launchd, sleep/wake or Wi-Fi-switch handling (release gate, not yet done).
- Stable physical device identity (`device_id` remains NULL; `confidence` is
  `inferred` for static metadata).

Full guide: [`docs/gateway-beta.md`](../docs/gateway-beta.md).
Status: [`docs/status.md`](../docs/status.md).

## Install

```bash
curl -fsSL https://github.com/mbpz/proxybot/releases/latest/download/install.sh | sh
```

Other options:

```bash
sh minimal/install.sh
PROXYBOT_VERSION=0.1.0 sh minimal/install.sh
PROXYBOT_LOCAL_BINARY=minimal/target/release/proxybot sh minimal/install.sh
PROXYBOT_PREFIX=/usr/local/bin sh minimal/install.sh
```

The script downloads the matching macOS archive, verifies SHA-256, and installs
`proxybot` (default `~/.local/bin`). No Rust/Node/Python toolchain required.

### Local development

```bash
sh minimal/dev-run.sh start
sh minimal/dev-install.sh
sh minimal/package.sh
```

`dev-run.sh` and `dev-install.sh` reuse `.cache-cargo` and build offline when
possible. Releases use `.github/workflows/minimal-release.yml`.

## Use

```bash
proxybot start                 # HTTP proxy + web UI
proxybot gateway start         # Gateway Beta (root; sing-box required)
proxybot gateway status
proxybot gateway stop
proxybot singbox-config --help
proxybot rules show
proxybot uninstall [--purge] [--yes]
proxybot update [--version X]
proxybot list
```

Web UI:

- **Live** — request list, app/host/method/status filters, detail, HAR export,
  identity-source badges.
- **WebSocket** — frames grouped by host.
- **TLS** — SNI attempts and interception/passthrough state.
- **Setup** — HTTP proxy instructions, Gateway Beta status, CA page, QR code.
- **Rules** — editable YAML app classification.

## Identity fields

Gateway requests carry:

| Field | Meaning |
| --- | --- |
| `source_type` | `local_proxy` or `gateway` |
| `client_ip` | trusted source IP when metadata exists; otherwise NULL |
| `peer_ip` | actual Hudsucker TCP peer (`127.0.0.1` for gateway) |
| `device_id` | NULL until a stable device registry exists |
| `connection_id` | proxy-side diagnostic connection key |
| `identity_source` | `tun_source_ip`, `device_port`, or `unknown` |
| `confidence` | `confirmed`, `inferred`, or `unknown` |

`confidence=inferred` is deliberate: static per-device metadata proves the
configured route mapping, not a per-connection ingress observation. Unknown
identity is a valid state and is never guessed from `Host`, `User-Agent` or
client-supplied headers.

## Certificate pinning

Pinned apps reject the ProxyBot CA (`CertificateUnknown` / handshake EOF). Add
their host suffixes to
`~/Library/Application Support/ProxyBot/rules/passthrough.txt` to keep them
working; their bodies remain encrypted.

## Legacy PF gateway

`proxybot start --gateway` and `proxybot gateway install|uninstall` are the old
pf `rdr`/NAT research path. They are **legacy and unsupported**; use
`proxybot gateway start` instead. They remain only for cleanup/diagnostics.

## Experimental tools

```bash
sudo proxybot sniff --device <unused IP> --duration 30   # metadata only
sudo proxybot tun-probe --duration 60                    # TUN ingress probe
```

Sniffing sees only packets reaching the Mac and cannot decrypt TLS; ECH/DoH can
hide SNI/DNS.

## Uninstall / update

```bash
proxybot uninstall          # remove binary + installed share files; keep data/CA
proxybot uninstall --purge  # also remove database/rules/CA files
proxybot update             # download + checksum-verify + replace this binary
```

Gateway service cleanup needs root and is delegated first by `uninstall`. CA
trust is never removed automatically.

## Licensing

ProxyBot itself is MIT. The Gateway Beta does not bundle sing-box, which is
GPL-3.0; the operator supplies it separately. See
[`docs/legal/sing-box.md`](../docs/legal/sing-box.md).

## Status and release

- [`docs/status.md`](../docs/status.md) — verified/unsupported behavior.
- [`docs/release-checklist.md`](../docs/release-checklist.md) — release gates.
- [`docs/adr/`](../docs/adr/README.md) — binding decisions.
