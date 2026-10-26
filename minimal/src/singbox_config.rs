//! Generate the sing-box TUN configuration used by the gateway beta.
//!
//! Configuration generation is intentionally centralized here so the CLI and
//! `gateway start` share one source of truth. Device entries are validated
//! before any JSON is produced: duplicate/overlapping source CIDRs would make
//! identity routing ambiguous.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::net::Ipv4Addr;
use std::path::{Path, PathBuf};

#[derive(clap::Args)]
pub struct SingboxConfigArgs {
    /// Reject UDP/443 at the sing-box route layer (QUIC fallback test).
    #[arg(long)]
    pub block_quic: bool,
    /// Output path (default: <data>/singbox-tun-mitm.json).
    #[arg(long)]
    pub output: Option<PathBuf>,
    /// Local HTTP proxy (Hudsucker) address.
    #[arg(long, default_value = "127.0.0.1:8088")]
    pub http_proxy: String,
    /// TUN interface name.
    #[arg(long, default_value = "utun6")]
    pub tun: String,
    /// Single-device prototype: attach this source IP to the outbound metadata.
    #[arg(long, conflicts_with = "devices")]
    pub client_ip: Option<String>,
    /// Device identity file for source-IP-routed per-device outbounds.
    #[arg(long)]
    pub devices: Option<PathBuf>,
    /// Transparent adapter address used for plain HTTP/80.
    #[arg(long, default_value = "127.0.0.1:8089")]
    pub transparent: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub device_key: String,
    pub source_ip_cidr: String,
    pub client_ip: String,
}

#[derive(Debug, Deserialize)]
pub struct DeviceFile {
    pub devices: Vec<Device>,
}

#[derive(Debug, Clone)]
struct Cidr {
    start: u32,
    end: u32,
}

fn parse_cidr(value: &str) -> Result<Cidr> {
    let (address, prefix) = value
        .split_once('/')
        .with_context(|| format!("device CIDR must be IPv4/prefix: {value}"))?;
    if address.contains(':') {
        bail!("device CIDR currently supports IPv4 only: {value}");
    }
    let address: Ipv4Addr = address
        .parse()
        .with_context(|| format!("invalid device IPv4 address: {value}"))?;
    let prefix: u8 = prefix
        .parse()
        .with_context(|| format!("invalid device CIDR prefix: {value}"))?;
    if prefix > 32 {
        bail!("device CIDR prefix out of range: {value}");
    }
    let raw = u32::from(address);
    let mask = if prefix == 0 { 0 } else { u32::MAX << (32 - prefix) };
    let start = raw & mask;
    let end = start | !mask;
    Ok(Cidr { start, end })
}

fn validate_devices(devices: &[Device]) -> Result<Vec<Cidr>> {
    if devices.is_empty() {
        bail!("devices file contains no devices");
    }
    let mut keys = std::collections::HashSet::new();
    let mut cidrs: Vec<Cidr> = Vec::new();
    let mut client_ips = std::collections::HashSet::new();
    for device in devices {
        if device.device_key.is_empty()
            || !device
                .device_key
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            bail!("invalid device_key {:?}; use letters, digits, '-' or '_'", device.device_key);
        }
        if !keys.insert(device.device_key.clone()) {
            bail!("duplicate device_key {:?}", device.device_key);
        }
        let cidr = parse_cidr(&device.source_ip_cidr)?;
        if cidrs.iter().any(|existing| cidr.start <= existing.end && existing.start <= cidr.end) {
            bail!("overlapping source_ip_cidr for device {:?}", device.device_key);
        }
        let client: Ipv4Addr = device
            .client_ip
            .parse()
            .with_context(|| format!("invalid client_ip for {:?}", device.device_key))?;
        let client_raw = u32::from(client);
        if client_raw < cidr.start || client_raw > cidr.end {
            bail!(
                "client_ip {} is outside source_ip_cidr {} for device {:?}",
                device.client_ip,
                device.source_ip_cidr,
                device.device_key
            );
        }
        if !client_ips.insert(device.client_ip.clone()) {
            bail!("duplicate client_ip {}", device.client_ip);
        }
        cidrs.push(cidr);
    }
    Ok(cidrs)
}

