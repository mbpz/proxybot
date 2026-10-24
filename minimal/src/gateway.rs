//! macOS transparent gateway integration: pf redirect + NAT + IP forwarding.
//!
//! This is intentionally a thin wrapper around `pfctl`/`sysctl`. It requires
//! administrator privileges and is only installed when the user starts the CLI
//! with `--gateway` or runs `proxybot gateway install`.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::process::Command;

pub const ANCHOR: &str = "com.apple/proxybot";
pub const ANCHOR_FILE: &str = "/etc/pf.anchors/com.proxybot";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    pub interface: String,
    pub lan_ip: String,
    pub lan_cidr: String,
    pub transparent_port: u16,
    /// Drop QUIC/UDP 443 so apps that would otherwise bypass the TCP proxy fall
    /// back to TCP/TLS. This is opt-in because it changes app behaviour.
    #[serde(default)]
    pub block_quic: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct InstallState {
    previous_forwarding: String,
    installed_at: i64,
}

pub fn detect(transparent_port: u16) -> Result<GatewayConfig> {
    let interface = default_interface()?;
    let lan_ip = interface_ip(&interface)?;
    let lan_cidr = interface_cidr(&interface, &lan_ip)?;
    Ok(GatewayConfig {
        interface,
        lan_ip,
        lan_cidr,
        transparent_port,
        block_quic: false,
    })
}

/// pf rules loaded into a dedicated anchor. The anchor name is under
/// `com.apple/*` so the stock macOS pf.conf evaluates its rdr/nat rules.
pub fn render_rules(config: &GatewayConfig) -> String {
    let iface = &config.interface;
    let ip = &config.lan_ip;
    let cidr = &config.lan_cidr;
    let port = config.transparent_port;
    // pf requires anchors to be ordered: translation rules (rdr/nat) first,
    // then filtering (block/pass). A mixed order is rejected with
    // "Rules must be in order: options, normalization, queueing, translation,
    // filtering".
    let quic_block = if config.block_quic {
        format!(
            "block drop quick on {iface} inet proto udp from {cidr} to any port {{80, 443}}\n"
        )
    } else {
        String::new()
    };
    format!(
        "rdr pass on {iface} inet proto tcp from any to any port {{80, 443}} -> {ip} port {port}\n\
         nat on {iface} inet from {cidr} to any -> ({iface})\n\
         {quic_block}\
         pass in quick on {iface} inet from {cidr} to any keep state\n\
         pass out quick on {iface} inet from any to any keep state\n"
    )
}

pub fn install(config: &GatewayConfig, data_dir: &Path, dry_run: bool) -> Result<()> {
    let rules = render_rules(config);
    if dry_run {
        println!("# {ANCHOR_FILE}\n{rules}");
        println!(
            "# would run: sysctl -w net.inet.ip.forwarding=1 && pfctl -a {ANCHOR} -f {ANCHOR_FILE} && pfctl -e"
        );
        return Ok(());
    }

    let previous = sysctl_value("net.inet.ip.forwarding").unwrap_or_else(|_| "0".into());
    let previous_for_rollback = previous.clone();
    let state = InstallState {
        previous_forwarding: previous,
        installed_at: now_ms(),
    };
    fs::create_dir_all(data_dir)?;
    fs::write(
        data_dir.join("gateway.json"),
        serde_json::to_vec_pretty(&state)?,
    )?;

    let tmp = std::env::temp_dir().join("proxybot-pf.conf");
    fs::write(&tmp, rules)?;
    let script = format!(
        "mkdir -p /etc/pf.anchors && cp '{}' {ANCHOR_FILE} && sysctl -w net.inet.ip.forwarding=1 && pfctl -a {ANCHOR} -f {ANCHOR_FILE} && pfctl -e",
        tmp.display()
    );
    if let Err(error) = run_privileged(&script) {
        // Roll back partial state: forwarding may already be enabled even when
        // pf rejected the rules.
        let rollback = format!(
            "pfctl -a {ANCHOR} -F all 2>/dev/null || true; rm -f {ANCHOR_FILE}; sysctl -w net.inet.ip.forwarding={previous_for_rollback} >/dev/null; true"
        );
        let _ = run_privileged(&rollback);
        let _ = fs::remove_file(data_dir.join("gateway.json"));
        return Err(error).context("install pf gateway rules (admin required); previous state restored");
    }
    println!(
        "gateway installed on {} ({}) with transparent port {}",
        config.interface, config.lan_ip, config.transparent_port
    );
    Ok(())
}

