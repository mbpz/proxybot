//! Minimal TUN forwarding probe (`proxybot tun-probe`).
//!
//! Creates a utun interface and prints IP packets that are routed into it by a
//! separate pf `route-to` rule. It deliberately does not forward or reply:
//! the only goal is to prove that forwarded iPhone packets can enter a TUN
//! interface (and later, that a return path can be built).

use anyhow::{bail, Context, Result};
use chrono::Local;
use std::io::Read;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::process::Command;
use std::time::{Duration, Instant};

#[derive(clap::Args)]
pub struct TunProbeArgs {
    /// How long to run; default 30 seconds.
    #[arg(long, default_value_t = 30)]
    pub duration: u64,
    /// Local TUN address (default 10.255.0.1).
    #[arg(long, default_value = "10.255.0.1")]
    pub address: String,
    /// TUN peer address (default 10.255.0.2).
    #[arg(long, default_value = "10.255.0.2")]
    pub peer: String,
    /// TUN MTU.
    #[arg(long, default_value_t = 1500)]
    pub mtu: i32,
}

pub fn run(args: TunProbeArgs) -> Result<()> {
    ensure_root()?;
    let address: Ipv4Addr = args.address.parse().context("parse --address")?;
    let peer: Ipv4Addr = args.peer.parse().context("parse --peer")?;

    let mut config = tun::Configuration::default();
    config
        .address(address)
        .destination(peer)
        .netmask((255, 255, 255, 252))
        .mtu(args.mtu)
        .up();
    let mut device = tun::create(&config).context("create utun (tun-probe requires root)")?;
    use tun::Device as _;
    let name = device.name().to_string();
    device
        .set_nonblock()
        .context("set utun non-blocking")?;

    println!("tun-probe: created {name}  local={address} peer={peer}");
    println!("next: sudo sh minimal/pf-tun-test.sh install {name}");
    println!("running for {}s (Ctrl+C to stop) — no return path is implemented", args.duration);
    println!();

    let started = Instant::now();
    let mut buffer = [0_u8; 65_536];
    let mut packets = 0_u64;
    let mut bytes = 0_u64;
    let mut last_header = Instant::now();

    while started.elapsed() < Duration::from_secs(args.duration) {
        match device.read(&mut buffer) {
            Ok(n) if n > 4 => {
                packets += 1;
                bytes += n as u64;
                if last_header.elapsed() >= Duration::from_millis(200) {
                    last_header = Instant::now();
                    print_packet(&buffer[4..n]);
                }
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(error) => return Err(error).context("read utun"),
        }
    }

    println!();
    println!("tun-probe stopped: {packets} packets, {bytes} bytes");
    if packets == 0 {
        println!("no packets entered the TUN; verify the pf route-to rule and the iPhone router setting");
    }
    Ok(())
}

fn ensure_root() -> Result<()> {
    let output = Command::new("id").arg("-u").output().context("run id -u")?;
    if String::from_utf8_lossy(&output.stdout).trim() != "0" {
        bail!("tun-probe requires root: sudo proxybot tun-probe ...");
    }
    Ok(())
}

fn print_packet(packet: &[u8]) {
    let Some(ip) = parse_ip(packet) else {
        return;
    };
    println!("[{}] {ip}", Local::now().format("%H:%M:%S"));
}

fn parse_ip(packet: &[u8]) -> Option<String> {
    let version = packet.first()? >> 4;
    match version {
        4 => {
            if packet.len() < 20 {
                return None;
            }
            let ihl = usize::from(packet[0] & 0x0f) * 4;
            let proto = packet[9];
            let src = IpAddr::V4(Ipv4Addr::new(packet[12], packet[13], packet[14], packet[15]));
            let dst = IpAddr::V4(Ipv4Addr::new(packet[16], packet[17], packet[18], packet[19]));
            Some(describe(proto, src, dst, &packet[ihl..]))
        }
        6 => {
            if packet.len() < 40 {
                return None;
            }
            let proto = packet[6];
            let mut src_bytes = [0_u8; 16];
            src_bytes.copy_from_slice(&packet[8..24]);
            let mut dst_bytes = [0_u8; 16];
            dst_bytes.copy_from_slice(&packet[24..40]);
            Some(describe(
                proto,
                IpAddr::V6(Ipv6Addr::from(src_bytes)),
                IpAddr::V6(Ipv6Addr::from(dst_bytes)),
                &packet[40..],
            ))
        }
        _ => None,
    }
}

fn describe(proto: u8, src: IpAddr, dst: IpAddr, payload: &[u8]) -> String {
    match proto {
        6 if payload.len() >= 4 => {
            let sport = u16::from_be_bytes([payload[0], payload[1]]);
            let dport = u16::from_be_bytes([payload[2], payload[3]]);
            format!("TCP {}:{} -> {}:{}", src, sport, dst, dport)
        }
        17 if payload.len() >= 4 => {
            let sport = u16::from_be_bytes([payload[0], payload[1]]);
            let dport = u16::from_be_bytes([payload[2], payload[3]]);
            format!("UDP {}:{} -> {}:{}", src, sport, dst, dport)
        }
        1 => format!("ICMP {src} -> {dst}"),
        _ => format!("proto {proto} {src} -> {dst}"),
    }
}
