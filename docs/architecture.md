# Architecture

## Overview

ProxyBot acts as a transparent HTTPS MITM proxy on your Mac. When your iOS/Android device is configured to use your Mac as the gateway, all HTTP/HTTPS traffic flows through ProxyBot, which can decrypt, inspect, and log the traffic.

## Traffic Flow

```
Phone --[WiFi]--> Mac (pf redirect :80/:443) --> ProxyBot (MITM) --> Internet
                                                            |
                                                            +--> DNS Server (log queries, correlate with apps)
```

## Components

### 1. Packet Filter (pf)

macOS's built-in firewall redirects all HTTP/HTTPS traffic from the phone to the local proxy port (8088). This is transparent to the phone — no per-app proxy configuration needed.

### 2. MITM Proxy (Rust)

The core proxy written in Rust using:
- `hyper` for HTTP parsing
- `rustls` for TLS (MITM with dynamically generated leaf certificates)
- `tokio` for async I/O

### 3. Certificate Authority (CA)

On first launch, ProxyBot generates a root CA certificate. This CA must be installed and trusted on the phone. For each HTTPS connection, ProxyBot dynamically generates a leaf certificate signed by the root CA.

### 4. DNS Server

A built-in DNS server on port 53 logs all DNS queries from the phone. This is used to correlate DNS lookups with observed traffic for app classification.

### 5. App Classification

By analyzing SNI (Server Name Indication) in TLS ClientHello messages and correlating with DNS query logs, ProxyBot groups traffic by application (WeChat, Douyin, Alipay, etc.).

### 6. Rule Engine

Domain rules determine how traffic is handled:
- **DIRECT** — Forward without MITM (for banking apps, etc.)
- **PROXY** — Route through an upstream proxy
- **REJECT** — Drop the connection
- **MAPREMOTE** — Map to a different remote host
- **MAPLOCAL** — Map to a local file or mock response
- **BREAKPOINT** — Pause for inspection before proceeding

## Data Storage

- **SQLite** (`~/.proxybot/proxybot.db`) stores request/response history, device registry, and alert state
- **Certificate storage** (`~/.proxybot/certs/`) for CA and generated leaf certificates
- **Rule files** (`~/.proxybot/rules/`) in YAML format, hot-reloaded on change