pub fn uninstall(data_dir: &Path, dry_run: bool) -> Result<()> {
    let state_path = data_dir.join("gateway.json");
    let previous = fs::read_to_string(&state_path)
        .ok()
        .and_then(|text| serde_json::from_str::<InstallState>(&text).ok())
        .map(|state| state.previous_forwarding)
        .unwrap_or_else(|| "0".into());

    if dry_run {
        println!(
            "# would run: pfctl -a {ANCHOR} -F all; rm -f {ANCHOR_FILE}; sysctl -w net.inet.ip.forwarding={previous}"
        );
        return Ok(());
    }

    let script = format!(
        "pfctl -a {ANCHOR} -F all 2>/dev/null || true; rm -f {ANCHOR_FILE}; sysctl -w net.inet.ip.forwarding={previous} >/dev/null; true"
    );
    run_privileged(&script).context("uninstall pf gateway rules (admin required)")?;
    let _ = fs::remove_file(&state_path);
    println!("gateway removed (ip.forwarding restored to {previous})");
    Ok(())
}

pub fn status() -> Result<String> {
    let forwarding = sysctl_value("net.inet.ip.forwarding").unwrap_or_else(|_| "unknown".into());
    let rules = Command::new("pfctl")
        .args(["-a", ANCHOR, "-s", "rules"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).into_owned())
        .filter(|text| !text.trim().is_empty());
    let mut lines = vec![format!("ip.forwarding = {forwarding}")];
    match rules {
        Some(text) => {
            lines.push(format!("anchor {ANCHOR} rules:"));
            lines.push(text);
        }
        None => lines.push(format!(
            "anchor {ANCHOR}: not loaded (or need sudo to read pf rules)"
        )),
    }
    Ok(lines.join("\n"))
}

fn default_interface() -> Result<String> {
    let output = Command::new("route")
        .args(["-n", "get", "default"])
        .output()
        .context("run `route -n get default`")?;
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines()
        .find_map(|line| line.trim().strip_prefix("interface:").map(str::trim))
        .map(str::to_string)
        .context("could not determine the default network interface")
}

fn interface_ip(interface: &str) -> Result<String> {
    let output = Command::new("ipconfig")
        .args(["getifaddr", interface])
        .output()
        .context("run `ipconfig getifaddr`")?;
    let ip = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if ip.is_empty() {
        bail!("interface {interface} has no IPv4 address");
    }
    Ok(ip)
}

fn interface_cidr(interface: &str, ip: &str) -> Result<String> {
    let output = Command::new("ifconfig")
        .arg(interface)
        .output()
        .context("run `ifconfig`")?;
    let text = String::from_utf8_lossy(&output.stdout);
    let netmask = text
        .lines()
        .find_map(|line| line.trim().strip_prefix("inet ").map(str::to_string))
        .and_then(|line| {
            line.split_whitespace()
                .position(|part| part == "netmask")
                .and_then(|index| line.split_whitespace().nth(index + 1))
                .map(str::to_string)
        });
    let prefix = netmask
        .as_deref()
        .and_then(prefix_from_netmask)
        .unwrap_or(24);
    let ip: std::net::Ipv4Addr = ip.parse().context("parse interface IPv4 address")?;
    let mask = if prefix == 0 {
        0
    } else {
        u32::MAX << (32 - prefix)
    };
    let network = std::net::Ipv4Addr::from(u32::from(ip) & mask);
    Ok(format!("{network}/{prefix}"))
}

fn prefix_from_netmask(netmask: &str) -> Option<u32> {
    if let Some(hex) = netmask.strip_prefix("0x") {
        let value = u32::from_str_radix(hex, 16).ok()?;
        return Some(value.count_ones());
    }
    netmask
        .parse::<std::net::Ipv4Addr>()
        .ok()
        .map(|mask| u32::from(mask).count_ones())
}

fn sysctl_value(name: &str) -> Result<String> {
    let output = Command::new("sysctl")
        .args(["-n", name])
        .output()
        .context("run `sysctl`")?;
    if !output.status.success() {
        bail!("sysctl {name} failed");
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn run_privileged(script: &str) -> Result<()> {
    let status = if libc_geteuid() == 0 {
        Command::new("sh").arg("-c").arg(script).status()
    } else {
        Command::new("sudo").arg("sh").arg("-c").arg(script).status()
    }
    .context("spawn privileged command")?;
    if !status.success() {
        bail!("privileged command failed (exit {status})");
    }
    Ok(())
}

#[cfg(unix)]
fn libc_geteuid() -> u32 {
    // Avoid pulling libc in for a single call: read the effective UID through
    // the `id` command is slower, but `unsafe { libc::geteuid() }` would add a
    // dependency. Keep the shell-level fallback instead.
    Command::new("id")
        .arg("-u")
        .output()
        .ok()
        .and_then(|output| String::from_utf8_lossy(&output.stdout).trim().parse().ok())
        .unwrap_or(1)
}

#[cfg(not(unix))]
fn libc_geteuid() -> u32 {
    1
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}
