# ProxyBot

**A macOS desktop traffic debugger for mobile application developers.**

ProxyBot connects an iOS or Android test device to a local Rust MITM Runtime so
you can inspect, filter, modify, replay, and export HTTP, HTTPS, and WebSocket
Captured Requests.

!!! warning "Development preview"

    The project is not yet a stable, notarized macOS distribution. Existing
    GitHub releases are previews; build from source for the current supported
    contributor workflow.

## The focused workflow

1. Start ProxyBot on a Mac.
2. Connect a test device using an explicit proxy.
3. Install and trust the local CA for HTTPS inspection.
4. Verify a known request appears in Traffic.
5. Inspect, change, replay, or export the Captured Request.
6. Stop capture and restore the device network settings.

[Get started](getting-started.md){ .md-button .md-button--primary }
[Read the roadmap](roadmap.md){ .md-button }

## Core capabilities

- HTTP, HTTPS, and WebSocket capture
- Captured Request history, detail, filters, and export
- Routing Rules, breakpoints, replay, and Composer
- certificate export and a local device-setup server
- DNS-supported Application Attribution
- a reusable Rust `proxybot-core` crate

## Scope

Explicit proxy is the default setup path. macOS `pf`, DNS, MCP, scripting, the
mobile dashboard, and protocol analysis are Advanced capabilities. TUN/iOS VPN,
SSL bypass, AI, and generation/deployment features are Labs and are not part of
the supported first-capture journey.

Use ProxyBot only on devices and networks you own or are authorized to test.
Captured traffic and local CA material are sensitive.

## Learn more

- [Getting started](getting-started.md)
- [Architecture](architecture.md)
- [Product comparison and lessons](comparison.md)
- [Product roadmap](roadmap.md)
- [Contributing](https://github.com/mbpz/proxybot/blob/main/CONTRIBUTING.md)
- [Security policy](https://github.com/mbpz/proxybot/blob/main/SECURITY.md)
