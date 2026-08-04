//! TUN/VPN interface management for macOS transparent proxy fallback.
//!
//! When pf/netsh redirect is unavailable (e.g., Android 7+ without MDM,
//! iOS without MDM), the agent falls back to TUN interface mode.
//!
//! On macOS, we create a utun interface and configure it as a VPN gateway.
//! The phone connects via VPN profile and all traffic is captured by the TUN device.

use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tun::platform::Device as TunDevice;
use tun::Device;

use tun::Configuration as TunConfig;

/// TUN interface configuration.
const TUN_IP: &str = "10.0.0.1";
const TUN_NETMASK: &str = "255.255.255.0";

/// Shared state for the TUN interface.
pub struct TunState {
    enabled: AtomicBool,
    operation: Mutex<()>,
    /// Own the device for exactly as long as the interface is enabled.
    device: Mutex<Option<TunDevice>>,
    /// Interface name for cleanup.
    iface_name: Mutex<Option<String>>,
}

impl TunState {
    pub fn new() -> Self {
        Self {
            enabled: AtomicBool::new(false),
            operation: Mutex::new(()),
            device: Mutex::new(None),
            iface_name: Mutex::new(None),
        }
    }

    pub(crate) fn shutdown(&self) -> Result<(), String> {
        let _operation = self.operation.lock().unwrap();
        if !self.enabled.load(Ordering::SeqCst) {
            return Ok(());
        }

        let device = self.device.lock().unwrap().take();
        let iface_name = self.iface_name.lock().unwrap().take();
        let result = iface_name
            .as_deref()
            .map(unconfigure_tun_interface)
            .transpose()
            .map(|_| ());
        drop(device);
        self.enabled.store(false, Ordering::SeqCst);
        result
    }
}

