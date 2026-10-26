//! launchd integration for the Gateway Beta.
//!
//! The Supervisor remains the sole owner of sing-box and network recovery.
//! launchd only manages whether the foreground service process exists; it does
//! not autostart and does not auto-restart.

use anyhow::{bail, Context, Result};
use clap::Args;
use std::fs;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const LABEL: &str = "com.proxybot.gateway";
const PLIST_PATH: &str = "/Library/LaunchDaemons/com.proxybot.gateway.plist";
const LOG_DIR: &str = "/Library/Logs/ProxyBot";
const LOG_FILE: &str = "/Library/Logs/ProxyBot/gateway-service.log";

#[derive(Args, Clone)]
pub struct ServiceArgs {
    #[arg(long)]
    pub block_quic: bool,
    #[arg(long)]
    pub singbox: PathBuf,
    #[arg(long, default_value = "utun6")]
    pub tun: String,
    #[arg(long, conflicts_with = "devices")]
    pub client_ip: Option<String>,
    #[arg(long)]
    pub devices: Option<PathBuf>,
}

#[derive(Args)]
pub struct InstallServiceArgs {
    #[arg(long)]
    pub block_quic: bool,
    #[arg(long)]
    pub singbox: Option<PathBuf>,
    #[arg(long, default_value = "utun6")]
    pub tun: String,
    #[arg(long, conflicts_with = "devices")]
    pub client_ip: Option<String>,
    #[arg(long)]
    pub devices: Option<PathBuf>,
}

fn root() -> bool {
    Command::new("id")
        .arg("-u")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim() == "0")
        .unwrap_or(false)
}

fn proxy_open() -> bool {
    TcpStream::connect_timeout(&"127.0.0.1:8088".parse().expect("addr"), Duration::from_millis(500))
        .is_ok()
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn service_args(args: &ServiceArgs) -> Vec<String> {
    let mut values = vec![
        "gateway".into(),
        "service".into(),
        "--singbox".into(),
        args.singbox.display().to_string(),
        "--tun".into(),
        args.tun.clone(),
    ];
    if args.block_quic {
        values.push("--block-quic".into());
    }
    if let Some(client_ip) = &args.client_ip {
        values.push("--client-ip".into());
        values.push(client_ip.clone());
    }
    if let Some(devices) = &args.devices {
        values.push("--devices".into());
        values.push(devices.display().to_string());
    }
    values
}

fn plist(program: &Path, args: &ServiceArgs) -> String {
    let mut arg_xml = String::new();
    for value in service_args(args) {
        arg_xml.push_str(&format!("        <string>{}</string>\n", xml_escape(&value)));
    }
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key><string>{label}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{program}</string>
{args}
    </array>
    <key>RunAtLoad</key><false/>
    <key>KeepAlive</key><false/>
    <key>ProcessType</key><string>Interactive</string>
    <key>StandardOutPath</key><string>{log}</string>
    <key>StandardErrorPath</key><string>{log}</string>
</dict>
</plist>
"#,
        label = LABEL,
        program = xml_escape(&program.display().to_string()),
        args = arg_xml,
        log = LOG_FILE
    )
}

pub fn service_installed() -> bool {
    Path::new(PLIST_PATH).is_file()
}

