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

Use the ProxyBot icon in the macOS menu bar and choose **Start Proxy**. The
current main window does not yet expose the full Capture Session control; this
is tracked as P0 in the [roadmap](roadmap.md).

The default proxy port is `8088`.

## 2. Find the Mac's LAN address

For a typical Wi-Fi connection:

```bash
ipconfig getifaddr en0
```

If that produces no address, identify the active interface in macOS Network
Settings. Do not use `127.0.0.1` on the phone; it refers to the phone itself.

## 3. Configure an explicit proxy

On the test device, edit the current Wi-Fi network and set its HTTP proxy to
**Manual**:

- **Server:** the Mac LAN address from step 2
- **Port:** `8088`
- **Authentication:** off

Do not change the device gateway or DNS for this first setup. Those settings are
part of the Advanced `pf` + DNS mode.

Open `http://example.com` on the device. A Captured Request should appear on the
Traffic page. If it does not, stop here and check that both devices are on the
same network, the proxy is running, and the macOS firewall allows the app.

## 4. Install the CA for HTTPS

1. Open **Certs** in ProxyBot.
2. Choose **Start CA Server** and note the displayed LAN URL.
3. Open that URL on the test device and download the CA or platform profile.
4. Install the profile on the test device.
5. On iOS, also enable full trust under **Settings → General → About →
   Certificate Trust Settings**.

Android trust behavior depends on OS version and application configuration. Many
apps do not trust user-installed CAs. Certificate-pinned apps may reject the
connection on either platform; that is an application security boundary, not a
guaranteed ProxyBot capability.

Open `https://example.com`. The request and response should now appear in
Traffic. Never publish the generated CA private key or a captured credential.

## 5. Debug a request

- Use Traffic to search by host, method, status, device, or application when
  available.
- Inspect headers and bodies in the request detail.
- Use a Routing Rule or breakpoint when you need to modify behavior.
- Use Replay or Composer to reproduce a request.
- Export only after removing secrets and personal data.

## 6. Clean up

1. Choose **Stop Proxy** from the ProxyBot menu-bar item.
2. Return the device Wi-Fi proxy setting to **Off**.
3. Stop the CA server.
4. Remove the ProxyBot profile and CA from the test device when no longer needed.
5. If you enabled Advanced `pf` routing, disable it before quitting.

## Known limitations

- The application is a development preview and is not yet shipped through a
  maintained Homebrew tap.
- Existing GitHub ZIP releases do not yet represent the target signed,
  notarized, and smoke-tested distribution pipeline.
- Start/Stop is currently available from the macOS menu-bar item rather than the
  mounted main-window Layout.
- Browser Playwright tests use a mock desktop Adapter and do not prove this real
  device journey.
- TUN/iOS VPN and SSL-bypass flows are Labs, not supported setup paths.

See [Architecture](architecture.md), [Product comparison](comparison.md), and the
[Product roadmap](roadmap.md) for the next work.
