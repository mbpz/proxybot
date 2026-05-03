import UIKit
import NetworkExtension

class AppDelegate: UIResponder, UIApplicationDelegate {
    var vpnManager: NETunnelProviderManager?

    func application(_ application: UIApplication, didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?) -> Bool {
        loadVPNConfiguration()
        return true
    }

    private func loadVPNConfiguration() {
        NETunnelProviderManager.loadAllFromPreferences { [weak self] managers, error in
            if let error = error {
                print("Failed to load VPN: \(error)")
                return
            }

            self?.vpnManager = managers?.first ?? NETunnelProviderManager()
        }
    }

    func toggleVPN() {
        guard let manager = vpnManager else { return }

        if manager.connection.status == .connected {
            manager.connection.stopVPNTunnel()
        } else {
            do {
                try manager.connection.startVPNTunnel()
            } catch {
                print("Failed to start VPN: \(error)")
            }
        }
    }
}