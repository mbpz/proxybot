# ProxyBot

**A lightweight MITM traffic debugger for mobile developers.**

macOS is the verified release platform. Windows x64 source support is added;
Windows CI and device acceptance are required before a support claim.

ProxyBot is one Rust CLI plus a local web UI. It starts an explicit HTTP proxy,
terminates TLS with a local CA, records HTTP/HTTPS/WebSocket traffic, and can
optionally run a Gateway Beta for full-device TCP testing.

!!! warning "Gateway Beta"

    Explicit HTTP proxy is the supported default. Gateway Beta is macOS only and requires root and
    a user-supplied sing-box; it is not a bundled dependency. See
    [Gateway Beta](gateway-beta.md) before enabling it.

## Start here

```bash
curl -fsSL https://github.com/mbpz/proxybot/releases/latest/download/install.sh | sh
proxybot start
```

Then:

1. Set the test device Wi-Fi HTTP proxy to `<computer LAN IP>:8088`.
2. Open `http://<computer LAN IP>:8090/` on the device and install the CA.
3. iOS: enable full trust in Certificate Trust Settings.
4. Open `https://example.com`; the request appears in the local UI.

[Get started](getting-started.md){ .md-button .md-button--primary }
[Gateway Beta](gateway-beta.md){ .md-button }
[Current status](status.md){ .md-button }

## Modes

| Mode | Status | Device setup | Admin |
| --- | --- | --- | --- |
| HTTP proxy | **Supported / default on macOS; Windows acceptance gated** | Wi-Fi HTTP proxy `<computer>:8088` | No |
| Gateway Beta | macOS Beta | Wi-Fi gateway `<Mac>`, proxy off | Yes |

The HTTP proxy does not change host routing. On macOS, Gateway uses TUN + sing-box + pf
route-to and needs a compatible sing-box binary with `-tags with_gvisor`.

## What it records

- HTTP and HTTPS request/response headers and bodies (up to 1 MiB per body).
- WebSocket frames.
- App/source classification from SNI/host rules.
- Gateway source metadata: `source_type`, `client_ip`, `peer_ip`,
  `identity_source`, `confidence`.

UDP/QUIC/DNS are **not** decrypted in this release. HTTP/80 identity remains
unknown, and certificate-pinned apps can refuse interception.

## Commands

The Gateway, update, and uninstall commands below are macOS only.

```bash
proxybot start                 # HTTP proxy + local web UI
proxybot gateway start         # Gateway Beta (root; sing-box required)
proxybot gateway stop
proxybot gateway status
proxybot uninstall             # prompt for data/verified CA removal
proxybot update                # download, verify, replace binary
proxybot list                  # full command reference
```

## Documentation

- [Getting started](getting-started.md)
- [Gateway Beta](gateway-beta.md)
- [Status and limitations](status.md)
- [Architecture](architecture.md)
- [Release checklist](release-checklist.md)
- [Architecture decisions](adr/README.md)
- [sing-box licensing](legal/sing-box.md)

Use ProxyBot only on devices and networks you own or are authorized to test.
Captured traffic, CA keys and tokens are sensitive.
