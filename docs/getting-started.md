# Getting started

This guide uses explicit proxy mode because it is the smallest and most
predictable setup. The current release artifacts are development previews, so
the supported contributor path is a source build. The shell has three
destinations — **Capture**, **Setup**, and **Settings**; device onboarding lives
entirely in Setup.

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

The build is self-contained: there are no second-stage downloads and no extra resource
step. Open **Setup** and choose iOS or Android.

The macOS menu-bar item provides the same Capture Session Start/Stop actions as
the Capture bar and stays synchronized with the main window. Capture-related
tools live inside the Capture context (Requests, Rules, Replay, DNS, Alerts);
Analysis appears there only when Labs are enabled. Labs experiments are off by
default and are enabled under **Settings → Labs** if you need them.

## 2. Start capture and prepare the Mac

The Setup page walks through four steps:

1. **Start the Capture Session** — starts the local MITM Runtime.
2. **Prepare this Mac** — discovers the active LAN interface and starts the
   temporary CA download server. ProxyBot shows the exact LAN address and proxy
   port to enter on the device.
3. **Configure the device** — the explicit-proxy values.
4. **Install and verify the CA** — the platform-specific certificate steps.

Step 1 and step 2 are separate lifecycles: a failure in one does not imply that
the other is running.
Do not use `127.0.0.1` on the phone; it refers to the phone itself. If the Mac
changes networks, stop the Setup Server and prepare again.

## 3. Configure the explicit proxy

On the test device, edit the current Wi-Fi network and set its HTTP proxy to
**Manual**:

- **Server:** the LAN address displayed by Setup
- **Port:** the port displayed by Setup (default `8088`)
- **Authentication:** off

Do not change the device gateway or DNS for this first setup. Those settings are
part of the Advanced `pf` + DNS mode.

Open `http://example.com` on the device. A Captured Request should appear on the
Capture → Requests page. If it does not, check that both devices are on the same
network, the proxy is running, and the macOS firewall allows the app.

## 4. Install the CA for HTTPS

Scan the QR code shown by Setup while the temporary Setup Server is running.
iOS downloads only the ProxyBot CA; ProxyBot does not change Wi-Fi or DNS through
a managed profile. Android opens a local guide and CA download.

Install the CA on the test device. On iOS, also enable full trust under
**Settings → General → About → Certificate Trust Settings**.

Android trust behavior depends on OS version and application configuration. Many
apps do not trust user-installed CAs. Certificate-pinned apps may reject the
connection on either platform; that is an application security boundary, not a
guaranteed ProxyBot capability.

Open `https://example.com`. The request and response should now appear in
Capture → Requests. Never publish the generated CA private key or a captured
credential. After installation, choose **Stop Setup Server**; this does not stop
capture.

## 5. Debug a request

- In Capture → Requests, filter by host, method, status, device, or application
  when available.
- Open a request to inspect headers and bodies.
- From the request detail you can replay the request, create a rule from it,
  copy it as cURL, or export it as HAR.
- Use Rules or a breakpoint when you need to change behavior.
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
- Existing GitHub releases are previews; they do not yet represent the target
  signed, notarized, and install-verified distribution pipeline.
- Browser Playwright tests use a mock desktop Adapter and do not prove this real
  device journey; the packaged acceptance lane proves the backend journey only.
- Physical iOS and Android validation is still tracked as release evidence; use
  the [device validation record](https://github.com/mbpz/proxybot/blob/main/docs/status/device-validation-template.md)
  when collecting it.
- Labs (AI analysis, Generate/Deploy, Graph/Topology) are experimental and off by
  default; they are not part of the supported first-capture journey.

See [Architecture](architecture.md), [Product comparison](comparison.md), and the
[Product roadmap](roadmap.md) for the next work.
