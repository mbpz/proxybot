# ProxyBot

[![Minimal CLI CI](https://github.com/mbpz/proxybot/actions/workflows/minimal-ci.yml/badge.svg)](https://github.com/mbpz/proxybot/actions/workflows/minimal-ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Latest CLI release](https://img.shields.io/github/v/release/mbpz/proxybot?filter=minimal-v*&label=cli)](https://github.com/mbpz/proxybot/releases)

ProxyBot is a lightweight macOS MITM traffic debugger for mobile developers.
One CLI binary starts an explicit HTTP proxy, a local web UI, a locally generated
CA, and an optional Gateway Beta (TUN + sing-box) for full-device testing.

The active implementation is [`minimal/`](minimal/README.md): a small Rust CLI +
local web UI. The Tauri desktop app remains in the repository as legacy work but
is not the current product path.

> Authorized use only: TLS interception exposes sensitive traffic. Use ProxyBot
> only on devices, applications, and networks you own or are explicitly
> authorized to test. Never share captured payloads, private keys, certificates
> or access tokens.

## Modes

| Mode | Status | Device setup | Admin |
| --- | --- | --- | --- |
| HTTP proxy | **Supported / default** | Wi-Fi HTTP proxy `<Mac>:8088` | No |
| Gateway Beta | Beta | Wi-Fi gateway `<Mac>`, proxy off | Yes (TUN/pf) |

Gateway currently requires a user-supplied sing-box built with `-tags with_gvisor`;
it is **not bundled**. See [Gateway Beta](docs/gateway-beta.md).

## Requirements

- macOS (Apple Silicon or Intel releases).
- No Rust/Node/Python toolchain needed for installation.
- Gateway additionally needs root and a compatible sing-box binary.

## Install

```bash
curl -fsSL https://github.com/mbpz/proxybot/releases/latest/download/install.sh | sh
proxybot --version
```

The installer verifies the archive SHA-256 and installs to `~/.local/bin` by
default. If that directory is not on `PATH`, add it:

```bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

Other install paths:

```bash
# From a repository checkout
sh minimal/install.sh

# Pin a release
PROXYBOT_VERSION=0.1.13 sh minimal/install.sh

# Local build (no download)
sh minimal/dev-install.sh

# Custom prefix
PROXYBOT_PREFIX=/usr/local/bin sh minimal/install.sh
```

## Quick start: HTTP proxy

```bash
proxybot start
```

The CLI prints the values you need:

```text
Web UI          http://127.0.0.1:8899
Explicit proxy  <Mac LAN IP>:8088
CA page         http://<Mac LAN IP>:8090/
```

1. Put the test device on the same Wi-Fi and set its HTTP proxy to
   `<Mac LAN IP>:8088`.
2. On the device, open `http://<Mac LAN IP>:8090/` and install the ProxyBot CA.
3. iOS only: enable full trust under
   Settings → General → About → Certificate Trust Settings.
4. Open `https://example.com`; the request appears in the Live UI.

HTTP proxy needs no admin and does not change the Mac's routing. Install/trust
the CA only on test devices you control.

## Gateway Beta

Gateway sends a device's TCP traffic through a TUN interface, sing-box and
Hudsucker. It is a Beta path; keep the device on automatic networking when it is
not running.

Prerequisites: root, and a sing-box v1.14.4-compatible binary with gvisor support.
Pass it explicitly if it is not on `PATH`:

```bash
# terminal 1: ProxyBot must be running
proxybot start

# terminal 2: install a LaunchDaemon (explicit start only, no autostart)
sudo proxybot gateway install-service --singbox /path/to/sing-box

sudo proxybot gateway start
proxybot gateway status
```

On the device: Wi-Fi **gateway** = `<Mac LAN IP>`, HTTP proxy off, DNS may stay on
the router.

Stop/remove:

```bash
sudo proxybot gateway stop
sudo proxybot gateway uninstall-service
```

The old `start --gateway` and `gateway install|uninstall` pf paths are legacy and
unsupported. See [Gateway Beta](docs/gateway-beta.md) and
[status](docs/status.md) for limitations (UDP/QUIC direct, HTTP/80 identity
unknown, no sleep/Wi-Fi automatic reconfiguration, certificate pinning).

## Commands

```bash
proxybot start                 # HTTP proxy + web UI
proxybot gateway start         # Gateway Beta (root; sing-box required)
proxybot gateway stop
proxybot gateway status
proxybot uninstall             # prompts for data/CA removal
proxybot update                # download + verify + replace binary
proxybot list                  # full command reference with examples
proxybot --help                # core commands
```

`proxybot uninstall --purge --yes` removes user data and the verified ProxyBot CA
without prompting; it is destructive. System Keychain removal may require sudo;
iPhone profiles/trust are always manual.

## Data and CA

```text
~/Library/Application Support/ProxyBot/
  ca.key, ca.pem       local CA (never share)
  history.sqlite       captured requests
  rules/               app classification / passthrough rules
```

CA trust lives outside this directory, in macOS Keychain or an iOS profile.
Uninstalling the binary does not automatically revoke it.

## Update / uninstall

```bash
proxybot update                 # verify GitHub SHA-256, replace this binary
proxybot uninstall              # remove binary/share; prompt for data + CA
proxybot uninstall --purge --yes
```

## Status and limitations

- Explicit HTTP proxy is the supported Core mode and is verified on iPhone
  Safari/WeChat.
- Gateway Beta passed lifecycle and two-device source-IP tests, but remains a
  Beta support claim.
- UDP/QUIC/DNS are not decrypted; TLS-pinned apps can refuse interception.
- HTTP/80 is MITM'd but its device identity remains unknown.
- `confidence=inferred` is not device authentication.
- launchd/sleep/Wi-Fi behavior is documented and tested for the current
  Supervisor; automatic network reconfiguration is out of scope.

See [docs/status.md](docs/status.md), [Gateway Beta](docs/gateway-beta.md), and
[release checklist](docs/release-checklist.md).

## Development

```bash
sh minimal/dev-run.sh start          # build + run HTTP proxy
sh minimal/dev-install.sh            # build + install ~/.local/bin
cargo test --manifest-path minimal/Cargo.toml
sh minimal/package.sh                # local release archive
sh minimal/rc-smoke-test.sh          # clean-install/HTTP/HTTPS/uninstall smoke
```

CI: `.github/workflows/minimal-ci.yml`; packaging:
`.github/workflows/minimal-release.yml` (manual dispatch).

## Documentation

- [minimal/README.md](minimal/README.md) — CLI details
- [docs/status.md](docs/status.md) — verified/unsupported behavior
- [docs/gateway-beta.md](docs/gateway-beta.md) — Gateway Beta guide
- [docs/release-checklist.md](docs/release-checklist.md) — release gates
- [docs/adr/](docs/adr/README.md) — architecture decisions
- [docs/legal/sing-box.md](docs/legal/sing-box.md) — sing-box/GPL decision

## License

ProxyBot is MIT-licensed. The Gateway Beta does not bundle sing-box, which is
GPL-3.0 and supplied separately by the operator. See [LICENSE](LICENSE) and
[minimal/THIRD_PARTY.md](minimal/THIRD_PARTY.md).
