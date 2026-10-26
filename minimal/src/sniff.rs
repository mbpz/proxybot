//! Passive packet capture mode (`proxybot sniff`).
//!
//! This observes traffic on a capture interface (normally the default-route
//! Wi-Fi interface) and prints flow/DNS/TLS-SNI metadata. It never modifies or
//! proxies traffic, so it is useful when a device's packets reach the Mac but
//! transparent interception is unavailable. Requires root for BPF access.

use anyhow::{bail, Context, Result};
use chrono::Local;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

#[derive(clap::Args)]
pub struct SniffArgs {
    /// Capture interface (default: interface of the default route).
    #[arg(long)]
    pub interface: Option<String>,
    /// Only capture traffic for this device IP (e.g. 192.168.3.20).
    #[arg(long)]
    pub device: Option<String>,
    /// Stop after N seconds (default: run until Ctrl+C).
    #[arg(long)]
    pub duration: Option<u64>,
    /// Append JSON-lines events to this file.
    #[arg(long)]
    pub log: Option<PathBuf>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Proto {
    Tcp,
    Udp,
}

impl Proto {
    fn as_str(self) -> &'static str {
        match self {
            Self::Tcp => "TCP",
            Self::Udp => "UDP",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct FlowKey {
    proto: Proto,
    src: SocketAddr,
    dst: SocketAddr,
}

struct FlowState {
    packets: u64,
    bytes: u64,
    sni: Option<String>,
}

pub fn run(args: SniffArgs) -> Result<()> {
    ensure_root()?;
    let interface = match args.interface {
        Some(value) => value,
        None => default_interface()?,
    };
    let device_ip: Option<IpAddr> = match args.device.as_deref() {
        Some(value) => Some(
            value
                .parse()
                .with_context(|| format!("parse --device IP {value}"))?,
        ),
        None => None,
    };

    let filter = match device_ip {
        Some(ip) => format!("host {ip}"),
        None => "tcp or udp".to_string(),
    };

    let mut cap = pcap::Capture::from_device(interface.as_str())
        .with_context(|| format!("open capture device {interface}"))?
        .promisc(true)
        .snaplen(65_535)
        .timeout(500)
        .immediate_mode(true)
        .open()
        .context("open BPF capture (sniff requires sudo)")?;
    cap.filter(&filter, true)
        .with_context(|| format!("install BPF filter: {filter}"))?;

    let mut log = match args.log.as_ref() {
        Some(path) => {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            Some(
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)
                    .with_context(|| format!("open sniff log {}", path.display()))?,
            )
        }
        None => None,
    };

    println!(
        "sniffing {interface} ({filter}) — Ctrl+C to stop{}",
        args.duration
            .map(|seconds| format!(", auto-stop after {seconds}s"))
            .unwrap_or_default()
    );
    if let Some(ip) = device_ip {
        println!("device filter: {ip}");
    }

    let started = Instant::now();
    let mut flows: HashMap<FlowKey, FlowState> = HashMap::new();
    let mut total_packets = 0_u64;
    let mut total_bytes = 0_u64;
    let mut events = 0_u64;

    loop {
        if let Some(limit) = args.duration {
            if started.elapsed() >= Duration::from_secs(limit) {
                break;
            }
        }
        match cap.next_packet() {
            Ok(packet) => {
                total_packets += 1;
                total_bytes += packet.data.len() as u64;
                process_packet(
                    packet.data,
                    &mut flows,
                    &mut log,
                    &mut events,
                );
            }
            Err(pcap::Error::TimeoutExpired) => {}
            Err(error) => return Err(error).context("capture next packet"),
        }
    }

    println!();
    println!(
        "sniff stopped: {} flows, {} events, {} packets, {} bytes",
        flows.len(),
        events,
        total_packets,
        total_bytes
    );
    let mut by_dest: HashMap<SocketAddr, (u64, u64)> = HashMap::new();
    for (key, flow) in &flows {
        let entry = by_dest.entry(key.dst).or_insert((0, 0));
        entry.0 += flow.packets;
        entry.1 += flow.bytes;
    }
    let mut dests: Vec<_> = by_dest.into_iter().collect();
    dests.sort_by_key(|(_, (_, bytes))| std::cmp::Reverse(*bytes));
    for (dst, (packets, bytes)) in dests.into_iter().take(10) {
        println!("  {dst}  {packets} pkts  {bytes} bytes");
    }
    Ok(())
}

fn ensure_root() -> Result<()> {
    let output = Command::new("id")
        .arg("-u")
        .output()
        .context("run id -u")?;
    let uid = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if uid != "0" {
        bail!("sniff requires BPF access; run it with sudo: sudo proxybot sniff ...");
    }
    Ok(())
}

fn default_interface() -> Result<String> {
    let output = Command::new("route")
        .args(["-n", "get", "default"])
        .output()
        .context("detect default route")?;
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines()
        .find_map(|line| line.trim().strip_prefix("interface:").map(str::trim))
        .map(str::to_string)
        .filter(|value| !value.is_empty())
        .context("default route interface not found; pass --interface")
}

fn process_packet(
    data: &[u8],
    flows: &mut HashMap<FlowKey, FlowState>,
    log: &mut Option<std::fs::File>,
    events: &mut u64,
) {
    let Some((linktype, offset)) = link_offset(data) else {
        return;
    };
    let ethertype = linktype;
    let ip = &data[offset..];
    match ethertype {
        0x0800 => process_ipv4(ip, flows, log, events),
        0x86dd => process_ipv6(ip, flows, log, events),
        _ => {}
    }
}

fn link_offset(data: &[u8]) -> Option<(u16, usize)> {
    // We cannot see the capture's linktype here; support Ethernet and raw IP.
    if data.len() >= 14 {
        let ethertype = u16::from_be_bytes([data[12], data[13]]);
        if ethertype == 0x0800 || ethertype == 0x86dd {
            return Some((ethertype, 14));
        }
        if ethertype == 0x8100 && data.len() >= 18 {
            let inner = u16::from_be_bytes([data[16], data[17]]);
            if inner == 0x0800 || inner == 0x86dd {
                return Some((inner, 18));
            }
        }
    }
    if data.first().is_some_and(|version| version >> 4 == 4) {
        return Some((0x0800, 0));
    }
    if data.first().is_some_and(|version| version >> 4 == 6) {
        return Some((0x86dd, 0));
    }
    None
}

fn process_ipv4(
    data: &[u8],
    flows: &mut HashMap<FlowKey, FlowState>,
    log: &mut Option<std::fs::File>,
    events: &mut u64,
) {
    if data.len() < 20 {
        return;
    }
    let ihl = usize::from(data[0] & 0x0f) * 4;
    if ihl < 20 || data.len() < ihl {
        return;
    }
    let total = u16::from_be_bytes([data[2], data[3]]) as usize;
    let end = total.min(data.len());
    let proto = data[9];
    let src = IpAddr::V4(Ipv4Addr::new(data[12], data[13], data[14], data[15]));
    let dst = IpAddr::V4(Ipv4Addr::new(data[16], data[17], data[18], data[19]));
    let payload = &data[ihl..end];
    process_transport(proto, src, dst, payload, flows, log, events);
}

fn process_ipv6(
    data: &[u8],
    flows: &mut HashMap<FlowKey, FlowState>,
    log: &mut Option<std::fs::File>,
    events: &mut u64,
) {
    if data.len() < 40 {
        return;
    }
    let next = data[6];
    let mut src_bytes = [0_u8; 16];
    src_bytes.copy_from_slice(&data[8..24]);
    let mut dst_bytes = [0_u8; 16];
    dst_bytes.copy_from_slice(&data[24..40]);
    let src = IpAddr::V6(Ipv6Addr::from(src_bytes));
    let dst = IpAddr::V6(Ipv6Addr::from(dst_bytes));
    let payload = &data[40..];
    // Extension headers are not expected for the flows we report; skip non-TCP/UDP.
    process_transport(next, src, dst, payload, flows, log, events);
}

fn process_transport(
    proto: u8,
    src: IpAddr,
    dst: IpAddr,
    payload: &[u8],
    flows: &mut HashMap<FlowKey, FlowState>,
    log: &mut Option<std::fs::File>,
    events: &mut u64,
) {
    match proto {
        6 => {
            if payload.len() < 20 {
                return;
            }
            let sport = u16::from_be_bytes([payload[0], payload[1]]);
            let dport = u16::from_be_bytes([payload[2], payload[3]]);
            let data_offset = usize::from(payload[12] >> 4) * 4;
            if data_offset < 20 || payload.len() < data_offset {
                return;
            }
            let flags = payload[13];
            let body = &payload[data_offset..];
            let key = FlowKey {
                proto: Proto::Tcp,
                src: SocketAddr::new(src, sport),
                dst: SocketAddr::new(dst, dport),
            };
            let is_new = !flows.contains_key(&key);
            let state = flows.entry(key).or_insert_with(|| FlowState {
                packets: 0,
                bytes: 0,
                sni: None,
            });
            state.packets += 1;
            state.bytes += body.len() as u64;

            if is_new && flags & 0x02 != 0 {
                emit(
                    log,
                    events,
                    "tcp",
                    format!(
                        "{}  {}:{} -> {}:{}  SYN",
                        Proto::Tcp.as_str(),
                        src,
                        sport,
                        dst,
                        dport
                    ),
                    serde_json::json!({
                        "kind": "tcp",
                        "src": src.to_string(),
                        "sport": sport,
                        "dst": dst.to_string(),
                        "dport": dport,
                    }),
                );
            }

            if dport == 443 && state.sni.is_none() && !body.is_empty() {
                if let Some(sni) = crate::transparent::parse_sni(body) {
                    state.sni = Some(sni.clone());
                    emit(
                        log,
                        events,
                        "tls",
                        format!("TLS   SNI {sni}  ({} -> {}:{})", src, dst, dport),
                        serde_json::json!({
                            "kind": "tls",
                            "sni": sni,
                            "src": src.to_string(),
                            "dst": dst.to_string(),
                            "dport": dport,
                        }),
                    );
                }
            }
        }
        17 => {
            if payload.len() < 8 {
                return;
            }
            let sport = u16::from_be_bytes([payload[0], payload[1]]);
            let dport = u16::from_be_bytes([payload[2], payload[3]]);
            let body = &payload[8..];
            let key = FlowKey {
                proto: Proto::Udp,
                src: SocketAddr::new(src, sport),
                dst: SocketAddr::new(dst, dport),
            };
            let is_new = !flows.contains_key(&key);
            let state = flows.entry(key).or_insert_with(|| FlowState {
                packets: 0,
                bytes: 0,
                sni: None,
            });
            state.packets += 1;
            state.bytes += body.len() as u64;

            if dport == 53 {
                if let Some(name) = dns_question(body) {
                    emit(
                        log,
                        events,
                        "dns",
                        format!("DNS   query {name}  ({} -> {})", src, dst),
                        serde_json::json!({
                            "kind": "dns",
                            "query": name,
                            "src": src.to_string(),
                            "dst": dst.to_string(),
                        }),
                    );
                }
            } else if is_new && sport != 5353 && dport != 5353 {
                emit(
                    log,
                    events,
                    "udp",
                    format!(
                        "{}  {}:{} -> {}:{}",
                        Proto::Udp.as_str(),
                        src,
                        sport,
                        dst,
                        dport
                    ),
                    serde_json::json!({
                        "kind": "udp",
                        "src": src.to_string(),
                        "sport": sport,
                        "dst": dst.to_string(),
                        "dport": dport,
                    }),
                );
            }
        }
        _ => {}
    }
}

fn emit(
    log: &mut Option<std::fs::File>,
    events: &mut u64,
    kind: &str,
    line: String,
    json: serde_json::Value,
) {
    *events += 1;
    println!("[{}] {line}", Local::now().format("%H:%M:%S"));
    if let Some(file) = log.as_mut() {
        let mut record = json;
        record["ts"] = serde_json::json!(chrono::Utc::now().to_rfc3339());
        record["event"] = serde_json::json!(kind);
        if writeln!(file, "{record}").is_ok() {
            let _ = file.flush();
        }
    }
}

fn dns_question(payload: &[u8]) -> Option<String> {
    if payload.len() < 12 {
        return None;
    }
    let qdcount = u16::from_be_bytes([payload[4], payload[5]]);
    if qdcount == 0 {
        return None;
    }
    let mut offset = 12;
    let mut labels = Vec::new();
    while offset < payload.len() {
        let length = payload[offset] as usize;
        offset += 1;
        if length == 0 {
            break;
        }
        if length & 0xc0 != 0 || offset + length > payload.len() {
            return None;
        }
        labels.push(
            String::from_utf8_lossy(&payload[offset..offset + length]).into_owned(),
        );
        offset += length;
        if labels.len() > 32 {
            return None;
        }
    }
    if labels.is_empty() {
        None
    } else {
        Some(labels.join("."))
    }
}
