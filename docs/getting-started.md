# Getting Started

## Prerequisites

- macOS (required for `pf` transparent proxy)
- Rust toolchain (for building from source)
- Homebrew (recommended installation method)

## Installation

### Homebrew (Recommended)

```bash
brew install mbpz/proxybot/proxybot-tui
```

### Build from Source

```bash
git clone https://github.com/mbpz/proxybot.git
cd proxybot/src-tauri
cargo build --release --bin proxybot-tui
./target/release/proxybot-tui
```

## Device Setup

### Step 1: Connect Phone to Mac's Network

Ensure your iOS/Android device is on the same WiFi network as your Mac.

### Step 2: Configure Device Gateway

On your phone, set:
- **Gateway**: Your Mac's IP address
- **DNS**: Your Mac's IP address

Find your Mac's IP:
```bash
ipconfig getifaddr en0
```

### Step 3: Install CA Certificate

1. Run `proxybot-tui` and navigate to the **Certs** tab
2. Export the CA certificate
3. AirDrop the certificate to your phone
4. On iOS: **Settings → General → About → Certificate Trust Settings** → Enable full trust for the ProxyBot CA

### Step 4: Start Proxying

1. Press `r` in the TUI to start the proxy
2. Watch traffic flow from your phone in real-time

## Next Steps

- Learn about [Keyboard Shortcuts](keyboard-shortcuts.md)
- Explore the [Architecture](architecture.md)
- Compare with [other tools](comparison.md)