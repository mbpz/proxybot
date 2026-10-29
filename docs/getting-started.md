# Getting started

This guide uses the supported explicit HTTP proxy path. It needs no admin rights
and does not change the Mac's routing.

## Requirements

- macOS (Apple Silicon or Intel).
- A test iPhone/Android device on the same Wi-Fi network.
- No Rust, Node or Python toolchain needed for installation.

## Install

```bash
curl -fsSL https://github.com/mbpz/proxybot/releases/latest/download/install.sh | sh
proxybot --version
```

The installer verifies the release archive checksum and installs to
`~/.local/bin` by default. Add it to `PATH` if needed:

```bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

From a checkout:

```bash
sh minimal/install.sh
# or a local build:
sh minimal/dev-install.sh
```

## Start capture

```bash
proxybot start
```

The CLI prints:

```text
Web UI          http://127.0.0.1:8899
Explicit proxy  <Mac LAN IP>:8088
CA page         http://<Mac LAN IP>:8090/
```

The web UI is loopback-only; the CA page and proxy are reachable on the LAN.

## Connect the device

1. Connect the device to the same Wi-Fi.
2. Set its HTTP proxy to `<Mac LAN IP>:8088`.
3. Keep gateway/DNS automatic for this mode.
4. Open `http://<Mac LAN IP>:8090/` on the device.
5. Install the certificate and enable trust:
   - iOS: Settings → General → VPN & Device Management, then
     Settings → General → About → Certificate Trust Settings.
   - Android: install as a user CA.
6. macOS: Keychain Access → System/Login → always trust the ProxyBot CA.

## Verify HTTPS

Open `https://example.com` and check the Live tab. A real request row with URL,
method and status means the MITM path works. A TLS/SNI-only row is not enough.

If the CA page is unreachable from the device:

- confirm the device and Mac are on the same LAN;
- check the macOS firewall;
- avoid guest/AP-isolated Wi-Fi;
- verify the CA page from the Mac first:
  ```bash
  curl -I "http://<Mac LAN IP>:8090/"
  ```

## Stop and restore

1. Disable the device HTTP proxy.
2. Confirm normal browsing still works.
3. Stop ProxyBot with Ctrl+C.
4. Remove the test CA from the device if it is no longer needed.

## Gateway Beta

Gateway uses TUN + sing-box and is a separate Beta path. It requires root and a
user-supplied sing-box built with `-tags with_gvisor`; see
[Gateway Beta](gateway-beta.md).

## Uninstall / update

```bash
proxybot update
proxybot uninstall              # prompts for data and verified CA removal
proxybot uninstall --purge --yes
```

The system-trusted CA is not removed by uninstalling the binary; remove it in
Keychain/iOS Settings separately.
