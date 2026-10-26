//! One-command gateway lifecycle: generate config, start sing-box, install the
//! pf route-to rules, and restore everything on stop.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const ANCHOR: &str = "com.apple/proxybot-tun";

#[derive(Debug, Serialize, Deserialize)]
struct State {
    pid: u32,
    tun: String,
    iface: String,
    cidr: String,
    peer: String,
    local: String,
    previous_forwarding: String,
    config: String,
}

fn state_dir() -> PathBuf {
    std::env::temp_dir().join("proxybot-gateway")
}
fn state_path() -> PathBuf {
    state_dir().join("state.json")
}

fn is_root() -> bool {
    Command::new("id")
        .arg("-u")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim() == "0")
        .unwrap_or(false)
}

fn run(cmd: &str, args: &[&str]) -> Result<()> {
    let status = Command::new(cmd)
        .args(args)
        .status()
        .with_context(|| format!("run {cmd}"))?;
    if !status.success() {
        bail!("{cmd} {} failed with {status}", args.join(" "));
    }
    Ok(())
}

fn output(cmd: &str, args: &[&str]) -> Result<String> {
    let out = Command::new(cmd)
        .args(args)
        .output()
        .with_context(|| format!("run {cmd}"))?;
    if !out.status.success() {
        bail!("{cmd} {} failed", args.join(" "));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn proxyport_open() -> bool {
    TcpStream::connect_timeout(
        &"127.0.0.1:8088".parse().expect("addr"),
        Duration::from_millis(500),
    )
    .is_ok()
}

fn find_singbox(explicit: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = explicit {
        if path.is_file() {
            return Ok(path.to_path_buf());
        }
        bail!("sing-box binary not found: {}", path.display());
    }
    if let Ok(path) = std::env::var("PROXYBOT_SINGBOX_BIN") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Ok(path);
        }
    }
    for candidate in ["/tmp/singbox-src/sing-box", "/opt/homebrew/bin/sing-box", "/usr/local/bin/sing-box"] {
        let path = PathBuf::from(candidate);
        if path.is_file() {
            return Ok(path);
        }
    }
    if let Ok(path) = output("sh", &["-c", "command -v sing-box"]) {
        if !path.is_empty() {
            return Ok(PathBuf::from(path));
        }
    }
    bail!(
        "sing-box not found. Pass --singbox <path> or set PROXYBOT_SINGBOX_BIN;\n\
         the binary must be built with -tags with_gvisor"
    )
}

fn write_policy(block_quic: bool) {
    let Some(user) = std::env::var("SUDO_USER").ok() else {
        return;
    };
    let path = PathBuf::from(format!("/Users/{user}/Library/Application Support/ProxyBot/quic-policy"));
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(path, if block_quic { "block" } else { "direct" });
}

fn read_state() -> Option<State> {
    let data = fs::read_to_string(state_path()).ok()?;
    serde_json::from_str(&data).ok()
}

