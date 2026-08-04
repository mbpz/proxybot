# Getting started

This guide uses explicit proxy mode because it is the smallest and most
predictable setup. The current release artifacts are development previews, so
the supported contributor path is a source build.

## Before you start

You need:

- a Mac with Xcode Command Line Tools
- a stable Rust toolchain
- Node.js 20 or newer and pnpm 10
- an iOS or Android test device on the same Wi-Fi network
- permission to inspect the device and its traffic

Do not install the ProxyBot CA on a personal or production device.

## 1. Run ProxyBot

```bash
git clone https://github.com/mbpz/proxybot.git
cd proxybot
pnpm install --frozen-lockfile
pnpm tauri dev
```

Open **Setup**, choose iOS or Android, then complete the numbered steps on one
page. **Start Capture** controls the MITM Runtime. **Prepare iOS/Android Setup**
separately discovers the active LAN Interface and starts the temporary CA
download server. A failure in one does not imply that the other is running.

The macOS menu-bar item provides the same Capture Session Start/Stop actions and
remains synchronized with the main window.

## 2. Prepare the Mac

Choose **Prepare iOS Setup** or **Prepare Android Setup**. ProxyBot displays the
exact LAN server address and proxy port for the active Interface. Do not use
`127.0.0.1` on the phone; it refers to the phone itself.

If preparation fails, check that the Mac has an active LAN address and that the
firewall allows ProxyBot. If the Mac changes networks, stop the Setup Server and
prepare again.

## 3. Configure the explicit proxy

On the test device, edit the current Wi-Fi network and set its HTTP proxy to
**Manual**:

- **Server:** the LAN address displayed by Setup
- **Port:** the port displayed by Setup (default `8088`)
- **Authentication:** off

Do not change the device gateway or DNS for this first setup. Those settings are
part of the Advanced `pf` + DNS mode.

Open `http://example.com` on the device. A Captured Request should appear on the
Traffic page. If it does not, stop here and check that both devices are on the
same network, the proxy is running, and the macOS firewall allows the app.

## 4. Install the CA for HTTPS

Scan the QR code shown by Setup while the temporary Setup Server is running.
iOS downloads only the ProxyBot CA; ProxyBot does not change Wi-Fi or DNS through
a managed profile. Android opens a local guide and CA download.

Install the CA on the test device. On iOS, also enable full trust under
**Settings → General → About →
   Certificate Trust Settings**.

Android trust behavior depends on OS version and application configuration. Many
apps do not trust user-installed CAs. Certificate-pinned apps may reject the
connection on either platform; that is an application security boundary, not a
guaranteed ProxyBot capability.

Open `https://example.com`. The request and response should now appear in
Traffic. Never publish the generated CA private key or a captured credential.
After installation, choose **Stop Setup Server**; this does not stop capture.

## 5. Debug a request

- Use Traffic to search by host, method, status, device, or application when
  available.
- Inspect headers and bodies in the request detail.
- Use a Routing Rule or breakpoint when you need to modify behavior.
- Use Replay or Composer to reproduce a request.
- Export only after removing secrets and personal data.

## 6. Clean up

1. Choose **Stop Capture** in the main window, or **Stop Proxy** from the
   ProxyBot menu-bar item.
2. Return the device Wi-Fi proxy setting to **Off**.
3. Choose **Stop Setup Server** if it is still running.
4. Remove the ProxyBot profile and CA from the test device when no longer needed.
5. If you enabled Advanced `pf` routing, disable it before quitting.

## Known limitations

- The application is a development preview and is not yet shipped through a
  maintained Homebrew tap.
- Existing GitHub ZIP releases do not yet represent the target signed,
  notarized, and smoke-tested distribution pipeline.
- Browser Playwright tests use a mock desktop Adapter and do not prove this real
  device journey.
- TUN/iOS VPN and SSL-bypass flows are Labs, not supported setup paths.

See [Architecture](architecture.md), [Product comparison](comparison.md), and the
[Product roadmap](roadmap.md) for the next work.
