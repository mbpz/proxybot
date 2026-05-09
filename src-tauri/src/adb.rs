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

impl AdbDevice {
    /// Execute a shell command on the device and return the output.
    pub async fn shell(&self, command: &str) -> Result<String, String> {
        let output = tokio::process::Command::new("adb")
            .args(["-s", &self.serial, "shell", command])
            .output()
            .await
            .map_err(|e| format!("Failed to execute adb shell: {}", e))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("adb shell failed: {}", stderr))
        }
    }

    /// List running processes on the device
    pub async fn list_processes(&self) -> Result<Vec<ProcessInfo>, String> {
        let output = self.shell("ps -A -o USER,PID,NAME").await?;
        Ok(parse_adb_ps_output(&output))
    }
}

/// Process info from `adb shell ps`
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub user: String, // Android user (e.g., u0_a123) - NOT the app package name
    pub name: String,
}

/// Parse output of `adb shell ps -A -o USER,PID,NAME`
/// Output format:
/// USER           PID  NAME
/// u0_a123        4567  com.example.app
pub fn parse_adb_ps_output(output: &str) -> Vec<ProcessInfo> {
    output
        .lines()
        .skip(1) // skip header line "USER           PID  NAME"
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let pid: u32 = parts[1].parse().ok()?;
                Some(ProcessInfo {
                    pid,
                    user: parts[0].to_string(),
                    name: parts[2..].join(" "), // Name may have spaces
                })
            } else {
                None
            }
        })
        .collect()
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
    fn test_parse_adb_ps_output() {
        let output = r#"USER           PID  NAME
u0_a123        4567  com.example.app
u0_a456        7890  com.another.app"#;
        let processes = parse_adb_ps_output(output);
        assert_eq!(processes.len(), 2);
        assert_eq!(processes[0].pid, 4567);
        assert_eq!(processes[0].user, "u0_a123");
        assert_eq!(processes[0].name, "com.example.app");
    }

    #[test]
    fn test_adb_device_shell_method() {
        // This test verifies the shell method exists on AdbDevice
        let device = AdbDevice {
            serial: "test123".to_string(),
            status: "device".to_string(),
            product: None,
            model: None,
        };
        // The shell method should be available (test passes if it compiles)
        let _ = device.shell("echo test");
    }

    #[test]
    fn test_adb_device_list_processes() {
        let device = AdbDevice {
            serial: "test123".to_string(),
            status: "device".to_string(),
            product: None,
            model: None,
        };
        // The list_processes method should be available (test passes if it compiles)
        let _ = device.list_processes();
    }
}