pub fn install(args: InstallServiceArgs) -> Result<()> {
    if !root() {
        bail!("gateway install-service requires root: sudo proxybot gateway install-service");
    }
    let singbox = match args.singbox {
        Some(path) => path,
        None => std::env::var("PROXYBOT_SINGBOX_BIN")
            .map(PathBuf::from)
            .or_else(|_| {
                ["/tmp/singbox-src/sing-box", "/opt/homebrew/bin/sing-box", "/usr/local/bin/sing-box"]
                    .iter()
                    .map(PathBuf::from)
                    .find(|path| path.is_file())
                    .ok_or_else(|| anyhow::anyhow!("sing-box not found"))
            })?,
    };
    if !singbox.is_file() {
        bail!("sing-box binary not found: {}", singbox.display());
    }
    let program = std::env::current_exe().context("resolve proxybot executable")?;
    let service = ServiceArgs {
        block_quic: args.block_quic,
        singbox,
        tun: args.tun,
        client_ip: args.client_ip,
        devices: args.devices,
    };
    let _ = crate::gateway_run::stop();
    fs::create_dir_all(LOG_DIR)?;
    let path = Path::new(PLIST_PATH);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, plist(&program, &service))?;
    let _ = Command::new("launchctl").args(["bootout", &format!("system/{LABEL}")]).status();
    let status = Command::new("launchctl")
        .args(["bootstrap", "system", PLIST_PATH])
        .status()
        .context("launchctl bootstrap")?;
    if !status.success() {
        bail!("launchctl bootstrap failed");
    }
    let _ = Command::new("launchctl").args(["enable", &format!("system/{LABEL}")]).status();
    println!("installed LaunchDaemon {LABEL}");
    println!("  plist {PLIST_PATH}");
    println!("  log   {LOG_FILE}");
    println!("  start with: sudo proxybot gateway start (explicit; does not start at boot)");
    Ok(())
}

pub fn uninstall() -> Result<()> {
    if !root() {
        bail!("gateway uninstall-service requires root: sudo proxybot gateway uninstall-service");
    }
    // Always try to stop/clean the current instance first.
    let _ = crate::gateway_run::stop();
    let _ = Command::new("launchctl").args(["bootout", &format!("system/{LABEL}")]).status();
    let _ = fs::remove_file(PLIST_PATH);
    println!("uninstalled LaunchDaemon {LABEL}; gateway network state was stopped");
    Ok(())
}

pub fn start_service() -> Result<()> {
    if !root() {
        bail!("gateway start requires root: sudo proxybot gateway start");
    }
    if !service_installed() {
        bail!("gateway service is not installed; run sudo proxybot gateway install-service");
    }
    if !proxy_open() {
        bail!("ProxyBot is not listening on 127.0.0.1:8088; start `proxybot start` first");
    }
    // Refuse to kick a second instance over a live supervisor.
    if let Some(state) = read_state() {
        if let Some(pid) = state.get("supervisor_pid").and_then(|v| v.as_u64()) {
            if Command::new("ps").args(["-p", &pid.to_string(), "-o", "pid="]).output().map(|o| o.status.success() && !o.stdout.is_empty()).unwrap_or(false)
                && state.get("state").and_then(|v| v.as_str()) == Some("running")
            {
                bail!("gateway already running");
            }
        }
        // A stale state must be cleaned before launchd starts a new instance.
        let _ = crate::gateway_run::stop();
    }
    let status = Command::new("launchctl")
        .args(["kickstart", &format!("system/{LABEL}")])
        .status()
        .context("launchctl kickstart")?;
    if !status.success() {
        bail!("launchctl kickstart failed");
    }
    // launchd accepted the request; wait for the supervisor's state file.
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    while std::time::Instant::now() < deadline {
        if let Some(state) = read_state() {
            match state.get("state").and_then(|v| v.as_str()) {
                Some("running") => {
                    println!("gateway started via launchd ({LABEL})");
                    println!("  instance {}", state.get("instance_id").and_then(|v| v.as_str()).unwrap_or("?"));
                    return Ok(());
                }
                Some("exited") | Some("recovery_failed") => break,
                _ => {}
            }
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    let log = fs::read_to_string(LOG_FILE).unwrap_or_default();
    bail!("gateway service did not reach running;\n--- {LOG_FILE} ---\n{log}");
}

fn read_state() -> Option<serde_json::Value> {
    serde_json::from_str(&fs::read_to_string("/tmp/proxybot-gateway/state.json").ok()?).ok()
}

pub fn run_service(args: ServiceArgs) -> Result<()> {
    if !root() {
        bail!("gateway service requires root");
    }
    if !proxy_open() {
        bail!("ProxyBot is not listening on 127.0.0.1:8088");
    }
    let instance = format!(
        "{}-{}",
        SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0),
        std::process::id()
    );
    crate::gateway_run::supervise(crate::gateway_run::SuperviseArgs {
        instance,
        block_quic: args.block_quic,
        singbox: args.singbox,
        tun: args.tun,
        client_ip: args.client_ip,
        devices: args.devices,
    })
}
