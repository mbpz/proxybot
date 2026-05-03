# iOS VPN API Implementation Design

## Status: Draft | Feasibility Assessment

## Overview

Implement iOS VPN support via NEPacketTunnel so mobile traffic (iPhone/iPad) can be captured without requiring manual proxy configuration on the device. Instead of manually setting gateway/DNS on the phone, the VPN tunnels all traffic through ProxyBot running on the Mac.

## NEPacketTunnel Architecture

NEPacketTunnel is Apple's framework for creating VPN apps. Key components:

### Packet Tunnel Provider (App Extension)

- Runs as a separate app extension (`NEPacketTunnelProvider` subclass)
- Receives network packets via a virtual interface
- Forwards packets to the ProxyBot Mac via local network connection
- Handles `startTunnel()` and `stopTunnel()` lifecycle

### Main App (Host Application)

- Contains NEVPNManager to configure and initiate the VPN
- Handles user permission dialogs (VPN must be approved by user in Settings)
- Communicates with the extension via App Groups (shared container)

### Data Flow

```
iPhone --> [NEPacketTunnel VPN] --> [Local WiFi to Mac] --> [ProxyBot MITM] --> Internet
                                                               |
                                                               +--> [DNS Server]
```

### Required Entitlements

```xml
<!-- Required for Packet Tunnel Provider -->
<key>com.apple.developer.networking.networkextension</key>
<array>
    <string>packet-tunnel-provider</string>
</array>

<!-- App Groups for IPC between main app and extension -->
<key>com.apple.security.application-groups</key>
<array>
    <string>group.com.proxybot.app</string>
</array>
```

## Implementation Options

### Option 1: Native iOS App + Swift Packet Tunnel Extension

**Description:** Separate native iOS app with Swift implementation for both UI and VPN.

**Pros:**
- Full access to NetworkExtension framework
- Proper VPN configuration UI via NEVPNManager
- Mature, well-documented approach
- Apple-approved mechanism for VPN apps

**Cons:**
- Requires Swift/ObjC development outside Rust/Tauri
- Separate code base from main ProxyBot
- Must maintain iOS-specific build pipeline
- Two apps to distribute (macOS ProxyBot + iOS companion)

**Effort:** High (2-4 weeks for basic implementation)

### Option 2: Standalone iOS VPN App (No Mac UI)

**Description:** iOS app that acts purely as VPN client, configured to connect to ProxyBot on the local Mac. Minimal UI - just on/off and status.

**Pros:**
- Simpler than full companion app
- Works with existing ProxyBot (Mac acts as VPN server)
- Can be distributed independently via TestFlight

**Cons:**
- User must configure VPN server address manually or via QR code
- No integration with ProxyBot UI on Mac
- Still requires separate iOS app

**Effort:** Medium (1-2 weeks)

### Option 3: Tauri Mobile + Future Extension Support

**Description:** Wait for Tauri v3 or later which may add iOS extension support.

**Pros:**
- Keep everything in Rust/Tauri
- Consistent codebase

**Cons:**
- Tauri currently does NOT support app extensions (NEPacketTunnel, Content Filter, etc.)
- Timeline uncertain - could be years
- Blocks this feature

**Effort:** Not feasible now

## Recommended Approach

**Option 1: Native iOS App + Swift Packet Tunnel Extension**

Rationale:
- NEPacketTunnelProvider is the ONLY Apple-approved way to implement VPN on iOS
- App extensions require native code - cannot be implemented in Tauri/Rust
- This is how all legitimate VPN apps work on iOS (ExpressVPN, Surge, etc.)
- Can be a lightweight companion app that pairs with ProxyBot on Mac

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    iOS Device                                │
│  ┌───────────────┐        ┌────────────────────────────┐   │
│  │ ProxyBot iOS  │        │ PacketTunnelExtension      │   │
│  │ (Main App)    │        │ (NEPacketTunnelProvider)    │   │
│  │               │        │                            │   │
│  │ NEVPNManager  │◄───────►│ - startTunnel()            │   │
│  │ VPN toggle   │ App     │ - stopTunnel()             │   │
│  │ Status       │ Groups  │ - handleAppMessage()       │   │
│  └───────┬───────┘        └─────────────┬──────────────┘   │
│          │                              │                  │
│          │        ┌─────────────────────┘                  │
│          │        │                                       │
│          ▼        ▼                                       │
│  ┌─────────────────────────────────────────┐              │
│  │         Virtual Network Interface        │              │
│  │         (All traffic goes here)          │              │
│  └────────────────────┬────────────────────┘              │
└───────────────────────┼────────────────────────────────────┘
                        │ Port 8088 (or encrypted tunnel)
                        ▼
              ┌─────────────────────┐
              │   ProxyBot Mac       │
              │   (MITM + Rules)     │
              └─────────────────────┘
```

### Key Implementation Details

**1. PacketTunnelProvider Subclass**

```swift
class ProxyBotTunnelProvider: NEPacketTunnelProvider {

