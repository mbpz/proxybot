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

pub fn run(args: SingboxConfigArgs) -> Result<()> {
    let data_dir = crate::default_data_dir();
    std::fs::create_dir_all(&data_dir)?;
    let output = args
        .output
        .unwrap_or_else(|| data_dir.join("singbox-tun-mitm.json"));

    let transparent_host = args.transparent.split(':').next().unwrap_or("127.0.0.1");
    let transparent_port = args
        .transparent
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
    if args.block_quic {
        rules.push(json!({"network": ["udp"], "port": 443, "action": "reject"}));
    }

    let config = json!({
        "log": {"level": "debug"},
        "inbounds": [{
            "type": "tun",
            "tag": "tun-in",
            "interface_name": args.tun,
            "auto_route": false,
            "strict_route": false,
            "stack": "gvisor"
        }],
        "outbounds": [
            {
                "type": "http",
                "tag": "mitm",
                "server": args.http_proxy.split(':').next().unwrap_or("127.0.0.1"),
                "server_port": args.http_proxy.rsplit(':').next().and_then(|p| p.parse::<u16>().ok()).unwrap_or(8088),
                "headers": {"X-ProxyBot-Source": "gateway"}
            },
            {"type": "direct", "tag": "direct"}
        ],
        "route": {"rules": rules, "final": "direct"}
    });
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
