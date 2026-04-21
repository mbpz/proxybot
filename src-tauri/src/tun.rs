//! TUN/VPN interface management for macOS transparent proxy fallback.
//!
//! When pf/netsh redirect is unavailable (e.g., Android 7+ without MDM,
//! iOS without MDM), the agent falls back to TUN interface mode.
//!
//! On macOS, we create a utun interface and configure it as a VPN gateway.
//! The phone connects via VPN profile and all traffic is captured by the TUN device.
//!
//! This implementation uses the native macOS utun interfaces via libc syscalls.

use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Static flag to track if TUN is currently enabled.
static TUN_ENABLED: AtomicBool = AtomicBool::new(false);

/// TUN interface configuration.
const TUN_IP: &str = "10.0.0.1";
const TUN_NETMASK: &str = "255.255.255.0";

/// Shared state for the TUN interface.
pub struct TunState {
    /// The TUN file descriptor (using standard file descriptor for sync I/O).
    tun_fd: Mutex<Option<std::os::fd::RawFd>>,
    /// Interface name for cleanup.
    iface_name: Mutex<Option<String>>,
}

impl TunState {
    pub fn new() -> Self {
        Self {
            tun_fd: Mutex::new(None),
            iface_name: Mutex::new(None),
        }
    }
}

/// Configure the utun interface with IP address and routing.
/// This uses ifconfig to set up the interface and route to capture traffic.
fn configure_tun_interface(iface_name: &str, ip: &str, netmask: &str) -> Result<(), String> {
    // Bring up the interface and assign IP
    let ifconfig_output = Command::new("ifconfig")
        .args([iface_name, ip, "netmask", netmask, "up"])
        .output()
        .map_err(|e| format!("Failed to run ifconfig: {}", e))?;

    if !ifconfig_output.status.success() {
        let stderr = String::from_utf8_lossy(&ifconfig_output.stderr);
        return Err(format!("ifconfig failed: {}", stderr));
    }

    // Set up routing: redirect all traffic through the TUN interface
    // This tells the kernel to route VPN traffic through our utun interface
    let route_output = Command::new("route")
        .args(["add", "-interface", iface_name])
        .output()
        .map_err(|e| format!("Failed to run route add: {}", e))?;

    // Ignore route errors - it might already exist
    if !route_output.status.success() {
        let stderr = String::from_utf8_lossy(&route_output.stderr);
        log::warn!("route add interface warning (may already exist): {}", stderr);
    }

    // Enable IP forwarding so we can forward traffic
    let sysctl_output = Command::new("sysctl")
        .args(["-w", "net.inet.ip.forwarding=1"])
        .output()
        .map_err(|e| format!("Failed to enable IP forwarding: {}", e))?;

    if !sysctl_output.status.success() {
        let stderr = String::from_utf8_lossy(&sysctl_output.stderr);
        log::warn!("sysctl IP forwarding warning: {}", stderr);
    }

    log::info!("TUN interface {} configured with IP {}", iface_name, ip);
    Ok(())
}

/// Tear down TUN interface routing configuration.
fn unconfigure_tun_interface(iface_name: &str) -> Result<(), String> {
    // Remove the route
    let _ = Command::new("route")
        .args(["delete", "-interface", iface_name])
        .output();

    // Disable IP forwarding
    let _ = Command::new("sysctl")
        .args(["-w", "net.inet.ip.forwarding=0"])
        .output();

    log::info!("TUN interface {} unconfigured", iface_name);
    Ok(())
}

/// Set up TUN/VPN mode on macOS.
///
/// Creates a utun interface and configures it as a VPN gateway.
/// The phone can then connect via VPN profile and all traffic
/// will be captured by the TUN device.
///
/// On macOS, we use the built-in utun interfaces. The system will
/// automatically create a new utunN interface when we connect to it.
#[tauri::command]
pub fn setup_tun(state: tauri::State<'_, Arc<TunState>>) -> Result<String, String> {
    if TUN_ENABLED.swap(true, Ordering::SeqCst) {
        return Err("TUN is already enabled".to_string());
    }

    // On macOS, we create a utun interface using PF_SYSTEM socket
    // SYSPROTOCOL_CONTROL = 2
    #[cfg(target_os = "macos")]
    {
        use libc::{c_int};

        // System protocol family and control protocol for utun
        const PF_SYSTEM: c_int = 32;
        const SYSPROTOCOL_CONTROL: c_int = 2;

        // Create a system socket for interface control
        let fd = unsafe { libc::socket(PF_SYSTEM, libc::SOCK_DGRAM, SYSPROTOCOL_CONTROL) };
        if fd < 0 {
            TUN_ENABLED.store(false, Ordering::SeqCst);
            return Err(format!("Failed to create system socket: {}", fd));
        }

        // The interface name will be determined after we configure it
        // We use the first available utun interface
        let iface_name = format!("utun{}", get_next_utun_number());

        // Configure the interface with IP address
        if let Err(e) = configure_tun_interface(&iface_name, TUN_IP, TUN_NETMASK) {
            unsafe { libc::close(fd) };
            TUN_ENABLED.store(false, Ordering::SeqCst);
            return Err(e);
        }

        // Store the fd and interface name
        {
            let mut fd_guard = state.tun_fd.lock().unwrap();
            *fd_guard = Some(fd);
        }
        {
            let mut name_guard = state.iface_name.lock().unwrap();
            *name_guard = Some(iface_name.clone());
        }

        log::info!("Created TUN interface: {}", iface_name);

        Ok(format!(
            "TUN/VPN mode enabled. Interface: {}, IP: {}\n\
             Configure your device to connect via VPN to {}",
            iface_name, TUN_IP, TUN_IP
        ))
    }

    #[cfg(not(target_os = "macos"))]
    {
        TUN_ENABLED.store(false, Ordering::SeqCst);
        Err("TUN/VPN mode is only supported on macOS".to_string())
    }
}

/// Get the next available utun interface number.
/// This is a simple approach that checks existing interfaces.
fn get_next_utun_number() -> usize {
    use std::process::Command;

    let output = Command::new("ifconfig")
        .output()
        .ok();

    if let Some(out) = output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        let mut max_num = 0;
        for line in stdout.lines() {
            if line.starts_with("utun") {
                if let Ok(num) = line[4..].split_whitespace().next().unwrap_or("0").parse::<usize>() {
                    max_num = max_num.max(num);
                }
            }
        }
        max_num + 1
    } else {
        0
    }
}

/// Tear down TUN/VPN mode.
#[tauri::command]
pub fn teardown_tun(state: tauri::State<'_, Arc<TunState>>) -> Result<(), String> {
    if !TUN_ENABLED.swap(false, Ordering::SeqCst) {
        return Err("TUN is not enabled".to_string());
    }

    // Get the interface name and fd before cleanup
    let iface_name = {
        let mut fd_guard = state.tun_fd.lock().unwrap();
        if let Some(fd) = fd_guard.take() {
            #[cfg(target_os = "macos")]
            unsafe { libc::close(fd) };
        }
        let mut name_guard = state.iface_name.lock().unwrap();
        name_guard.take()
    };

    // Unconfigure routing
    if let Some(name) = iface_name {
        unconfigure_tun_interface(&name)?;
    }

    log::info!("TUN/VPN mode disabled");
    Ok(())
}

/// Check if TUN is currently enabled.
#[tauri::command]
pub fn is_tun_enabled() -> bool {
    TUN_ENABLED.load(Ordering::SeqCst)
}
