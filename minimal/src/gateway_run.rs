//! Gateway supervisor and lifecycle.
//!
//! `gateway start` launches a detached supervisor that owns sing-box, applies
//! pf route-to rules, records exit status, and restores network state when the
//! child exits. `gateway stop` signals the supervisor; repeated stops are safe.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::os::unix::io::AsRawFd;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const ANCHOR: &str = "com.apple/proxybot-tun";
static STOP: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct State {
    instance_id: String,
    supervisor_pid: u32,
    singbox_pid: Option<u32>,
    started_at: i64,
    last_heartbeat_at: i64,
    state: String,
    exit_code: Option<i32>,
    exit_signal: Option<i32>,
    exit_reason: Option<String>,
    recovery_status: String,
    tun: String,
    iface: String,
    #[serde(default)]
    lan_ip: String,
    cidr: String,
    peer: String,
    local: String,
    previous_forwarding: String,
    config: String,
    log_dir: String,
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn state_dir() -> PathBuf {
    PathBuf::from("/tmp/proxybot-gateway")
}
fn state_path() -> PathBuf {
    state_dir().join("state.json")
}
fn instance_dir(id: &str) -> PathBuf {
    state_dir().join("instances").join(id)
}

fn atomic_write(path: &Path, data: &[u8]) -> Result<()> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, data).with_context(|| format!("write {}", tmp.display()))?;
    fs::rename(&tmp, path).with_context(|| format!("rename {}", path.display()))?;
    Ok(())
}

fn write_state(state: &State) -> Result<()> {
    atomic_write(&state_path(), &serde_json::to_vec_pretty(state)?)
}

fn read_state() -> Option<State> {
    serde_json::from_str(&fs::read_to_string(state_path()).ok()?).ok()
}

const LOG_LIMIT: u64 = 2 * 1024 * 1024;

fn rotate_if_needed(path: &Path) {
    let size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    if size <= LOG_LIMIT {
        return;
    }
    let _ = fs::remove_file(path.with_extension("log.2"));
    let _ = fs::rename(path.with_extension("log.1"), path.with_extension("log.2"));
    let _ = fs::rename(path, path.with_extension("log.1"));
}

fn append_log(path: &Path, bytes: &[u8]) {
    rotate_if_needed(path);
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = file.write_all(bytes);
    }
}

fn log_line(dir: &Path, name: &str, message: &str) {
    let line = format!(
        "{} {}\n",
        chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        message
    );
    append_log(&dir.join(name), line.as_bytes());
}

fn lock_path() -> PathBuf {
    state_dir().join("lock")
}

fn acquire_lock(path: &Path, message: &str) -> Result<File> {
    fs::create_dir_all(state_dir())?;
    let file = OpenOptions::new().create(true).write(true).open(path)?;
    let rc = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if rc != 0 {
        bail!("{message}");
    }
    Ok(file)
}

fn acquire_supervisor_lock() -> Result<File> {
    acquire_lock(&lock_path(), "another gateway supervisor holds the instance lock")
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

fn alive(pid: u32) -> bool {
    Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "pid="])
        .output()
        .map(|out| out.status.success() && !String::from_utf8_lossy(&out.stdout).trim().is_empty())
        .unwrap_or(false)
}

fn process_command(pid: u32) -> String {
    output("ps", &["-p", &pid.to_string(), "-o", "command="]).unwrap_or_default()
}

