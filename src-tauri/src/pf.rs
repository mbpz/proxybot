//! pf (Packet Filter) management for macOS transparent proxy.
//!
//! This module handles setting up and tearing down pf rules to redirect
//! incoming TCP traffic on ports 80 and 443 to the local proxy listening on port 8088.
//!
//! Privilege escalation is performed using osascript to prompt the user
//! with a system authentication dialog, providing better UX than raw sudo.

use std::fs;
use std::process::Command;

use crate::config::{dns_port, pf_anchor_file, pf_anchor_name, proxy_port};

/// Set up pf rules for transparent proxying.
/// Redirects TCP traffic on ports 80 and 443 to the local proxy on port 8088.
/// Requires administrator privileges via osascript prompt.
pub fn setup_pf(interface: String, local_ip: String) -> Result<String, String> {
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
        port = proxy_port(),
        dns_port = dns_port(),
        ip = local_ip,
    );
    fs::write(tmp_file, &rules).map_err(|e| format!("Failed to write temp pf rules: {}", e))?;

    // Privileged shell: copy to /etc/pf.anchors, load rules, enable pf.
    let privileged_script = format!(
        r#"do shell script "mkdir -p /etc/pf.anchors && cp {tmp} {anchor_file} && sysctl -w net.inet.ip.forwarding=1 && pfctl -a {anchor} -f {anchor_file} && pfctl -e; echo done" with administrator privileges"#,
        tmp = tmp_file,
        anchor_file = pf_anchor_file().display(),
        anchor = pf_anchor_name(),
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
        interface,
        proxy_port()
    ))
}

/// Tear down pf rules and disable IP forwarding.
pub fn teardown_pf() -> Result<(), String> {
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
    let _ = fs::remove_file(pf_anchor_file());

    log::info!("pf rules removed successfully via osascript");
    Ok(())
}

/// Check if pf is currently enabled.
pub fn is_pf_enabled() -> bool {
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

/// Set up pf rules for transport-layer proxying.
///
/// Unlike [`setup_pf`] which only redirects :80/:443, this mode redirects
/// ALL TCP traffic to the transport proxy (default port 8089).
/// The transport proxy detects the protocol, extracts metadata, and either
/// passes through or forwards to the HTTP MITM proxy.
///
/// DNS (UDP 53) is also redirected as in standard mode.
pub fn setup_pf_transport(interface: String, local_ip: String, transport_port: u16) -> Result<String, String> {
    if !interface.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err("Invalid interface name".to_string());
    }
    if interface.is_empty() || interface.len() > 10 {
        return Err("Invalid interface name".to_string());
    }
    if !local_ip.chars().all(|c| c.is_ascii_digit() || c == '.') || local_ip.is_empty() {
        return Err("Invalid local IP address".to_string());
    }

    let tmp_file = "/tmp/proxybot.pf.transport.conf";
    let rules = format!(
        "rdr on {iface} proto tcp from any to any port 1:65535 -> {ip} port {tport}\nrdr on {iface} proto udp from any to any port 53 -> {ip} port {dns_port}\npass on {iface} proto tcp from any to any port 1:65535\n",
        iface = interface,
        tport = transport_port,
        dns_port = dns_port(),
        ip = local_ip,
    );
    fs::write(tmp_file, &rules).map_err(|e| format!("Failed to write temp pf rules: {}", e))?;

    let privileged_script = format!(
        r#"do shell script "mkdir -p /etc/pf.anchors && cp {tmp} {anchor_file} && sysctl -w net.inet.ip.forwarding=1 && pfctl -a {anchor} -f {anchor_file} && pfctl -e; echo done" with administrator privileges"#,
        tmp = tmp_file,
        anchor_file = pf_anchor_file().display(),
        anchor = pf_anchor_name(),
    );

    let output = Command::new("osascript")
        .args(["-e", &privileged_script])
        .output()
        .map_err(|e| format!("Failed to execute osascript: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        return Err(format!("pf transport setup failed: {}{}", stderr, stdout));
    }

    log::info!("pf transport rules loaded: all TCP → port {}", transport_port);
    Ok(format!(
        "Transport proxy enabled. Redirecting {} all TCP traffic to port {}",
        interface, transport_port
    ))
}

/// Tear down transport-mode pf rules.
pub fn teardown_pf_transport() -> Result<(), String> {
    teardown_pf()
}

/// Check if transport-mode pf rules are loaded.
pub fn is_pf_transport_enabled() -> bool {
    is_pf_enabled()
}
