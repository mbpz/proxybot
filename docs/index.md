# ProxyBot

**A lightweight macOS MITM traffic debugger for mobile application developers.**

ProxyBot connects an iOS or Android test device to a local Rust MITM Runtime so
you can inspect, filter, modify, replay, and export HTTP, HTTPS, and WebSocket
Captured Requests — without a bundled native toolchain or a second-stage
download.

!!! warning "Development preview"

    The project is not yet a stable, notarized macOS distribution. Existing
    GitHub releases are previews; build from source for the current supported
    contributor workflow.

## The focused workflow

1. Start ProxyBot on a Mac.
2. Connect a test device using an explicit proxy.
3. Install and trust the local CA for HTTPS inspection.
4. Verify a known request appears in Capture → Requests.
5. Inspect, change, replay, or export the Captured Request.
6. Stop capture and restore the device network settings.

[Get started](getting-started.md){ .md-button .md-button--primary }
[Read the roadmap](roadmap.md){ .md-button }

## Core capabilities

- HTTP, HTTPS, and WebSocket capture
- Captured Request history, detail, filters, and export
- Routing Rules, breakpoints, replay, and Composer
- Request detail actions: replay, add rule, copy as cURL, export HAR
- certificate export and a local device-setup server
- DNS-supported Application Attribution
- a reusable Rust `proxybot-core` crate

The default shell exposes three destinations: **Capture**, **Setup**, and
**Settings**. Requests, Rules, Replay, DNS, Alerts, and Analysis live in the
Capture context; Replay and Composer share the Replay context.

## Scope

Explicit proxy is the default setup path. macOS `pf`, DNS, MCP, scripting, the
mobile dashboard, and protocol analysis are Advanced capabilities.

Labs — AI analysis, Generate, Deploy, and the Graph/Topology analysis views — are
experimental, off by default, and not part of the supported first-capture
journey. Enable them under **Settings → Labs**; while disabled, Labs routes show
a single "Labs are off" page.

Use ProxyBot only on devices and networks you own or are authorized to test.
Captured traffic and local CA material are sensitive.

## Learn more

- [Getting started](getting-started.md)
- [Architecture](architecture.md)
- [Product comparison and lessons](comparison.md)
- [Product roadmap](roadmap.md)
- [Architecture decisions](adr/README.md)
- [Changelog](https://github.com/mbpz/proxybot/blob/main/CHANGELOG.md)
- [Support](https://github.com/mbpz/proxybot/blob/main/SUPPORT.md)
- [Contributing](https://github.com/mbpz/proxybot/blob/main/CONTRIBUTING.md)
- [Security policy](https://github.com/mbpz/proxybot/blob/main/SECURITY.md)
