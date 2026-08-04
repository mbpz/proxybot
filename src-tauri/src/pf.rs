//! pf (Packet Filter) management for macOS transparent proxy.
//!
//! This module handles setting up and tearing down pf rules to redirect
//! incoming TCP traffic on ports 80 and 443 to the local proxy listening on port 8088.
//!
//! Privilege escalation is performed using osascript to prompt the user
//! with a system authentication dialog, providing better UX than raw sudo.

use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};

use proxybot_core::AppConfig;

/// Process-local ownership for ProxyBot's PF anchor.
pub struct PfRuntimeState {
    pub(crate) operation: tokio::sync::Mutex<()>,
    enabled: AtomicBool,
}

impl PfRuntimeState {
    pub fn new(config: &AppConfig) -> Self {
        Self {
            operation: tokio::sync::Mutex::new(()),
            enabled: AtomicBool::new(config.pf_anchor_file.exists() && is_system_pf_enabled()),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Acquire)
    }

    pub(crate) fn mark_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Release);
    }
}

/// Set up pf rules for transparent proxying.
/// Redirects TCP traffic on ports 80 and 443 to the local proxy on port 8088.
/// Requires administrator privileges via osascript prompt.
pub fn setup_pf(interface: String, local_ip: String, config: &AppConfig) -> Result<String, String> {
    // Validate interface name - must be alphanumeric only to prevent command injection.
    if !interface.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err("Invalid interface name".to_string());
    }
    if interface.is_empty() || interface.len() > 10 {
        return Err("Invalid interface name".to_string());
    }
    // Validate IP — only digits and dots allowed.
    if !local_ip.chars().all(|c| c.is_ascii_digit() || c == '.') || local_ip.is_empty() {
        return Err("Invalid local IP address".to_string());
    }

    // Write rules to a temp file first (no root needed for /tmp).
    let tmp_file = "/tmp/proxybot.pf.conf";
    let rules = format!(
        "rdr on {iface} proto tcp from any to any port {{80,443}} -> {ip} port {port}\nrdr on {iface} proto udp from any to any port 53 -> {ip} port {dns_port}\npass on {iface} proto tcp from any to any port {{80,443}}\n",
        iface = interface,
        port = config.proxy_port,
        dns_port = config.dns_port,
        ip = local_ip,
    );
    fs::write(tmp_file, &rules).map_err(|e| format!("Failed to write temp pf rules: {}", e))?;

    // Privileged shell: copy to /etc/pf.anchors, load rules, enable pf.
    let privileged_script = format!(
        r#"do shell script "mkdir -p /etc/pf.anchors && cp {tmp} {anchor_file} && sysctl -w net.inet.ip.forwarding=1 && pfctl -a {anchor} -f {anchor_file} && pfctl -e; echo done" with administrator privileges"#,
        tmp = tmp_file,
        anchor_file = config.pf_anchor_file.display(),
        anchor = config.pf_anchor_name,
    );

    // Execute the privileged operations via osascript.
    let output = Command::new("osascript")
        .args(["-e", &privileged_script])
        .output()
        .map_err(|e| format!("Failed to execute osascript: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        return Err(format!("pf setup failed: {}{}", stderr, stdout));
    }

    log::info!("pf rules loaded successfully via osascript");
    Ok(format!(
        "Transparent proxy enabled. Redirecting {} traffic on ports 80/443 to port {}",
        interface, config.proxy_port
    ))
}

/// Tear down pf rules and disable IP forwarding.
pub fn teardown_pf(config: &AppConfig) -> Result<(), String> {
    // Build the osascript command to remove rules and disable pf.
    let privileged_script = r#"do shell script "
        # Remove pf rules
        pfctl -a com.apple/proxybot -F all 2>&1 || true

        # Disable pf
        pfctl -d 2>&1 || true

        # Reset IP forwarding (ProxyBot is done with it)
        sysctl -w net.inet.ip.forwarding=0 2>/dev/null || true

        echo 'pf teardown complete'
    " with administrator privileges
    "#;

    // Execute the privileged operations via osascript.
    let output = Command::new("osascript")
        .args(["-e", privileged_script])
        .output()
        .map_err(|e| format!("Failed to execute osascript: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!("pf teardown failed: {}{}", stderr, stdout));
    }

    // Clean up the anchor file.
    let _ = fs::remove_file(&config.pf_anchor_file);

    log::info!("pf rules removed successfully via osascript");
    Ok(())
}

/// Check if pf is currently enabled.
fn is_system_pf_enabled() -> bool {
    let output = Command::new("pfctl").args(["-s", "info"]).output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            stdout.contains("Status: Enabled")
        }
        Err(_) => false,
    }
}

// ─── Transport-layer pf mode (all TCP ports) ──────────────────────────────

// ─── Windows WFP (Windows Filtering Platform) ─────────────────────────
//
// Windows transparent proxy uses the Windows Filtering Platform (WFP) API
// instead of macOS pfctl. Full implementation requires:
//
//   1. windows-sys or winapi crate with WFP bindings
//   2. Administrator privileges (run as admin)
//   3. WFP callout driver for TCP stream redirection
//
// API contract (implement when Windows target is available):
//   setup_pf(interface, local_ip) → redirects TCP 80/443 → local proxy port
//   teardown_pf() → removes WFP filters
//   is_pf_enabled() → checks WFP filter state
//
// Current stub returns manual proxy instructions.

#[cfg(windows)]
pub fn setup_pf(
    _interface: String,
    _local_ip: String,
    _config: &AppConfig,
) -> Result<String, String> {
    Err(
        "Windows transparent proxy via WFP is not yet implemented.\n\
         Use manual proxy mode: Settings → Network → Proxy → 127.0.0.1:8088\n\
         WFP API integration requires windows-sys or winapi crate."
            .into(),
    )
}

#[cfg(windows)]
pub fn teardown_pf(_config: &AppConfig) -> Result<(), String> {
    Ok(())
}