fn alive(pid: u32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn cleanup(state: &State) -> Result<()> {
    let _ = Command::new("kill").args(["-TERM", &state.pid.to_string()]).status();
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(5) && alive(state.pid) {
        thread::sleep(Duration::from_millis(100));
    }
    let _ = Command::new("kill").args(["-KILL", &state.pid.to_string()]).status();

    for anchor in [
        "com.apple/proxybot",
        "com.apple/proxybot-nat",
        "com.apple/proxybot-rdr",
        ANCHOR,
    ] {
        let _ = Command::new("pfctl").args(["-a", anchor, "-F", "all"]).status();
    }
    let _ = Command::new("sysctl")
        .args(["-w", &format!("net.inet.ip.forwarding={}", state.previous_forwarding)])
        .status();
    Ok(())
}

fn wait_tun(tun: &str) -> Result<()> {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(8) {
        if Command::new("ifconfig").arg(tun).output().map(|o| o.status.success()).unwrap_or(false) {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(100));
    }
    bail!("{tun} did not appear; check the sing-box log")
}

pub fn start(block_quic: bool, singbox: Option<PathBuf>, tun: String) -> Result<()> {
    if !is_root() {
        bail!("gateway start requires root: sudo proxybot gateway start");
    }
    if let Some(state) = read_state() {
        if alive(state.pid) {
            bail!("gateway already running (pid {})", state.pid);
        }
        let _ = fs::remove_file(state_path());
    }
    if !proxyport_open() {
        bail!("ProxyBot is not listening on 127.0.0.1:8088; start `proxybot start` first");
    }
    let bin = find_singbox(singbox.as_deref())?;
    write_policy(block_quic);
    let detected = crate::gateway::detect(8089)?;
    let dir = state_dir();
    fs::create_dir_all(&dir)?;
    let config_path = dir.join("singbox.json");
    let config = crate::singbox_config::build_config(
        "127.0.0.1:8088",
        &tun,
        "127.0.0.1:8089",
        block_quic,
    );
    fs::write(&config_path, serde_json::to_vec_pretty(&config)?)?;
    let log_path = dir.join("singbox.log");
    let log = File::create(&log_path)?;
    let child = Command::new(&bin)
        .args(["run", "-c"])
        .arg(&config_path)
        .stdout(Stdio::from(log.try_clone()?))
        .stderr(Stdio::from(log))
        .spawn()
        .with_context(|| format!("start {}", bin.display()))?;
    let pid = child.id();
    let previous_forwarding = output("sysctl", &["-n", "net.inet.ip.forwarding"])
        .unwrap_or_else(|_| "0".into());
    let state = State {
        pid,
        tun: tun.clone(),
        iface: detected.interface,
        cidr: detected.lan_cidr,
        peer: "10.255.0.2".into(),
        local: "10.255.0.1".into(),
        previous_forwarding,
        config: config_path.display().to_string(),
    };
    fs::write(state_path(), serde_json::to_vec_pretty(&state)?)?;

    if let Err(error) = wait_tun(&tun) {
        let _ = cleanup(&state);
        let _ = fs::remove_file(state_path());
        let tail = fs::read_to_string(&log_path).unwrap_or_default();
        bail!("{error}\n--- sing-box log ---\n{tail}");
    }
    run("ifconfig", &[&tun, "inet", &state.local, &state.peer, "up"])?;

    let pf_conf = dir.join("pf.conf");
    let rules = format!(
        "pass in quick on {iface} inet from {cidr} to 224.0.0.0/4 keep state\n\
         pass in quick on {iface} inet from {cidr} to 255.255.255.255/32 keep state\n\
         pass in quick on {iface} route-to ({tun} {peer}) inet from {cidr} to ! {cidr} keep state\n",
        iface = state.iface,
        cidr = state.cidr,
        tun = state.tun,
        peer = state.peer
    );
    fs::write(&pf_conf, rules)?;
    if let Err(error) = (|| -> Result<()> {
        run("pfctl", &["-n", "-f", pf_conf.to_str().unwrap_or_default()])?;
        for anchor in [
            "com.apple/proxybot",
            "com.apple/proxybot-nat",
            "com.apple/proxybot-rdr",
            ANCHOR,
        ] {
            let _ = Command::new("pfctl").args(["-a", anchor, "-F", "all"]).status();
        }
        run("pfctl", &["-a", ANCHOR, "-f", pf_conf.to_str().unwrap_or_default()])?;
        run("sysctl", &["-w", "net.inet.ip.forwarding=1"])?;
        Ok(())
    })() {
        let _ = cleanup(&state);
        let _ = fs::remove_file(state_path());
        return Err(error).context("install pf route-to; previous state restored");
    }

    // Detached watchdog: restore pf if sing-box exits unexpectedly.
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("proxybot"));
    let watchdog = format!(
        "while kill -0 {} 2>/dev/null; do sleep 2; done; '{}' gateway stop >/dev/null 2>&1 || true",
        state.pid,
        exe.display().to_string().replace('\'', "'\\''")
    );
    let _ = Command::new("sh").arg("-c").arg(watchdog).spawn();

    println!("gateway started (pid {})", state.pid);
    println!("  interface {}  LAN {}  TUN {} ({})", state.iface, state.cidr, state.tun, state.peer);
    println!("  QUIC/UDP 443: {}", if block_quic { "blocked" } else { "direct" });
    println!("  sing-box log: {}", log_path.display());
    println!("  stop: sudo proxybot gateway stop");
    Ok(())
}

pub fn stop() -> Result<()> {
    if !is_root() {
        bail!("gateway stop requires root: sudo proxybot gateway stop");
    }
    let Some(state) = read_state() else {
        println!("gateway is not running");
        return Ok(());
    };
    cleanup(&state)?;
    let _ = fs::remove_file(state_path());
    println!("gateway stopped; pf route-to removed and forwarding restored");
    Ok(())
}

pub fn status() -> Result<()> {
    match read_state() {
        Some(state) if alive(state.pid) => {
            println!("gateway: running (pid {})", state.pid);
            println!("  interface {}  LAN {}  TUN {}", state.iface, state.cidr, state.tun);
            println!("  config {}", state.config);
        }
        Some(state) => {
            println!("gateway: stale state (pid {} not running)", state.pid);
            println!("  run: sudo proxybot gateway stop");
        }
        None => println!("gateway: stopped"),
    }
    Ok(())
}