fn terminate_state_child(state: &State) {
    let Some(pid) = state.singbox_pid else { return; };
    if !alive(pid) {
        return;
    }
    // Only kill a process that still carries this instance's config path; never
    // kill a recycled PID.
    if !process_command(pid).contains(&state.config) {
        return;
    }
    let _ = Command::new("kill").args(["-TERM", &pid.to_string()]).status();
    let start = std::time::Instant::now();
    while start.elapsed() < Duration::from_secs(5) && alive(pid) {
        thread::sleep(Duration::from_millis(100));
    }
    if alive(pid) {
        let _ = Command::new("kill").args(["-KILL", &pid.to_string()]).status();
    }
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
    let out = Command::new(cmd).args(args).output().with_context(|| format!("run {cmd}"))?;
    if !out.status.success() {
        bail!("{cmd} {} failed", args.join(" "));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn proxyport_open() -> bool {
    TcpStream::connect_timeout(&"127.0.0.1:8088".parse().expect("addr"), Duration::from_millis(500)).is_ok()
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
    bail!("sing-box not found. Pass --singbox <path> or set PROXYBOT_SINGBOX_BIN")
}

fn wait_tun(tun: &str) -> Result<()> {
    let start = std::time::Instant::now();
    while start.elapsed() < Duration::from_secs(8) {
        if Command::new("ifconfig").arg(tun).output().map(|o| o.status.success()).unwrap_or(false) {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(100));
    }
    bail!("{tun} did not appear")
}

fn install_pf(state: &State, dir: &Path) -> Result<()> {
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
    run("pfctl", &["-n", "-f", pf_conf.to_str().unwrap_or_default()])?;
    for anchor in ["com.apple/proxybot", "com.apple/proxybot-nat", "com.apple/proxybot-rdr", ANCHOR] {
        let _ = Command::new("pfctl").args(["-a", anchor, "-F", "all"]).status();
    }
    run("pfctl", &["-a", ANCHOR, "-f", pf_conf.to_str().unwrap_or_default()])?;
    run("sysctl", &["-w", "net.inet.ip.forwarding=1"])?;
    Ok(())
}

/// Restore only what this instance changed. Idempotent.
fn restore_network(state: &State, dir: &Path) -> Result<()> {
    let mut failures = Vec::new();
    for anchor in ["com.apple/proxybot", "com.apple/proxybot-nat", "com.apple/proxybot-rdr", ANCHOR] {
        let status = Command::new("pfctl").args(["-a", anchor, "-F", "all"]).status();
        if !status.map(|s| s.success()).unwrap_or(false) {
            failures.push(format!("flush anchor {anchor}"));
        }
    }
    // Only restore forwarding if it is still the value we enabled; do not
    // overwrite a later user/network change.
    let current = output("sysctl", &["-n", "net.inet.ip.forwarding"]).unwrap_or_default();
    if current.trim() == "1" {
        if let Err(error) = run("sysctl", &["-w", &format!("net.inet.ip.forwarding={}", state.previous_forwarding)]) {
            failures.push(format!("restore forwarding: {error}"));
        }
    }
    if std::env::var("PROXYBOT_RECOVERY_FAIL").ok().as_deref() == Some("1") {
        failures.push("injected recovery failure (PROXYBOT_RECOVERY_FAIL=1)".into());
    }
    log_line(dir, "recovery.log", &format!("restore instance={} failures={:?}", state.instance_id, failures));
    if failures.is_empty() {
        Ok(())
    } else {
        bail!("{} recovery step(s) failed: {:?}", failures.len(), failures)
    }
}

fn terminate_child(child: &mut Child, dir: &Path) {
    let pid = child.id().to_string();
    let _ = Command::new("kill").args(["-TERM", &pid]).status();
    let start = std::time::Instant::now();
    while start.elapsed() < Duration::from_secs(5) {
        if child.try_wait().ok().flatten().is_some() {
            return;
        }
        thread::sleep(Duration::from_millis(100));
    }
    let _ = child.kill();
    let _ = child.wait();
    log_line(dir, "supervisor.log", "child exceeded graceful-stop timeout; killed");
}

fn exit_status_fields(status: &std::process::ExitStatus) -> (Option<i32>, Option<i32>, &'static str) {
    if let Some(code) = status.code() {
        // sing-box handles SIGTERM and can exit 0; from the supervisor's point
        // of view any child exit while running is still unexpected.
        (Some(code), None, "unexpected_child_exit")
    } else if let Some(signal) = status.signal() {
        (None, Some(signal), "signal_terminated")
    } else {
        (None, None, "unexpected_child_exit")
    }
}

fn signal_handler(_: libc::c_int) {
    STOP.store(true, Ordering::SeqCst);
}

fn install_signal_handlers() {
    unsafe {
        libc::signal(libc::SIGTERM, signal_handler as *const () as libc::sighandler_t);
        libc::signal(libc::SIGINT, signal_handler as *const () as libc::sighandler_t);
    }
}

pub fn start(
    block_quic: bool,
    singbox: Option<PathBuf>,
    tun: String,
    client_ip: Option<String>,
    devices: Option<PathBuf>,
) -> Result<()> {
    if !is_root() {
        bail!("gateway start requires root: sudo proxybot gateway start");
    }
    let _start_lock = acquire_lock(
        &state_dir().join("start.lock"),
        "another gateway start is already in progress",
    )?;
    if crate::launchd::service_installed() {
        eprintln!("launchd service is installed; delegating to its configured arguments");
        return crate::launchd::start_service();
    }
    if let Some(old) = read_state() {
        if alive(old.supervisor_pid) {
            bail!("gateway already running (instance {}, supervisor pid {})", old.instance_id, old.supervisor_pid);
        }
        terminate_state_child(&old);
        let _ = restore_network(&old, Path::new(&old.log_dir));
        let _ = fs::remove_file(state_path());
    }
    if !proxyport_open() {
        bail!("ProxyBot is not listening on 127.0.0.1:8088; start `proxybot start` first");
    }
    let bin = find_singbox(singbox.as_deref())?;
    write_policy(block_quic);
    let instance = format!("{}-{}", now_ms(), std::process::id());
    let dir = instance_dir(&instance);
    fs::create_dir_all(&dir)?;
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("proxybot"));
    let supervisor_log = File::create(dir.join("supervisor.log"))?;
    let mut cmd = Command::new("/usr/bin/nohup");
    cmd.arg(exe)
        .args(["gateway", "supervise", "--instance", &instance, "--singbox"])
        .arg(&bin)
        .args(["--tun", &tun]);
    if let Some(client_ip) = client_ip.as_deref() {
        cmd.args(["--client-ip", client_ip]);
    }
    if let Some(devices) = devices.as_deref() {
        cmd.arg("--devices").arg(devices);
    }
    if block_quic {
        cmd.arg("--block-quic");
    }
    cmd.stdin(Stdio::null())
        .stdout(Stdio::from(supervisor_log.try_clone()?))
        .stderr(Stdio::from(supervisor_log));
    cmd.spawn().context("start gateway supervisor")?;

    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    let mut last_state = String::from("starting");
    while std::time::Instant::now() < deadline {
        if let Some(state) = read_state() {
            last_state = state.state.clone();
            match state.state.as_str() {
                "running" => {
                    println!("gateway started");
                    println!("  instance {}  supervisor {}  sing-box {}", state.instance_id, state.supervisor_pid, state.singbox_pid.unwrap_or(0));
                    println!("  interface {}  LAN {}  TUN {}", state.iface, state.cidr, state.tun);
                    println!("  QUIC/UDP 443: {}", if block_quic { "blocked" } else { "direct" });
                    println!("  logs {}", dir.display());
                    return Ok(());
                }
                "exited" | "recovery_failed" => break,
                _ => {}
            }
        }
        thread::sleep(Duration::from_millis(200));
    }
    // Do not leave a half-started supervisor behind when start times out or
    // observes a terminal state.
    if let Some(current) = read_state() {
        if alive(current.supervisor_pid) {
            let _ = Command::new("kill")
                .args(["-TERM", &current.supervisor_pid.to_string()])
                .status();
            let wait = std::time::Instant::now();
            while wait.elapsed() < Duration::from_secs(5) && alive(current.supervisor_pid) {
                thread::sleep(Duration::from_millis(100));
            }
            let _ = Command::new("kill")
                .args(["-KILL", &current.supervisor_pid.to_string()])
                .status();
        }
    }
    let tail = fs::read_to_string(dir.join("supervisor.log")).unwrap_or_default();
    let child_tail = fs::read_to_string(dir.join("sing-box.log")).unwrap_or_default();
    bail!(
        "gateway failed to become healthy (last state: {last_state})\n--- supervisor ---\n{tail}\n--- sing-box ---\n{child_tail}"
    );
}

fn write_policy(block_quic: bool) {
    let Some(user) = std::env::var("SUDO_USER").ok() else { return; };
    let path = PathBuf::from(format!("/Users/{user}/Library/Application Support/ProxyBot/quic-policy"));
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(path, if block_quic { "block" } else { "direct" });
}

#[derive(clap::Args)]
pub struct SuperviseArgs {
    #[arg(long)]
    pub instance: String,
    #[arg(long)]
    pub block_quic: bool,
    #[arg(long)]
    pub singbox: PathBuf,
    #[arg(long, default_value = "utun6")]
    pub tun: String,
    #[arg(long)]
    pub client_ip: Option<String>,
    #[arg(long)]
    pub devices: Option<PathBuf>,
}

pub fn supervise(args: SuperviseArgs) -> Result<()> {
    if !is_root() {
        bail!("gateway supervise requires root");
    }
    let dir = instance_dir(&args.instance);
    fs::create_dir_all(&dir)?;
    let _instance_lock = acquire_supervisor_lock()?;
    install_signal_handlers();
    let detected = crate::gateway::detect(8089)?;
    let config_path = dir.join("singbox.json");
    let config = if let Some(path) = args.devices.as_deref() {
        let devices = crate::singbox_config::load_devices(path)?;
        crate::singbox_config::build_config_devices(
            "127.0.0.1:8088",
            &args.tun,
            "127.0.0.1:8089",
            args.block_quic,
            &devices,
        )?
    } else {
        crate::singbox_config::build_config(
            "127.0.0.1:8088",
            &args.tun,
            "127.0.0.1:8089",
            args.block_quic,
            args.client_ip.as_deref(),
        )
    };
    fs::write(&config_path, serde_json::to_vec_pretty(&config)?)?;
    let previous_forwarding = output("sysctl", &["-n", "net.inet.ip.forwarding"]).unwrap_or_else(|_| "0".into());
    let mut state = State {
        instance_id: args.instance.clone(),
        supervisor_pid: std::process::id(),
        singbox_pid: None,
        started_at: now_ms(),
        last_heartbeat_at: now_ms(),
        state: "starting".into(),
        exit_code: None,
        exit_signal: None,
        exit_reason: None,
        recovery_status: "not_needed".into(),
        tun: args.tun.clone(),
        iface: detected.interface,
        lan_ip: detected.lan_ip,
        cidr: detected.lan_cidr,
        peer: "10.255.0.2".into(),
        local: "10.255.0.1".into(),
        previous_forwarding,
        config: config_path.display().to_string(),
        log_dir: dir.display().to_string(),
    };
    write_state(&state)?;
    log_line(&dir, "supervisor.log", &format!("supervisor start instance={}", args.instance));

    let check = Command::new(&args.singbox)
        .args(["check", "-c", config_path.to_str().unwrap_or_default()])
        .status();
    if !check.map(|status| status.success()).unwrap_or(false) {
        state.state = "exited".into();
        state.exit_reason = Some("startup_failed".into());
        state.recovery_status = "success".into();
        write_state(&state)?;
        log_line(&dir, "supervisor.log", "sing-box config check failed");
        bail!("sing-box config check failed");
    }

    let mut child = match Command::new(&args.singbox)
        .args(["run", "-c"])
        .arg(&config_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            state.state = "exited".into();
            state.exit_reason = Some("startup_failed".into());
            state.recovery_status = "success".into();
            write_state(&state)?;
            log_line(&dir, "supervisor.log", &format!("spawn failed: {error}"));
            bail!("spawn sing-box: {error}");
        }
    };
    state.singbox_pid = Some(child.id());
    write_state(&state)?;
    let log_path = Arc::new(dir.join("sing-box.log"));
    let log_lock = Arc::new(Mutex::new(()));
    for stream in [
        child.stdout.take().map(|s| Box::new(s) as Box<dyn Read + Send>),
        child.stderr.take().map(|s| Box::new(s) as Box<dyn Read + Send>),
    ]
    .into_iter()
    .flatten()
    {
        let path = log_path.clone();
        let lock = log_lock.clone();
        std::thread::spawn(move || {
            let mut stream = stream;
            let mut buffer = [0u8; 8192];
            loop {
                match stream.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        let _guard = lock.lock().ok();
                        append_log(&path, &buffer[..n]);
                    }
                }
            }
        });
    }

    if let Err(error) = wait_tun(&args.tun) {
        let _ = child.kill();
        let _ = child.wait();
        state.state = "exited".into();
        state.exit_reason = Some("healthcheck_failed".into());
        state.exit_code = child.try_wait().ok().flatten().and_then(|s| s.code());
        state.recovery_status = "success".into();
        write_state(&state)?;
        log_line(&dir, "supervisor.log", &format!("TUN health check failed: {error}"));
        return Ok(());
    }
    if let Err(error) = run("ifconfig", &[&state.tun, "inet", &state.local, &state.peer, "up"]) {
        let _ = child.kill();
        let _ = child.wait();
        state.state = "exited".into();
        state.exit_reason = Some("startup_failed".into());
        state.recovery_status = "success".into();
        write_state(&state)?;
        log_line(&dir, "supervisor.log", &format!("ifconfig failed: {error}"));
        return Ok(());
    }
    if let Err(error) = install_pf(&state, &dir) {
        let _ = child.kill();
        let _ = child.wait();
        let recovery = restore_network(&state, &dir);
        state.state = "exited".into();
        state.exit_reason = Some("startup_failed".into());
        state.recovery_status = if recovery.is_ok() { "success" } else { "failed" }.into();
        write_state(&state)?;
        log_line(&dir, "supervisor.log", &format!("pf install failed: {error}"));
        return Ok(());
    }

    state.state = "running".into();
    state.last_heartbeat_at = now_ms();
    write_state(&state)?;
    log_line(&dir, "supervisor.log", "gateway running");

    let mut last_heartbeat = std::time::Instant::now();
    loop {
        if STOP.load(Ordering::SeqCst) {
            state.state = "stopping".into();
            write_state(&state)?;
            log_line(&dir, "supervisor.log", "stop requested");
            terminate_child(&mut child, &dir);
            state.exit_reason = Some("supervisor_stop_requested".into());
            break;
        }
        if let Ok(Some(status)) = child.try_wait() {
            let (code, signal, reason) = exit_status_fields(&status);
            state.exit_code = code;
            state.exit_signal = signal;
            state.exit_reason = Some(reason.into());
            state.state = "recovering".into();
            write_state(&state)?;
            log_line(&dir, "supervisor.log", &format!("child exited code={code:?} signal={signal:?} reason={reason}"));
            break;
        }
        if last_heartbeat.elapsed() >= Duration::from_secs(2) {
            state.last_heartbeat_at = now_ms();
            if let Err(error) = write_state(&state) {
                log_line(&dir, "supervisor.log", &format!("heartbeat write failed: {error}"));
            }
            last_heartbeat = std::time::Instant::now();
        }
        thread::sleep(Duration::from_millis(200));
    }

    log_line(&dir, "recovery.log", "network recovery start");
    let recovery = restore_network(&state, &dir);
    state.recovery_status = if recovery.is_ok() { "success" } else { "failed" }.into();
    state.state = if !recovery.is_ok() {
        "recovery_failed"
    } else if state.exit_reason.as_deref() == Some("supervisor_stop_requested") {
        "stopped"
    } else {
        "exited"
    }
    .into();
    write_state(&state)?;
    log_line(&dir, "supervisor.log", &format!("supervisor exit state={} recovery={}", state.state, state.recovery_status));
    Ok(())
}

