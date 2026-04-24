//! TUN/VPN interface management for macOS transparent proxy fallback.
//!
//! When pf/netsh redirect is unavailable (e.g., Android 7+ without MDM,
//! iOS without MDM), the agent falls back to TUN interface mode.
//!
//! On macOS, we create a utun interface and configure it as a VPN gateway.
//! The phone connects via VPN profile and all traffic is captured by the TUN device.

use std::os::fd::AsRawFd;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tun::platform::Device as TunDevice;
use tun::Device;

use tun::Configuration as TunConfig;

/// Static flag to track if TUN is currently enabled.
static TUN_ENABLED: AtomicBool = AtomicBool::new(false);

/// TUN interface configuration.
const TUN_IP: &str = "10.0.0.1";
const TUN_NETMASK: &str = "255.255.255.0";

/// Shared state for the TUN interface.
pub struct TunState {
    /// The TUN device file descriptor.
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
        iface_name, ip, netmask
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
    if TUN_ENABLED.swap(true, Ordering::SeqCst) {
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
            iface_name, TUN_IP, TUN_NETMASK
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
                TUN_ENABLED.store(false, Ordering::SeqCst);
                return Err(format!(
                    "Failed to create TUN device: {}. \
                    Make sure you have administrator privileges.",
                    e
                ));
            }
        };

        let actual_name = dev.name().to_string();
        let fd = dev.as_raw_fd();
        log::info!("[tun] TUN device actual name: {}, fd: {}", actual_name, fd);

        // Configure routing (requires admin)
        if let Err(e) = configure_tun_interface(&actual_name, TUN_IP, TUN_NETMASK) {
            log::error!("[tun] Failed to configure TUN interface: {}", e);
            // Leak the device intentionally so the fd stays open
            std::mem::forget(dev);
            TUN_ENABLED.store(false, Ordering::SeqCst);
            return Err(e);
        }

        // Store fd and name
        {
            let mut fd_guard = state.tun_fd.lock().unwrap();
            *fd_guard = Some(fd);
        }
        {
            let mut name_guard = state.iface_name.lock().unwrap();
            *name_guard = Some(actual_name.clone());
        }

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
        TUN_ENABLED.store(false, Ordering::SeqCst);
        Err("TUN/VPN mode is only supported on macOS".to_string())
    }
}

/// Tear down TUN/VPN mode.
#[tauri::command]
pub fn teardown_tun(state: tauri::State<'_, Arc<TunState>>) -> Result<(), String> {
    if !TUN_ENABLED.swap(false, Ordering::SeqCst) {
        return Err("TUN is not enabled".to_string());
    }

    let iface_name = {
        let mut fd_guard = state.tun_fd.lock().unwrap();
        if let Some(fd) = fd_guard.take() {
            log::info!("[tun] Closing TUN fd: {}", fd);
            unsafe { libc::close(fd) };
        }
        let mut name_guard = state.iface_name.lock().unwrap();
        name_guard.take()
    };

    if let Some(name) = iface_name {
        unconfigure_tun_interface(&name)?;
    }

    log::info!("[tun] TUN/VPN mode disabled");
    Ok(())
}

/// Check if TUN is currently enabled.
#[tauri::command]
pub fn is_tun_enabled() -> bool {
    TUN_ENABLED.load(Ordering::SeqCst)
}