pub fn load_devices(path: &Path) -> Result<Vec<Device>> {
    let data = std::fs::read_to_string(path)
        .with_context(|| format!("read device file {}", path.display()))?;
    let file: DeviceFile = serde_json::from_str(&data)
        .with_context(|| format!("parse device file {}", path.display()))?;
    validate_devices(&file.devices)?;
    Ok(file.devices)
}

/// Build a single-device (or no-identity) config. `client_ip` attaches static
/// metadata to every gateway CONNECT; this is a prototype convenience.
pub fn build_config(
    http_proxy: &str,
    tun: &str,
    transparent: &str,
    block_quic: bool,
    client_ip: Option<&str>,
) -> serde_json::Value {
    let mut headers = json!({"X-ProxyBot-Source": "gateway"});
    if let Some(client_ip) = client_ip {
        headers["X-ProxyBot-Client-IP"] = json!(client_ip);
        headers["X-ProxyBot-Identity-Source"] = json!("tun_source_ip");
        headers["X-ProxyBot-Confidence"] = json!("inferred");
    }
    let outbounds = vec![
        json!({
            "type": "http", "tag": "mitm",
            "server": proxy_host(http_proxy), "server_port": proxy_port(http_proxy),
            "headers": headers
        }),
        json!({"type": "direct", "tag": "direct"}),
    ];
    let mut rules = vec![json!({"action": "sniff"})];
    push_common_rules(&mut rules, http_proxy, transparent, block_quic, None);
    build(tun, outbounds, rules)
}

/// P4.3-A: one outbound per registered device, selected by TUN source CIDR.
/// Unregistered sources fall through to the generic `mitm` outbound, which has
/// no client-ip metadata and therefore records an unknown identity.
pub fn build_config_devices(
    http_proxy: &str,
    tun: &str,
    transparent: &str,
    block_quic: bool,
    devices: &[Device],
) -> Result<serde_json::Value> {
    validate_devices(devices)?;
    let mut outbounds = vec![json!({
        "type": "http", "tag": "mitm",
        "server": proxy_host(http_proxy), "server_port": proxy_port(http_proxy),
        "headers": {"X-ProxyBot-Source": "gateway"}
    })];
    for device in devices {
        outbounds.push(json!({
            "type": "http",
            "tag": format!("proxybot-device-{}", device.device_key),
            "server": proxy_host(http_proxy),
            "server_port": proxy_port(http_proxy),
            "headers": {
                "X-ProxyBot-Source": "gateway",
                "X-ProxyBot-Client-IP": device.client_ip,
                "X-ProxyBot-Identity-Source": "tun_source_ip",
                "X-ProxyBot-Confidence": "inferred"
            }
        }));
    }
    outbounds.push(json!({"type": "direct", "tag": "direct"}));

    let mut rules = vec![json!({"action": "sniff"})];
    for device in devices {
        rules.push(json!({
            "network": ["tcp"],
            "port": 443,
            "source_ip_cidr": [device.source_ip_cidr],
            "outbound": format!("proxybot-device-{}", device.device_key)
        }));
    }
    push_common_rules(&mut rules, http_proxy, transparent, block_quic, None);
    Ok(build(tun, outbounds, rules))
}

fn proxy_host(addr: &str) -> &str {
    addr.split(':').next().unwrap_or("127.0.0.1")
}
fn proxy_port(addr: &str) -> u16 {
    addr.rsplit(':').next().and_then(|p| p.parse().ok()).unwrap_or(8088)
}
fn transparent_host(addr: &str) -> &str {
    addr.split(':').next().unwrap_or("127.0.0.1")
}
fn transparent_port(addr: &str) -> u16 {
    addr.rsplit(':').next().and_then(|p| p.parse().ok()).unwrap_or(8089)
}

