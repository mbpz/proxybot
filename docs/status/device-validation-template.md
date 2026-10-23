# Device validation record (issue #28)

Copy this file to `docs/status/device-validation-YYYY-MM-DD.md`, fill it in while
testing, and link it from [issue #28](https://github.com/mbpz/proxybot/issues/28).
A passing record here is the physical-device evidence that complements the
packaged desktop acceptance lane.

Do not include captured payloads, credentials, private keys, certificate
material, or personal data. Record hosts, methods, status codes, and observation
IDs only.

## Build under test

| Field | Value |
| --- | --- |
| Product version / tag | |
| Commit (short SHA) | |
| Build kind | source build / unsigned local bundle / signed DMG |
| macOS version | |
| App run command | `pnpm tauri dev` / packaged app |
| Tester | |
| Date | |

## Setup values recorded from the Setup page

| Field | Value |
| --- | --- |
| Mac LAN address | |
| Proxy port | |
| Setup server used for CA download | yes / no |
| `pf` / DNS mode | off (explicit proxy only) |

## iOS

| Check | Result | Evidence |
| --- | --- | --- |
| Device model / iOS version | | |
| Wi-Fi HTTP proxy set to the Mac address and port | | |
| `http://example.com` captured | | request id / host / status |
| ProxyBot CA downloaded via Setup | | |
| CA enabled under Certificate Trust Settings | | |
| `https://example.com` decrypted and captured | | request id / host / status |
| Capture stop restores normal browsing | | |
| Device proxy turned off and CA removed | | |

Notes (pinning-related failures, app-specific behavior):

## Android

| Check | Result | Evidence |
| --- | --- | --- |
| Device model / Android version | | |
| Wi-Fi HTTP proxy set to the Mac address and port | | |
| `http://example.com` captured | | request id / host / status |
| ProxyBot CA downloaded and installed | | |
| `https://example.com` decrypted and captured | | request id / host / status |
| Capture stop restores normal browsing | | |
| Device proxy turned off and CA removed | | |

Notes (apps that ignore user CAs, pinning-related failures):

## Cleanup

- [ ] Capture Session stopped from the main window
- [ ] Device Wi-Fi proxy returned to Off
- [ ] Setup Server stopped
- [ ] CA removed from the device
- [ ] Advanced `pf` routing disabled (if it was enabled)

## Verdict

- [ ] Pass — explicit proxy, CA trust, HTTP, and HTTPS capture all verified on both platforms
- [ ] Fail — record the failing step and the observed error/status below

Failure detail / follow-up issue:

## Out of scope

Certificate-pinned and platform-protected applications are expected to resist
decryption; a failure there is not a ProxyBot regression. AI/Generate/Deploy and
Graph/Topology are Labs and are not part of this validation.
