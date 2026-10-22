# iOS VPN Research: NEPacketTunnelProvider

## Overview

NEPacketTunnelProvider is Apple's API for creating VPN apps on iOS/macOS. Proxyman Atlantis uses this to capture iOS traffic without per-app proxy configuration.

## Architecture

```
iOS Device                          Mac ProxyBot
+--------+                         +--------+
| NEPacket| ======= TLS =====>> | ProxyBot |
| Tunnel  | <<===== tunnel ===== |  MITM   |
|Provider |                         +--------+
+--------+
```

## Key Components

### 1. PacketTunnelProvider (iOS Swift)

The core VPN provider that runs as a Network Extension on iOS.

```swift
class PacketTunnelProvider: NEPacketTunnelProvider {

    override func startTunnel(options: [String: NSObject]?, completionHandler: @escaping (Error?) -> Void) {
        // Called when VPN is started
        // Set up tunnel network settings
        let settings = NEPacketTunnelNetworkSettings(tunnelRemoteAddress: "10.0.0.1")

        settings.ipv4Settings = NEIPv4Settings(addresses: ["10.0.0.2"], subnetMasks: ["255.255.255.0"])
        settings.ipv4Settings?.includedRoutes = [NEIPv4Route.default()]

        setTunnelNetworkSettings(settings) { error in
            completionHandler(error)
        }
    }

    override func stopTunnel(with reason: NEProviderStopReason, completionHandler: @escaping () -> Void) {
        // Called when VPN is stopped
        completionHandler()
    }

    override func handleAppMessage(_ messageData: Data, completionHandler: ((Data?) -> Void)?) {
        // IPC with main app
        completionHandler?(nil)
    }

    override func sleep(completionHandler: @escaping () -> Void) {
        completionHandler()
    }

    override func wake() {
        // Resume from sleep
    }
}
```

### 2. ProxyBot VPN Server (Rust)

Accepts connections from iOS tunnel, terminates TLS, routes to MITM proxy.

```rust
// New module: src-tauri/src/vpn_server.rs

pub struct VpnServer {
    port: u16,
    proxy_context: Arc<ProxyContext>,
}

impl VpnServer {
    /// Start accepting connections from iOS tunnel
    pub async fn start(&self) -> Result<(), String> {
        let listener = TcpListener::bind(("0.0.0.0", self.port)).await
            .map_err(|e| e.to_string())?;

        loop {
            if let Ok((stream, addr)) = listener.accept().await {
                // Handle tunnel connection
                tokio::spawn(self.handle_tunnel_connection(stream, addr));
            }
        }
    }

    async fn handle_tunnel_connection(&self, stream: TcpStream, _addr: SocketAddr) {
        // Read packets from tunnel
        let mut buf = [0u8; 65535];
        loop {
            match stream.read(&mut buf).await {
                Ok(0) => break, // Connection closed
                Ok(n) => {
                    // Process IP packet
                    self.process_packet(&buf[..n]).await;
                }
                Err(e) => {
                    eprintln!("Tunnel read error: {}", e);
                    break;
                }
            }
        }
    }

    async fn process_packet(&self, packet: &[u8]) {
        // Parse IP header, extract destination, forward to MITM
        // This is a simplified version - real implementation needs proper IP parsing
    }
}
```

### 3. Configuration Profile (.mobileconfig)

XML configuration that iOS users install to connect VPN.

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>PayloadContent</key>
    <array>
        <dict>
            <key>IPv4</key>
            <dict>
                <key>OverridePrimary</key>
                <true/>
            </dict>
            <key>PayloadDescription</key>
            <string>Configures VPN settings for ProxyBot</string>
            <key>PayloadDisplayName</key>
            <string>ProxyBot VPN</string>
            <key>PayloadIdentifier</key>
            <string>com.proxybot.vpn</string>
            <key>PayloadType</key>
            <string>com.apple.vpn.managed</string>
            <key>PayloadUUID</key>
            <string>UUID-HERE</string>
            <key>PayloadVersion</key>
            <integer>1</integer>
            <key>Proxies</key>
            <dict>
                <key>HTTPEnable</key>
                <integer>1</integer>
                <key>HTTPSEnable</key>
                <integer>1</integer>
                <key>HTTPProxy</key>
                <string>PROXYBOT-MAC-IP:8080</string>
                <key>HTTPSProxy</key>
                <string>PROXYBOT-MAC-IP:8080</string>
            </dict>
            <key>ServerAddress</key>
            <string>PROXYBOT-MAC-IP</string>
            <key>VPNType</key>
            <string>packet-tunnel</string>
        </dict>
    </array>
    <key>PayloadDisplayName</key>
    <string>ProxyBot</string>
    <key>PayloadIdentifier</key>
    <string>com.proxybot.vpn-config</string>
    <key>PayloadRemovalDisallowed</key>
    <false/>
    <key>PayloadType</key>
    <string>Configuration</string>
    <key>PayloadUUID</key>
    <string>UUID-HERE</string>
    <key>PayloadVersion</key>
    <integer>1</integer>
</dict>
</plist>
```

## Implementation Steps

### Phase 1: VPN Server (Rust)
1. Create `src-tauri/src/vpn_server.rs` with TLS tunnel termination
2. Add packet forwarding to MITM proxy
3. Add connection state management

### Phase 2: Swift PacketTunnelProvider
1. Create separate Swift package: `proxybot-tunnel`
2. Implement NEPacketTunnelProvider subclass
3. Add TLS connection to ProxyBot
4. Handle IP packet read/write

### Phase 3: Configuration Generator
1. Create `.mobileconfig` XML generator
2. Add TUI/GUI option to export VPN configuration
3. Add QR code scanning for easy setup

### Phase 4: Integration
1. Integrate VPN server start/stop with TUI
2. Add device connection status
3. Add traffic statistics

## Challenges and Risks

### Technical Challenges
1. **IP Packet Parsing**: Need to parse IP headers to extract destination addresses
2. **Tunnel Performance**: UDP-based tunnel may need optimization
3. **Certificate Trust**: iOS requires VPN config to be signed/trusted

### Apple Requirements
1. **Apple Developer Program**: Required for Network Extension entitlement ($99/year)
2. **Network Extension Entitlement**: Must be approved by Apple (review process)
3. **App Store Distribution**: If distributing via App Store, more restrictions

### NEPacketTunnelProvider Limitations
1. **iOS 15+ Required**: For some newer APIs
2. **Memory Limits**: Network Extensions have memory constraints
3. **CPU Limits**: Background execution is limited

## Comparison with Proxyman Atlantis

| Aspect | Proxyman Atlantis | ProxyBot (Planned) |
|--------|-------------------|-------------------|
| Language | Swift | Swift + Rust |
| Architecture | Native Swift | Rust proxy core |
| Certificate | System CA | Existing ProxyBot CA |
| Setup | Guided wizard | TUI + .mobileconfig |

## References

- [Apple NEPacketTunnelProvider Docs](https://developer.apple.com/documentation/networkextension/nepackettunnelprovider)
- [Proxyman Atlantis Source](https://github.com/ProxymanApp/atlantis)
- [Network Extension entitlements](https://developer.apple.com/documentation/networkextension)