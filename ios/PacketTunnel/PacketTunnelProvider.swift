import NetworkExtension
import os.log

class PacketTunnelProvider: NEPacketTunnelProvider {
    private let logger = Logger(subsystem: "com.proxybot.app.packetTunnel", category: "PacketTunnel")

    override func startTunnel(options: [String : Any]?, completionHandler: @escaping (Error?) -> Void) {
        logger.info("Starting tunnel...")

        let settings = NEPacketTunnelNetworkSettings(tunnelRemoteAddress: "127.0.0.1")

        settings.ipv4Settings = NEIPv4Settings(addresses: ["10.0.0.1"], subnetMasks: ["255.255.255.0"])
        settings.ipv4Settings?.includedRoutes = [NEIPv4Route.default()]

        setTunnelNetworkSettings(settings) { error in
            if let error = error {
                self.logger.error("Failed to set tunnel settings: \(error.localizedDescription)")
                completionHandler(error)
                return
            }

            self.logger.info("Tunnel settings configured successfully")

            // Start reading packets
            self.readPackets()

            completionHandler(nil)
        }
    }

    private func readPackets() {
        packetFlow.readPackets { [weak self] packets, protocols in
            guard let self = self else { return }

            for (index, packet) in packets.enumerated() {
                let protocolNumber = protocols[index]
                self.processPacket(packet, protocolNumber: protocolNumber)
            }

            // Continue reading
            self.readPackets()
        }
    }

    private func processPacket(_ packet: Data, protocolNumber: NSNumber) {
        // Forward packet to ProxyBot on Mac
        // TODO: Connect via local TCP (Mac IP on LAN)
    }

    override func stopTunnel() {
        logger.info("Stopping tunnel...")
    }

    override func handleAppMessage(_ messageData: Data, completionHandler: ((Data?) -> Void)?) {
        // IPC with main app
        completionHandler?(nil)
    }
}