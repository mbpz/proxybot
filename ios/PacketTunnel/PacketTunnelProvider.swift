import NetworkExtension
import os.log

/// PacketTunnelProvider — VPN tunnel extension that captures raw IP packets
/// and forwards them over TCP to the ProxyBot Mac bridge.
///
/// Architecture (Atlantis-style):
///   iOS VPN API captures packets → framed over TCP → Mac tunnel_server →
///   proxy pipeline (MITM, classification, etc.)
///
/// This avoids on-device TLS termination — the Mac does all heavy lifting.
class PacketTunnelProvider: NEPacketTunnelProvider {
    private let logger = Logger(
        subsystem: "com.proxybot.app.packetTunnel",
        category: "PacketTunnel"
    )

    // MARK: - Configuration

    /// Default Mac IP on a typical LAN (override via startTunnel options or app message).
    private var macHost: String = "10.0.0.2"

    /// TCP port the Mac tunnel server listens on.
    private let vpnPort: UInt16 = 9999

    /// Active TCP connection to the Mac bridge.
    private var tcpConnection: NWConnection?

    /// Serial queue for connection operations.
    private let connectionQueue = DispatchQueue(label: "com.proxybot.app.tunnel.connection")

    // MARK: - Tunnel Lifecycle

    override func startTunnel(
        options: [String: Any]? = nil,
        completionHandler: @escaping (Error?) -> Void
    ) {
        logger.info("Starting VPN tunnel…")

        // Allow the main app to override the Mac host at start time
        if let host = options?["macHost"] as? String, !host.isEmpty {
            macHost = host
            logger.info("Mac host overridden to \(host)")
        }

        // --- Configure tunnel network settings ---
        let settings = NEPacketTunnelNetworkSettings(tunnelRemoteAddress: "127.0.0.1")

        // Assign a virtual IP to the iOS device (must be on the same subnet as the Mac)
        settings.ipv4Settings = NEIPv4Settings(
            addresses: ["10.0.0.1"],
            subnetMasks: ["255.255.255.0"]
        )

        // Route ALL traffic through the tunnel
        settings.ipv4Settings?.includedRoutes = [NEIPv4Route.default()]

        // Exclude Mac traffic from the VPN to avoid an infinite loop
        settings.ipv4Settings?.excludedRoutes = [
            NEIPv4Route(
                destinationAddress: macHost,
                subnetMask: "255.255.255.255"
            )
        ]

        // Use the Mac as the DNS server so ProxyBot can log DNS queries
        settings.dnsSettings = NEDNSSettings(servers: [macHost])

        // Hand over proxy HTTP/HTTPS settings (optional — most traffic goes raw)
        let proxySettings = NEProxySettings()
        proxySettings.httpEnabled = true
        proxySettings.httpServer = NEProxyServer(address: macHost, port: 8080)
        proxySettings.httpsEnabled = true
        proxySettings.httpsServer = NEProxyServer(address: macHost, port: 8080)
        proxySettings.exceptionList = ["10.0.0.0/8", "192.168.0.0/16", "172.16.0.0/12"]
        settings.proxySettings = proxySettings

        setTunnelNetworkSettings(settings) { [weak self] error in
            guard let self = self else { return }

            if let error = error {
                self.logger.error("Failed to apply tunnel settings: \(error.localizedDescription)")
                completionHandler(error)
                return
            }

            self.logger.info(
                "Tunnel configured. Connecting to Mac bridge at \(self.macHost):\(self.vpnPort)"
            )

            // Establish TCP bridge to Mac
            self.connectToMac()

            // Begin the read-forward loop
            self.readPackets()

            completionHandler(nil)
        }
    }

    override func stopTunnel(
        with reason: NEProviderStopReason,
        completionHandler: @escaping () -> Void
    ) {
        logger.info("Stopping VPN tunnel (reason: \(reason.rawValue))…")
        tcpConnection?.cancel()
        tcpConnection = nil
        completionHandler()
    }

    // MARK: - App Communication

    override func handleAppMessage(
        _ messageData: Data,
        completionHandler: ((Data?) -> Void)?
    ) {
        // IPC from the main ProxyBot app:
        //   "HOST:<ip>"   — update the Mac IP address
        //   "STATUS"      — return connection status
        if let message = String(data: messageData, encoding: .utf8) {
            logger.info("App message received: \(message)")

            if message.hasPrefix("HOST:") {
                let newHost = String(message.dropFirst(5))
                if !newHost.isEmpty {
                    macHost = newHost
                    logger.info("Mac host updated to \(newHost) via app message")
                    // Reconnect with the new host if we're already connected
                    reconnectToMac()
                }
            } else if message == "STATUS" {
                let status = tcpConnection != nil ? "connected" : "disconnected"
                let response = """
                    {"status":"\(status)","host":"\(macHost)","port":\(vpnPort)}
                    """
                completionHandler?(Data(response.utf8))
                return
            }
        }
        completionHandler?(nil)
    }

    // MARK: - TCP Bridge Connection

