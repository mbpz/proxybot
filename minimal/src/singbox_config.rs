//! Generate the sing-box TUN configuration used by the gateway beta.
//!
//! Keeping config generation in ProxyBot makes the QUIC policy explicit and
//! avoids maintaining divergent hand-written JSON files.

use anyhow::{Context, Result};
use serde_json::json;
use std::path::PathBuf;

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
    /// Transparent adapter address used for plain HTTP/80.
    #[arg(long, default_value = "127.0.0.1:8089")]
    pub transparent: String,
}

/// Build the gateway config. Kept here so the CLI and `gateway start` share one
/// source of truth.
pub fn build_config(http_proxy: &str, tun: &str, transparent: &str, block_quic: bool) -> serde_json::Value {
    let proxy_host = http_proxy.split(':').next().unwrap_or("127.0.0.1");
    let proxy_port = http_proxy
        .rsplit(':')
        .next()
        .and_then(|port| port.parse::<u16>().ok())
        .unwrap_or(8088);
    let transparent_host = transparent.split(':').next().unwrap_or("127.0.0.1");
    let transparent_port = transparent
        .rsplit(':')
        .next()
        .and_then(|port| port.parse::<u16>().ok())
        .unwrap_or(8089);
    let mut rules = vec![json!({"action": "sniff"})];
    rules.push(json!({
        "network": ["tcp"],
        "port": 80,
        "action": "route",
        "outbound": "direct",
        "override_address": transparent_host,
        "override_port": transparent_port
    }));
    rules.push(json!({"network": ["tcp"], "port": 443, "outbound": "mitm"}));
    if block_quic {
        rules.push(json!({"network": ["udp"], "port": 443, "action": "reject"}));
    }
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
        "outbounds": [
            {
                "type": "http",
                "tag": "mitm",
                "server": proxy_host,
                "server_port": proxy_port,
                "headers": {"X-ProxyBot-Source": "gateway"}
            },
            {"type": "direct", "tag": "direct"}
        ],
        "route": {"rules": rules, "final": "direct"}
    })
}

pub fn run(args: SingboxConfigArgs) -> Result<()> {
    let data_dir = crate::default_data_dir();
    std::fs::create_dir_all(&data_dir)?;
    let output = args
        .output
        .unwrap_or_else(|| data_dir.join("singbox-tun-mitm.json"));

    let config = build_config(&args.http_proxy, &args.tun, &args.transparent, args.block_quic);
    std::fs::write(&output, serde_json::to_vec_pretty(&config)?)
        .with_context(|| format!("write sing-box config {}", output.display()))?;

    let policy = if args.block_quic { "block" } else { "direct" };
    std::fs::write(data_dir.join("quic-policy"), policy)?;
    println!("wrote {}", output.display());
    println!("QUIC / UDP 443 policy: {policy}");
    println!();
    println!("Run the gateway with:");
    println!("  sudo sing-box run -c {}", output.display());
    println!("  sudo sh minimal/pf-tun-test.sh install {}", args.tun);
    Ok(())
}