pub fn stop() -> Result<()> {
    if !is_root() {
        bail!("gateway stop requires root: sudo proxybot gateway stop");
    }
    let Some(mut state) = read_state() else {
        println!("gateway is not running");
        return Ok(());
    };
    if !alive(state.supervisor_pid) {
        if state.state == "stopped" && state.recovery_status == "success" {
            println!("gateway is already stopped");
            return Ok(());
        }
        // Stale state: recover what we can. Kill only a child that still carries
        // this instance's config path, never an unrelated recycled PID.
        terminate_state_child(&state);
        state.state = "recovering".into();
        let _ = write_state(&state);
        let recovery = restore_network(&state, Path::new(&state.log_dir));
        state.recovery_status = if recovery.is_ok() { "success" } else { "failed" }.into();
        state.state = if recovery.is_ok() {
            "stopped"
        } else {
            "recovery_failed"
        }
        .into();
        let _ = write_state(&state);
        println!(
            "gateway stopped (stale supervisor state; recovery: {})",
            state.recovery_status
        );
        return Ok(());
    }
    state.state = "stopping".into();
    let _ = write_state(&state);
    let _ = Command::new("kill").args(["-TERM", &state.supervisor_pid.to_string()]).status();
    let deadline = std::time::Instant::now() + Duration::from_secs(15);
    while std::time::Instant::now() < deadline {
        if let Some(current) = read_state() {
            if matches!(current.state.as_str(), "exited" | "stopped" | "recovery_failed") {
                println!("gateway stopped (recovery: {})", current.recovery_status);
                return Ok(());
            }
        }
        if !alive(state.supervisor_pid) {
            break;
        }
        thread::sleep(Duration::from_millis(200));
    }
    // Supervisor ignored TERM or died without cleaning up.
    let _ = Command::new("kill").args(["-KILL", &state.supervisor_pid.to_string()]).status();
    let recovery = restore_network(&state, Path::new(&state.log_dir));
    state.recovery_status = if recovery.is_ok() { "success" } else { "failed" }.into();
    state.state = if recovery.is_ok() {
        "stopped"
    } else {
        "recovery_failed"
    }
    .into();
    let _ = write_state(&state);
    println!("gateway stopped (forced; recovery: {})", state.recovery_status);
    Ok(())
}

