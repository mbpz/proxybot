//! Android ADB reverse tunnel support for ProxyBot.
//!
//! Allows Android phones to proxy traffic via USB (adb reverse) instead of WiFi.

use std::collections::HashMap;
use std::process::Command;

/// ADB device information.
#[derive(Debug, Clone)]
pub struct AdbDevice {
    pub serial: String,
    pub status: String,
    pub product: Option<String>,
    pub model: Option<String>,
}

/// ADB state managing devices and reverse tunnels.
pub struct AdbState {
    /// List of connected ADB devices.
    pub devices: Vec<AdbDevice>,
    /// Map of serial -> tunnel active status.
    pub reverse_tunnels: HashMap<String, bool>,
    /// Whether ADB mode is enabled globally.
    pub enabled: bool,
}

impl Default for AdbState {
    fn default() -> Self {
        Self {
            devices: Vec::new(),
            reverse_tunnels: HashMap::new(),
            enabled: false,
        }
    }
}

/// Check if ADB is available on the system.
pub fn is_adb_available() -> bool {
    Command::new("adb")
        .arg("version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// List connected ADB devices.
/// Parses output from `adb devices -l` which looks like:
/// serial product:model:device
pub fn list_devices() -> Vec<AdbDevice> {
    let output = match Command::new("adb").args(["devices", "-l"]).output() {
        Ok(o) if o.status.success() => o,
        _ => return Vec::new(),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut devices = Vec::new();

    for line in stdout.lines().skip(1) {
        // Skip header line "List of devices attached"
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }

        let serial = parts[0].to_string();
        let status = parts[1].to_string();

        // Parse product:model:device from -l output
        let mut product = None;
        let mut model = None;

        if let Some(rest) = parts.get(2) {
            for kv in rest.split(':') {
                if kv.starts_with("product:") {
                    product = Some(kv.trim_start_matches("product:").to_string());
                } else if kv.starts_with("model:") {
                    model = Some(kv.trim_start_matches("model:").to_string());
                }
            }
        }

        devices.push(AdbDevice {
            serial,
            status,
            product,
            model,
        });
    }

    devices
}

/// Set up a reverse tunnel for the given device serial.
/// This maps localhost:8088 on the device to localhost:8088 on the host.
pub fn setup_reverse(serial: &str) -> Result<(), String> {
    let output = Command::new("adb")
        .args(["-s", serial, "reverse", "tcp:8088", "tcp:8088"])
        .output()
        .map_err(|e| format!("Failed to execute adb: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("adb reverse failed: {}", stderr))
    }
}

/// Remove the reverse tunnel for the given device serial.
pub fn remove_reverse(serial: &str) -> Result<(), String> {
    let output = Command::new("adb")
        .args(["-s", serial, "reverse", "--remove", "tcp:8088"])
        .output()
        .map_err(|e| format!("Failed to execute adb: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("adb reverse remove failed: {}", stderr))
    }
}

/// Get the tunnel status for a device.
pub fn is_tunnel_active(serial: &str, tunnels: &HashMap<String, bool>) -> bool {
    tunnels.get(serial).copied().unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adb_state_default() {
        let state = AdbState::default();
        assert!(state.devices.is_empty());
        assert!(state.reverse_tunnels.is_empty());
        assert!(!state.enabled);
    }

    #[test]
    fn test_is_tunnel_active() {
        let mut tunnels = HashMap::new();
        tunnels.insert("abc123".to_string(), true);
        tunnels.insert("def456".to_string(), false);

        assert!(is_tunnel_active("abc123", &tunnels));
        assert!(!is_tunnel_active("def456", &tunnels));
        assert!(!is_tunnel_active("unknown", &tunnels));
    }
}