fn push_common_rules(
    rules: &mut Vec<serde_json::Value>,
    _http_proxy: &str,
    transparent: &str,
    block_quic: bool,
    _: Option<()>,
) {
    rules.push(json!({
        "network": ["tcp"],
        "port": 80,
        "action": "route",
        "outbound": "direct",
        "override_address": transparent_host(transparent),
        "override_port": transparent_port(transparent)
    }));
    rules.push(json!({"network": ["tcp"], "port": 443, "outbound": "mitm"}));
    if block_quic {
        rules.push(json!({"network": ["udp"], "port": 443, "action": "reject"}));
    }
}

fn build(tun: &str, outbounds: Vec<serde_json::Value>, rules: Vec<serde_json::Value>) -> serde_json::Value {
    json!({
        "log": {"level": "debug"},
        "inbounds": [{
            "type": "tun",
            "tag": "tun-in",
            "interface_name": tun,
            "auto_route": false,
            "strict_route": false,
            "stack": "gvisor"
        }],
        "outbounds": outbounds,
        "route": {"rules": rules, "final": "direct"}
    })
}

pub fn run(args: SingboxConfigArgs) -> Result<()> {
    let data_dir = crate::default_data_dir();
    std::fs::create_dir_all(&data_dir)?;
    let output = args.output.unwrap_or_else(|| data_dir.join("singbox-tun-mitm.json"));

    let config = if let Some(path) = args.devices.as_deref() {
        let devices = load_devices(path)?;
        build_config_devices(
            &args.http_proxy,
            &args.tun,
            &args.transparent,
            args.block_quic,
            &devices,
        )?
    } else {
        build_config(
            &args.http_proxy,
            &args.tun,
            &args.transparent,
            args.block_quic,
            args.client_ip.as_deref(),
        )
    };
    std::fs::write(&output, serde_json::to_vec_pretty(&config)?)
        .with_context(|| format!("write sing-box config {}", output.display()))?;

    let policy = if args.block_quic { "block" } else { "direct" };
    std::fs::write(data_dir.join("quic-policy"), policy)?;
    println!("wrote {}", output.display());
    println!("QUIC / UDP 443 policy: {policy}");
    println!();
    println!("Run the gateway with:");
    println!("  sudo sing-box run -c {}", output.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn device(key: &str, cidr: &str, ip: &str) -> Device {
        Device { device_key: key.into(), source_ip_cidr: cidr.into(), client_ip: ip.into() }
    }

    #[test]
    fn rejects_overlapping_cidrs_and_duplicate_keys() {
        assert!(validate_devices(&[
            device("a", "192.168.3.190/32", "192.168.3.190"),
            device("b", "192.168.3.190/32", "192.168.3.191"),
        ])
        .is_err());
        assert!(validate_devices(&[
            device("a", "192.168.3.0/24", "192.168.3.190"),
            device("a", "192.168.3.128/25", "192.168.3.191"),
        ])
        .is_err());
        assert!(validate_devices(&[
            device("a", "192.168.3.190/32", "192.168.3.190"),
            device("b", "192.168.3.191/32", "192.168.3.192"),
        ])
        .is_err());
    }

    #[test]
    fn device_rules_precede_generic_mitm() {
        let config = build_config_devices(
            "127.0.0.1:8088",
            "utun6",
            "127.0.0.1:8089",
            false,
            &[
                device("a", "192.168.3.190/32", "192.168.3.190"),
                device("b", "192.168.3.191/32", "192.168.3.191"),
            ],
        )
        .unwrap();
        let rules = config["route"]["rules"].as_array().unwrap();
        assert_eq!(rules[1]["source_ip_cidr"][0], "192.168.3.190/32");
        assert_eq!(rules[2]["source_ip_cidr"][0], "192.168.3.191/32");
        assert_eq!(rules[3]["port"], 80);
        assert_eq!(rules[4]["port"], 443);
        let outbounds = config["outbounds"].as_array().unwrap();
        assert!(outbounds.iter().any(|o| o["tag"] == "proxybot-device-a"));
    }
}