impl Default for TunState {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Admin command helper
// =============================================================================

/// Run a command with administrator privileges via AppleScript.
fn run_admin_command(script: &str) -> Result<String, String> {
    log::info!("[tun] Running admin command: {}", script);
    let output = Command::new("osascript")
        .args([
            "-e",
            &format!(
                "do shell script \"{}\" with administrator privileges",
                script.replace('"', "\"\\\"\"")
            ),
        ])
        .output()
        .map_err(|e| format!("Failed to run osascript: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        log::error!("[tun] Admin command failed: {}", stderr);
        return Err(format!("Admin command failed: {}", stderr));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    log::info!("[tun] Admin command stdout: {}", stdout.trim());
    Ok(stdout.into_owned())
}

// =============================================================================
// Interface configuration
// =============================================================================

/// Configure the utun interface with IP address and routing.
fn configure_tun_interface(iface_name: &str, ip: &str, netmask: &str) -> Result<(), String> {
    log::info!(
        "[tun] Configuring {} with ip={}, netmask={}",
        iface_name,
        ip,
        netmask
    );

    // Bring up the interface and assign IP — requires admin privileges
    let ifconfig_script = format!(
        "/usr/sbin/ifconfig {} {} netmask {} up",
        iface_name, ip, netmask
    );
    run_admin_command(&ifconfig_script)?;
    log::info!("[tun] ifconfig up succeeded for {}", iface_name);

    // Set up routing: redirect all traffic through the TUN interface
    log::info!("[tun] Adding route for interface {}", iface_name);
    let route_output = Command::new("route")
        .args(["add", "-interface", iface_name])
        .output();

    if let Ok(out) = route_output {
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            log::warn!("[tun] route add warning (may already exist): {}", stderr);
        } else {
            log::info!("[tun] route add succeeded");
        }
    }

    // Enable IP forwarding — requires admin privileges
    log::info!("[tun] Enabling IP forwarding");
    run_admin_command("/usr/sbin/sysctl -w net.inet.ip.forwarding=1")?;
    log::info!("[tun] IP forwarding enabled");

    log::info!("[tun] Interface {} configured successfully", iface_name);
    Ok(())
}

/// Tear down TUN interface routing configuration.
fn unconfigure_tun_interface(iface_name: &str) -> Result<(), String> {
    log::info!("[tun] Unconfiguring TUN interface: {}", iface_name);

    // Remove the route
    let _ = Command::new("route")
        .args(["delete", "-interface", iface_name])
        .output();

    // Disable IP forwarding — requires admin privileges
    let _ = run_admin_command("/usr/sbin/sysctl -w net.inet.ip.forwarding=0");

    log::info!("[tun] TUN interface {} unconfigured", iface_name);
    Ok(())
}

// =============================================================================
// TUN setup/teardown commands
// =============================================================================

/// Set up TUN/VPN mode on macOS.
///
/// Creates a utun interface and configures it as a VPN gateway.
/// The phone can then connect via VPN profile and all traffic
/// will be captured by the TUN device.
#[tauri::command]
pub fn setup_tun(state: tauri::State<'_, Arc<TunState>>) -> Result<String, String> {
    let _operation = state.operation.lock().unwrap();
    if state.enabled.load(Ordering::SeqCst) {
        return Err("TUN is already enabled".to_string());
    }

    log::info!("[tun] Setting up TUN/VPN mode...");

    #[cfg(target_os = "macos")]
    {
        let iface_name = format!("utun{}", 0);

        let mut cfg = TunConfig::default();
        cfg.name(&iface_name)
            .address(TUN_IP)
            .netmask(TUN_NETMASK)
            .up();

        log::info!(
            "[tun] Creating TUN device: name={}, ip={}, netmask={}",
            iface_name,
            TUN_IP,
            TUN_NETMASK
        );

        // Use the tun crate's platform-specific Device
        let dev = match TunDevice::new(&cfg) {
            Ok(dev) => {
                let name = dev.name().to_string();
                log::info!("[tun] TUN device created: {}", name);
                dev
            }
            Err(e) => {
                log::error!("[tun] Failed to create TUN device: {}", e);
                return Err(format!(
                    "Failed to create TUN device: {}. \
                    Make sure you have administrator privileges.",
                    e
                ));
            }
        };

        let actual_name = dev.name().to_string();
        log::info!("[tun] TUN device actual name: {}", actual_name);

        // Configure routing (requires admin)
        if let Err(e) = configure_tun_interface(&actual_name, TUN_IP, TUN_NETMASK) {
            log::error!("[tun] Failed to configure TUN interface: {}", e);
            if let Err(cleanup_error) = unconfigure_tun_interface(&actual_name) {
                log::warn!(
                    "[tun] Failed to roll back partially configured interface {actual_name}: {cleanup_error}"
                );
            }
            drop(dev);
            return Err(e);
        }

        *state.device.lock().unwrap() = Some(dev);
        {
            let mut name_guard = state.iface_name.lock().unwrap();
            *name_guard = Some(actual_name.clone());
        }
        state.enabled.store(true, Ordering::SeqCst);

        log::info!(
            "[tun] TUN/VPN mode enabled successfully. Interface: {}",
            actual_name
        );

        Ok(format!(
            "TUN/VPN mode enabled. Interface: {}, IP: {}\n\
             Configure your device to connect via VPN to {}",
            actual_name, TUN_IP, TUN_IP
        ))
    }

    #[cfg(not(target_os = "macos"))]
    {
        Err("TUN/VPN mode is only supported on macOS".to_string())
    }
}

/// Tear down TUN/VPN mode.
#[tauri::command]
pub fn teardown_tun(state: tauri::State<'_, Arc<TunState>>) -> Result<(), String> {
    state.shutdown()?;
    log::info!("[tun] TUN/VPN mode disabled");
    Ok(())
}

/// Check if TUN is currently enabled.
#[tauri::command]
pub fn is_tun_enabled(state: tauri::State<'_, Arc<TunState>>) -> bool {
    state.enabled.load(Ordering::SeqCst)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shutdown_is_idempotent_when_no_device_is_owned() {
        let state = TunState::new();

        state.shutdown().unwrap();
        state.shutdown().unwrap();

        assert!(!state.enabled.load(Ordering::SeqCst));
        assert!(state.device.lock().unwrap().is_none());
        assert!(state.iface_name.lock().unwrap().is_none());
    }
}