fn tun_inet(tun: &str) -> Option<String> {
    let text = output("ifconfig", &[tun]).ok()?;
    text.lines()
        .find_map(|line| line.trim().strip_prefix("inet ").map(str::to_string))
        .map(|line| line.split_whitespace().next().unwrap_or_default().to_string())
}

pub fn status() -> Result<()> {
    // CLI output may be piped to `head`/`grep -q`; use normal SIGPIPE behavior
    // instead of Rust's panic-on-EPIPE (release builds abort on panic).
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
    let Some(state) = read_state() else {
        println!("gateway: stopped");
        return Ok(());
    };
    let supervisor_alive = alive(state.supervisor_pid);
    let heartbeat_age = now_ms().saturating_sub(state.last_heartbeat_at) / 1000;
    let child_alive = state.singbox_pid.map(alive).unwrap_or(false);
    let tun_present = Command::new("ifconfig")
        .arg(&state.tun)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    let tun_addr = tun_inet(&state.tun);
    let proxy_open = proxyport_open();
    let current_network = crate::gateway::detect(8089).ok();
    let network_changed = match current_network.as_ref() {
        Some(net) => {
            net.interface != state.iface
                || net.lan_cidr != state.cidr
                || (!state.lan_ip.is_empty() && net.lan_ip != state.lan_ip)
        }
        None => true,
    };
    let unhealthy =
        !supervisor_alive || !child_alive || !tun_present || heartbeat_age > 10 || network_changed;
    let display = if state.state == "running" && unhealthy {
        "stale"
    } else if state.state == "running" {
        "running"
    } else {
        state.state.as_str()
    };
    println!("gateway: {display}");
    println!("  instance {}  supervisor {}  sing-box {:?}", state.instance_id, state.supervisor_pid, state.singbox_pid);
    println!("  heartbeat {}s ago  recovery {}", heartbeat_age, state.recovery_status);
    println!(
        "  health tun={} ({}) child={} proxy={}",
        if tun_present { "up" } else { "absent" },
        tun_addr.as_deref().unwrap_or("no-ipv4"),
        if child_alive { "alive" } else { "missing" },
        if proxy_open { "ok" } else { "down" }
    );
    match current_network {
        Some(net) => {
            println!(
                "  network {} {} {} (state expected {} {} {})",
                net.interface, net.lan_ip, net.lan_cidr, state.iface, state.lan_ip, state.cidr
            );
            if net.interface != state.iface {
                println!("  stale state: network interface changed");
            } else if net.lan_cidr != state.cidr {
                println!("  stale state: LAN CIDR changed");
            } else if !state.lan_ip.is_empty() && net.lan_ip != state.lan_ip {
                println!("  stale state: LAN IP changed");
            }
        }
        None => println!("  network unknown (Wi-Fi down or default route missing)"),
    }
    if let Some(code) = state.exit_code {
        println!("  exit code {code}");
    }
    if let Some(signal) = state.exit_signal {
        println!("  exit signal {signal}");
    }
    if let Some(reason) = state.exit_reason {
        println!("  exit reason {reason}");
    }
    if !supervisor_alive && state.state == "running" {
        println!("  stale state: supervisor is not running");
    } else if supervisor_alive && !child_alive && state.state == "running" {
        println!("  stale state: sing-box is not running");
    } else if supervisor_alive && !tun_present && state.state == "running" {
        println!("  stale state: TUN interface is missing");
    } else if network_changed && state.state == "running" {
        println!("  stale state: default network/CIDR changed or unavailable");
    } else if supervisor_alive && heartbeat_age > 10 {
        println!("  stale state: heartbeat is old");
    }
    if crate::launchd::service_installed() {
        println!("  launchd service installed ({})", crate::launchd::LABEL);
    }
    println!("  logs {}", state.log_dir);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotates_logs_at_limit() {
        let path = std::env::temp_dir().join(format!("proxybot-rotate-{}.log", std::process::id()));
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("log.1"));
        let _ = fs::remove_file(path.with_extension("log.2"));
        let data = vec![b'x'; LOG_LIMIT as usize + 1];
        fs::write(&path, data).unwrap();
        rotate_if_needed(&path);
        assert!(!path.exists());
        assert!(path.with_extension("log.1").exists());
        fs::write(&path, b"new").unwrap();
        rotate_if_needed(&path); // below limit: no second rotation
        assert!(path.exists());
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("log.1"));
        let _ = fs::remove_file(path.with_extension("log.2"));
    }
}
