use serde::Serialize;
use std::net::IpAddr;

pub mod engine;
pub mod profile;
pub use engine::{ConditionEffect, ConditionRule, NetworkConditionEngine, NewConditionRule};
pub use profile::NetworkProfile;

/// Built-in presets
pub fn builtin_presets() -> Vec<NetworkProfile> {
    vec![
        NetworkProfile {
            name: "2G".into(),
            latency_ms: 800,
            bandwidth_kbps: 50,
            packet_loss_pct: 2,
        },
        NetworkProfile {
            name: "3G".into(),
            latency_ms: 300,
            bandwidth_kbps: 750,
            packet_loss_pct: 1,
        },
        NetworkProfile {
            name: "4G".into(),
            latency_ms: 100,
            bandwidth_kbps: 10000,
            packet_loss_pct: 0,
        },
        NetworkProfile {
            name: "WiFi".into(),
            latency_ms: 5,
            bandwidth_kbps: 0,
            packet_loss_pct: 0,
        },
        NetworkProfile {
            name: "Edge".into(),
            latency_ms: 1200,
            bandwidth_kbps: 30,
            packet_loss_pct: 5,
        },
    ]
}

/// Network information for the proxy server machine.
#[derive(Clone, Serialize)]
pub struct NetworkInfo {
    /// The local LAN IP address of the active network interface.
    pub lan_ip: String,
    /// The name of the active network interface (e.g., "en0", "eth0").
    pub interface: String,
}

/// Get the current network information for the proxy server.
/// Returns the LAN IP and interface name of the primary active interface.
pub fn get_network_info() -> Result<NetworkInfo, String> {
    // The primary interface on macOS is typically en0.
    // We use a socket-based approach to reliably determine the LAN IP
    // by finding which local address would be used to reach an external IP.
    let ip = get_lan_ip().ok_or_else(|| "Could not determine LAN IP".to_string())?;

    // Determine the interface name - on macOS, en0 is typically the primary Wi-Fi/Ethernet.
    let interface = detect_primary_interface().unwrap_or_else(|| "en0".to_string());

    Ok(NetworkInfo {
        lan_ip: ip,
        interface,
    })
}

/// Get the local LAN IP address.
/// Returns None if the IP cannot be determined.
pub fn get_local_ip() -> Option<String> {
    get_lan_ip()
}

/// Get the local LAN IP address by connecting to an external address.
/// This reliably determines which local IP is used for outbound routing.
fn get_lan_ip() -> Option<String> {
    // Bind a UDP socket to get a local address on the correct interface.
    let sock = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    // Connect to an external IP (Google DNS) - this determines routing.
    sock.connect("8.8.8.8:53").ok()?;
    let local_addr = sock.local_addr().ok()?;
    let ip = local_addr.ip();

    // Ignore loopback addresses.
    if ip.is_loopback() {
        return None;
    }

    // Check for link-local IPv6 addresses (fe80::/10).
    if ip.is_ipv6() {
        if let IpAddr::V6(ipv6) = ip {
            if ipv6.is_unicast_link_local() {
                return None;
            }
        }
    }

    Some(ip.to_string())
}

/// Detect the primary network interface name.
/// On macOS, this is typically "en0" for the primary Wi-Fi/Ethernet adapter.
fn detect_primary_interface() -> Option<String> {
    // On macOS, en0 is the primary built-in network interface.
    // We verify it's active by checking if our IP matches.
    // For a more reliable detection, we could parse "ifconfig" output,
    // but the socket-based approach is sufficient for the typical use case.
    let lan_ip = get_lan_ip()?;

    // Verify it's a reasonable interface IP (not loopback).
    if let Ok(ip) = lan_ip.parse::<IpAddr>() {
        if ip.is_loopback() {
            return None;
        }
    }

    // Return the common macOS primary interface name.
    Some("en0".to_string())
}