    private func connectToMac() {
        let endpoint = NWEndpoint.hostPort(
            host: NWEndpoint.Host(macHost),
            port: NWEndpoint.Port(integerLiteral: vpnPort)
        )
        let connection = NWConnection(to: endpoint, using: .tcp)
        self.tcpConnection = connection

        connection.stateUpdateHandler = { [weak self] state in
            guard let self = self else { return }
            switch state {
            case .ready:
                self.logger.info("Connected to Mac VPN bridge at \(self.macHost):\(self.vpnPort)")
            case .failed(let error):
                self.logger.error(
                    "VPN bridge connection failed: \(error.localizedDescription)"
                )
                // Retry after a short delay
                self.scheduleReconnect()
            case .cancelled:
                self.logger.info("VPN bridge disconnected")
            case .waiting(let error):
                self.logger.warning(
                    "VPN bridge waiting: \(error.localizedDescription)"
                )
            default:
                break
            }
        }

        connection.start(queue: connectionQueue)
    }

    private func reconnectToMac() {
        tcpConnection?.cancel()
        tcpConnection = nil
        connectToMac()
    }

    private func scheduleReconnect() {
        connectionQueue.asyncAfter(deadline: .now() + .seconds(3)) { [weak self] in
            guard let self = self else { return }
            self.logger.info("Attempting reconnect to Mac bridge…")
            self.connectToMac()
        }
    }

    // MARK: - Packet Read Loop

    private func readPackets() {
        packetFlow.readPackets { [weak self] packets, protocols in
            guard let self = self else { return }

            // Forward each captured packet to the Mac bridge
            for (index, packet) in packets.enumerated() {
                let protocolNumber = protocols[index].uint8Value
                self.forwardPacket(packet, protocol: protocolNumber)
            }

            // Write back any response packets (empty for now — responses come
            // back from the Mac via the TCP connection and are injected separately)
            self.packetFlow.writePackets([], withProtocols: [])

            // Continue the loop — readPackets is called recursively to keep
            // the flow alive
            self.readPackets()
        }
    }

    // MARK: - Packet Forwarding

    /// Build a framed packet and send it over the TCP bridge to the Mac.
    ///
    /// Wire format (big-endian):
    ///   [length: u32][protocol: u8][src_ip: 4B][src_port: u16]
    ///   [dst_ip: 4B][dst_port: u16][payload: N bytes]
    ///
    /// length = 13 + payload.count (the 13-byte header)
    private func forwardPacket(_ data: Data, protocol protocolNumber: UInt8) {
        guard let connection = tcpConnection else {
            logger.debug("No bridge connection — dropping packet")
            return
        }

        var frame = Data()

        // --- Extract IP header fields ---
        // IPv4 header layout:
        //   bytes 0-3:  version/ihl/dscp/total_length
        //   bytes 4-7:  identification/flags/fragment_offset
        //   bytes 8-11: ttl/protocol/header_checksum
        //   bytes 12-15: source IP
        //   bytes 16-19: destination IP
        var srcIP: [UInt8] = [0, 0, 0, 0]
        var dstIP: [UInt8] = [0, 0, 0, 0]
        var srcPort: UInt16 = 0
        var dstPort: UInt16 = 0

        if data.count >= 20 {
            // Extract IPs
            srcIP = [data[12], data[13], data[14], data[15]]
            dstIP = [data[16], data[17], data[18], data[19]]

            // Extract ports from TCP/UDP header
            // The IP header length field (lower nibble of byte 0) * 4 gives the
            // IHL in bytes. For standard IPv4 without options this is 20.
            let ihl = Int((data[0] & 0x0F)) * 4
            if data.count >= ihl + 4 {
                // TCP/UDP ports are the first two 16-bit fields in the transport header
                srcPort = (UInt16(data[ihl]) << 8) | UInt16(data[ihl + 1])
                dstPort = (UInt16(data[ihl + 2]) << 8) | UInt16(data[ihl + 3])
            }
        }

        // --- Build frame ---
        let payloadLength = UInt32(13 + data.count)
        frame.append(payloadLength.bigEndianData)          // frame length (u32 BE)
        frame.append(protocolNumber)                        // IP protocol number
        frame.append(contentsOf: srcIP)                     // source IP (4 bytes)
        frame.append(srcPort.bigEndianData)                 // source port (u16 BE)
        frame.append(contentsOf: dstIP)                     // dest IP (4 bytes)
        frame.append(dstPort.bigEndianData)                 // dest port (u16 BE)
        frame.append(data)                                  // raw IP packet payload

        // --- Send ---
        connection.send(
            content: frame,
            completion: .contentProcessed { [weak self] error in
                if let error = error {
                    self?.logger.error(
                        "Send failed: \(error.localizedDescription)"
                    )
                }
            }
        )
    }
}

// MARK: - Convenience Extensions

extension UInt16 {
    /// Return the big-endian byte representation as 2-byte Data.
    var bigEndianData: Data {
        var value = self.bigEndian
        return Data(bytes: &value, count: MemoryLayout<UInt16>.size)
    }
}

extension UInt32 {
    /// Return the big-endian byte representation as 4-byte Data.
    var bigEndianData: Data {
        var value = self.bigEndian
        return Data(bytes: &value, count: MemoryLayout<UInt32>.size)
    }
}
