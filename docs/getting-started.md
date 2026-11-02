# Getting started

This guide uses the supported explicit HTTP proxy path. It needs no admin rights
and does not change the computer's routing.

## Requirements

- macOS (Apple Silicon or Intel) or Windows x64.
- A test iPhone/Android device on the same Wi-Fi network.
- No Rust, Node or Python toolchain needed for installation.

## Install on macOS

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

## Install on Windows x64

After a Windows release is published, download
`proxybot-min-x86_64-pc-windows-msvc.zip` and its `.sha256` file from the same
CLI release. In PowerShell:

```powershell
$zip = 'proxybot-min-x86_64-pc-windows-msvc.zip'
$expected = (Get-Content "$zip.sha256").Split(' ')[0]
if ((Get-FileHash -Algorithm SHA256 $zip).Hash -ne $expected) { throw 'Checksum mismatch' }
Expand-Archive $zip -DestinationPath "$env:LOCALAPPDATA\Programs\ProxyBot" -Force
& "$env:LOCALAPPDATA\Programs\ProxyBot\proxybot.exe" start
```

Allow access on private networks when Windows Firewall asks. The Windows source
build implements the explicit HTTP proxy mode only; Windows CI and device
acceptance are pending. Data is stored in `%LOCALAPPDATA%\ProxyBot`.

## Start capture

```bash
proxybot start
```

The CLI prints:

```text
Web UI          http://127.0.0.1:8899
Explicit proxy  <computer LAN IP>:8088
CA page         http://<computer LAN IP>:8090/
```

The web UI is loopback-only; the CA page and proxy are reachable on the LAN.

## Connect the device

1. Connect the device to the same Wi-Fi.
2. Set its HTTP proxy to `<computer LAN IP>:8088`.
3. Keep gateway/DNS automatic for this mode.
4. Open `http://<computer LAN IP>:8090/` on the device.
5. Install the certificate and enable trust:
   - iOS: Settings → General → VPN & Device Management, then
     Settings → General → About → Certificate Trust Settings.
   - Android: install as a user CA.
6. Trust the ProxyBot CA on the device; no host OS trust is needed for the device proxy.

## Verify HTTPS

Open `https://example.com` and check the Live tab. A real request row with URL,
method and status means the MITM path works. A TLS/SNI-only row is not enough.

If the CA page is unreachable from the device:

- confirm the device and computer are on the same LAN;
- check the host firewall;
- avoid guest/AP-isolated Wi-Fi;
- verify the CA page from the computer first:
  ```bash
  curl -I "http://<computer LAN IP>:8090/"
  ```

## Stop and restore

1. Disable the device HTTP proxy.
2. Confirm normal browsing still works.
3. Stop ProxyBot with Ctrl+C.
4. Remove the test CA from the device if it is no longer needed.

## Gateway Beta

Gateway is macOS only. It uses TUN + sing-box and is a separate Beta path. It requires root and a
user-supplied sing-box built with `-tags with_gvisor`; see
[Gateway Beta](gateway-beta.md).

## Uninstall / update

The commands below are macOS only. On Windows, stop ProxyBot, then remove its
extracted folder; remove `%LOCALAPPDATA%\ProxyBot` only if you want to delete
the history, rules and local CA. Remove the device CA separately.

```bash
proxybot update
proxybot uninstall              # prompts for data and verified CA removal
proxybot uninstall --purge --yes
```

The system-trusted CA is not removed by uninstalling the binary; remove it in
Keychain/iOS Settings separately.