    override func startTunnel(options: [String: NSObject]?, completionHandler: @escaping (Error?) -> Void) {
        // Configure virtual interface settings
        let settings = NEPacketTunnelNetworkSettings(tunnelRemoteAddress: "10.0.0.1")

        // Set DNS to Mac's local IP
        let dnsSettings = NEDNSSettings(servers: [macIP])
        dnsSettings.matchDomains = [""] // All domains
        settings.dnsSettings = dnsSettings

        // IPv4 settings
        let ipv4Settings = NEIPv4Settings(addresses: ["10.0.0.2"], subnetMasks: ["255.255.255.0"])
        ipv4Settings.includedRoutes = [NEIPv4Route.default()]
        settings.ipv4Settings = ipv4Settings

        setTunnelNetworkSettings(settings) { error in
            completionHandler(error)
        }

        // Start reading packets and forwarding to Mac
        startPacketCapture()
    }

    override func stopTunnel(with reason: NEProviderStopReason, completionHandler: @escaping () -> Void) {
        // Cleanup
        completionHandler()
    }

    override func handleAppMessage(_ messageData: Data, completionHandler: ((Data?) -> Void)?) {
        // IPC with main app (via App Groups)
        completionHandler?(responseData)
    }
}
```

**2. Main App VPN Configuration**

```swift
// Use NEVPNManager to install VPN profile
let manager = NEVPNManager.shared()
manager.loadFromPreferences { error in
    let protocolConfig = NEPacketTunnelProviderProtocol()
    protocolConfig.providerBundleIdentifier = "com.proxybot.app.tunnel"
    protocolConfig.serverAddress = "Mac's IP"

    manager.protocolConfiguration = protocolConfig
    manager.localizedDescription = "ProxyBot"
    manager.isEnabled = true
    manager.saveToPreferences { error in
        // User must approve in Settings > VPN
    }
}
```

**3. Communication with Mac**

The extension forwards packets to the Mac proxy. Options:
- Direct connection to Mac IP:8088 (same as current pf approach)
- Encrypted tunnel (harder to detect/block)
- Use App Groups for configuration sharing

## Required Entitlements

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>com.apple.developer.networking.networkextension</key>
    <array>
        <string>packet-tunnel-provider</string>
    </array>
    <key>com.apple.security.application-groups</key>
    <array>
        <string>group.com.proxybot.app</string>
    </array>
</dict>
</plist>
```

**Additional requirements:**
- Apple Developer account (paid membership)
- Network Extension capability enabled in App ID
- Proper provisioning profiles with Network Extension entitlement

## Minimal Implementation Steps

1. **Create iOS project structure**
   - Main app target (ProxyBot iOS)
   - Packet Tunnel Extension target
   - App Groups capability for IPC

2. **Implement NEPacketTunnelProvider subclass**
   - `startTunnel()`: Configure network settings, start packet capture
   - `stopTunnel()`: Cleanup resources
   - Forward packets to Mac via UDP/TCP

3. **Implement NEVPNManager integration**
   - Create/load VPN preferences
   - Handle approval flow (user must enable in Settings)

4. **Add macOS-side VPN server support**
   - Accept tunneled connections on a new port
   - Integrate with existing proxy pipeline

5. **Add pairing flow (optional)**
   - QR code with Mac IP + port
   - Bonjour discovery for automatic pairing

6. **Test with Apple approval process**
   - Network Extension entitlements require Apple review
   - May need to explain why the VPN is needed

## Open Questions

1. **How to handle certificate installation?**
   - Current approach: User installs CA on phone manually
   - VPN approach: Can we push CA automatically? Or should we use TLS interception within the tunnel?

2. **Should we use a dedicated tunnel protocol?**
   - Option A: Plain TCP to Mac proxy (same as current pf approach)
   - Option B: Custom encrypted protocol (avoids detection)
   - Option C: WireGuard-style protocol for better performance

3. **Apple review concerns**
   - Network Extension entitlements require justification
   - Apple scrutinizes VPN apps heavily
   - Must demonstrate legitimate development use case

4. **Distribution method**
   - TestFlight for beta testing
   - App Store requires additional review for Network Extension
   - Alternative: Ad-hoc distribution for enterprise/developer users

5. **Integration with ProxyBot UI**
   - Should iOS VPN toggle appear in Mac's Tauri GUI?
   - Real-time status sync between iOS app and Mac proxy

## Timeline Estimate

| Phase | Duration | Description |
|-------|----------|-------------|
| Research & Design | 1 week | This document + architecture decisions |
| iOS App Development | 2-3 weeks | Main app + extension implementation |
| macOS VPN Server | 1-2 weeks | Add tunnel listener to ProxyBot |
| Integration Testing | 1 week | End-to-end flow verification |
| Apple Approval | 1-2 weeks | Network Extension entitlement approval |

**Total: 6-9 weeks** for production-ready implementation

## References

- [Apple NetworkExtension Documentation](https://developer.apple.com/documentation/networkextension)
- [NEPacketTunnelProvider Reference](https://developer.apple.com/documentation/networkextension/packet-tunnel-provider)
- [NEVPNManager Reference](https://developer.apple.com/documentation/networkextension/nevpnmanager)
- [App Extension Programming Guide](https://developer.apple.com/documentation/appextensions)