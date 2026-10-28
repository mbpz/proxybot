//! ProxyBot minimal: CLI entry point.
//!
//! One process owns the explicit proxy, the transparent adapter used by the pf
//! gateway redirect, and the local web UI.

mod ca;
mod capture;
mod gateway;
mod launchd;
mod maintenance;
mod gateway_run;
mod rules;
mod store;
mod transparent;
mod sniff;
mod singbox_config;
mod tun_probe;
mod web;

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{broadcast, oneshot, RwLock};

#[derive(Parser)]
#[command(
    name = "proxybot",
    version,
    about = "Minimal macOS traffic debugger for mobile apps"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Start the HTTP proxy and local web UI.
    Start(StartArgs),
    /// Manage Gateway Beta (TUN + sing-box).
    Gateway(GatewayArgs),
    /// Show or reset app rules.
    #[command(hide = true)]
    Rules(RulesArgs),
    /// Print version information.
    Version,
    /// Remove this binary and installed files.
    Uninstall(maintenance::UninstallArgs),
    /// Download and install a release.
    Update(maintenance::UpdateArgs),
    /// Print the core command reference.
    List,
    /// Experimental passive capture (root).
    #[command(hide = true)]
    Sniff(sniff::SniffArgs),
    /// Generate the Gateway sing-box config.
    #[command(hide = true)]
    SingboxConfig(singbox_config::SingboxConfigArgs),
    /// Experimental TUN ingress probe (root).
    #[command(hide = true)]
    TunProbe(tun_probe::TunProbeArgs),
}

#[derive(Args)]
struct StartArgs {
    /// HTTP proxy port.
    #[arg(long, default_value_t = 8088)]
    proxy_port: u16,
    /// Plain-HTTP adapter port.
    #[arg(long, default_value_t = 8089)]
    transparent_port: u16,
    /// Local web UI port.
    #[arg(long, default_value_t = 8899)]
    web_port: u16,
    /// LAN CA page port.
    #[arg(long, default_value_t = 8090)]
    ca_port: u16,
    /// Do not open a browser.
    #[arg(long)]
    no_open: bool,
    /// Legacy pf gateway (unsupported).
    #[arg(long)]
    gateway: bool,
    /// Legacy QUIC block (with --gateway only).
    #[arg(long)]
    block_quic: bool,
    /// Data directory.
    #[arg(long, env = "PROXYBOT_DATA_DIR")]
    data_dir: Option<PathBuf>,
}

impl Default for StartArgs {
    fn default() -> Self {
        Self {
            proxy_port: 8088,
            transparent_port: 8089,
            web_port: 8899,
            ca_port: 8090,
            no_open: false,
            gateway: false,
            block_quic: false,
            data_dir: None,
        }
    }
}

#[derive(Args)]
struct GatewayArgs {
    #[command(subcommand)]
    command: GatewayCommand,
}

#[derive(Subcommand)]
enum GatewayCommand {
    /// Start Gateway Beta (root).
    Start {
        /// Reject UDP/443 (QUIC fallback test).
        #[arg(long)]
        block_quic: bool,
        /// sing-box binary.
        #[arg(long)]
        singbox: Option<std::path::PathBuf>,
        /// TUN interface name.
        #[arg(long, default_value = "utun6")]
        tun: String,
        /// Single-device metadata IP.
        #[arg(long, conflicts_with = "devices")]
        client_ip: Option<String>,
        /// Per-device source-IP routing file.
        #[arg(long)]
        devices: Option<std::path::PathBuf>,
    },
    /// Stop Gateway Beta and restore networking.
    Stop,
    /// Install Gateway service (root).
    InstallService(launchd::InstallServiceArgs),
    /// Remove Gateway service and clean up (root).
    UninstallService,
    /// Internal foreground supervisor entry point used by launchd.
    #[command(hide = true)]
    Service(launchd::ServiceArgs),
    /// Internal: detached supervisor owning sing-box and network recovery.
    #[command(hide = true)]
    Supervise(gateway_run::SuperviseArgs),
    /// LEGACY: old pf redirect/NAT anchor; use `gateway start` instead.
    #[command(hide = true)]
    Install {
        #[arg(long)]
        dry_run: bool,
        #[arg(long, default_value_t = 8089)]
        transparent_port: u16,
        #[arg(long)]
        interface: Option<String>,
        #[arg(long)]
        lan_cidr: Option<String>,
        /// Drop QUIC (UDP 80/443) so apps fall back to TCP/TLS capture.
        #[arg(long)]
        block_quic: bool,
    },
    /// LEGACY: remove the old pf anchor and restore forwarding state.
    #[command(hide = true)]
    Uninstall {
        #[arg(long)]
        dry_run: bool,
    },
    /// Show Gateway state and health.
    Status,
}

#[derive(Args)]
struct RulesArgs {
    #[command(subcommand)]
    command: RulesCommand,
}

#[derive(Subcommand)]
enum RulesCommand {
    /// Print the active rules file path.
    Path,
    /// Print the active rules YAML.
    Show,
    /// Restore the built-in rules file.
    Reset,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "proxybot=info,warn".into()),
        )
        .init();

    let cli = Cli::parse();
    match cli.command.unwrap_or(Command::Start(StartArgs::default())) {
        Command::Start(args) => run_start(args).await,
        Command::Gateway(args) => run_gateway(args),
        Command::Rules(args) => run_rules(args),
        Command::Uninstall(args) => maintenance::uninstall(args),
        Command::Update(args) => maintenance::update(args),
        Command::Version => {
            println!("proxybot {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Command::List => {
            print_command_list();
            Ok(())
        }
        Command::Sniff(args) => sniff::run(args),
        Command::SingboxConfig(args) => singbox_config::run(args),
        Command::TunProbe(args) => tun_probe::run(args),
    }
}

fn print_command_list() {
    println!("ProxyBot {} — core commands", env!("CARGO_PKG_VERSION"));
    println!();
    println!("USAGE");
    println!("  proxybot [command]");
    println!();
    println!("CORE");
    println!("  start                 HTTP proxy + local web UI");
    println!("  gateway start         Gateway Beta (root; sing-box required)");
    println!("  gateway stop          Stop gateway and restore networking");
    println!("  gateway status        Show gateway state and health");
    println!("  uninstall             Remove binary and installed files");
    println!("  update                Download and install a release");
    println!("  version               Show version");
    println!("  list                  Show this reference");
    println!();
    println!("DEVICE");
    println!("  HTTP proxy            <Mac IP>:8088");
    println!("  CA page               http://<Mac IP>:8090/");
    println!("  Web UI                http://127.0.0.1:8899");
    println!();
    println!("Advanced commands remain available but are not listed here.");
}

fn run_gateway(args: GatewayArgs) -> Result<()> {
    let data_dir = default_data_dir();
    match args.command {
        GatewayCommand::Start { block_quic, singbox, tun, client_ip, devices } => {
            gateway_run::start(block_quic, singbox, tun, client_ip, devices)
        }
        GatewayCommand::Stop => gateway_run::stop(),
        GatewayCommand::InstallService(args) => launchd::install(args),
        GatewayCommand::UninstallService => launchd::uninstall(),
        GatewayCommand::Service(args) => launchd::run_service(args),
        GatewayCommand::Supervise(args) => gateway_run::supervise(args),
        GatewayCommand::Install {
            dry_run,
            transparent_port,
            interface,
            lan_cidr,
            block_quic,
        } => {
            eprintln!(
                "WARNING: this pf redirect/NAT gateway is LEGACY and unsupported.\n\
                 Use `proxybot gateway start` for the current TUN gateway."
            );
            let mut config = gateway::detect(transparent_port)?;
            if let Some(interface) = interface {
                config.interface = interface;
            }
            if let Some(lan_cidr) = lan_cidr {
                config.lan_cidr = lan_cidr;
            }
            config.block_quic = block_quic;
            gateway::install(&config, &data_dir, dry_run)
        }
        GatewayCommand::Uninstall { dry_run } => gateway::uninstall(&data_dir, dry_run),
        GatewayCommand::Status => gateway_run::status(),
    }
}

fn run_rules(args: RulesArgs) -> Result<()> {
    let path = rules_path(&default_data_dir());
    match args.command {
        RulesCommand::Path => {
            println!("{}", path.display());
            Ok(())
        }
        RulesCommand::Show => {
            rules::Rules::ensure_file(&path)?;
            println!("{}", std::fs::read_to_string(&path)?);
            Ok(())
        }
        RulesCommand::Reset => {
            std::fs::create_dir_all(path.parent().unwrap_or(std::path::Path::new(".")))?;
            std::fs::write(&path, rules::DEFAULT_RULES_YAML)
                .with_context(|| format!("write {}", path.display()))?;
            println!("rules reset at {}", path.display());
            Ok(())
        }
    }
}

async fn run_start(args: StartArgs) -> Result<()> {
    // pf redirects LAN traffic to the Mac's LAN IP (not loopback), so the
    // transparent adapter must listen on all interfaces in gateway mode.
    // PROXYBOT_TRANSPARENT_BIND=all is an explicit override for tests/dev.
    let transparent_bind_all = args.gateway
        || std::env::var("PROXYBOT_TRANSPARENT_BIND")
            .map(|value| value == "all")
            .unwrap_or(false);
    let transparent_bind: SocketAddr = if transparent_bind_all {
        ([0, 0, 0, 0], args.transparent_port).into()
    } else {
        ([127, 0, 0, 1], args.transparent_port).into()
    };

    ensure_port_available(
        ([0, 0, 0, 0], args.proxy_port).into(),
        "explicit proxy",
    )?;
    ensure_port_available(transparent_bind, "transparent adapter")?;
    ensure_port_available(([127, 0, 0, 1], args.web_port).into(), "web UI")?;
    ensure_port_available(([0, 0, 0, 0], args.ca_port).into(), "device CA page")?;

    let data_dir = args.data_dir.clone().unwrap_or_else(default_data_dir);
    std::fs::create_dir_all(data_dir.join("rules"))?;

    let rules_path = rules_path(&data_dir);
    rules::Rules::ensure_file(&rules_path)?;
    let rules = Arc::new(RwLock::new(rules::Rules::load_or_default(&rules_path)));

    let passthrough_path = data_dir.join("rules").join("passthrough.txt");
    let passthrough = Arc::new(RwLock::new(load_passthrough(&passthrough_path)));

    let ca_material = ca::load_or_create(&data_dir)?;
    let ca_pem = Arc::new(ca_material.cert_pem.clone());
    let issuer = ca::issuer(&ca_material)?;
    let authority = hudsucker::certificate_authority::RcgenAuthority::new(
        issuer,
        30 * 24 * 3600,
        hudsucker::rustls::crypto::aws_lc_rs::default_provider(),
    );

    let store = Arc::new(store::Store::open(&data_dir.join("history.sqlite"))?);
    let (events, _) = broadcast::channel::<String>(512);
    let capture_state = Arc::new(capture::CaptureState {
        store: store.clone(),
        events: events.clone(),
        rules: rules.clone(),
        passthrough: passthrough.clone(),
        sni_by_client: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        source_by_client: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        identity_by_client: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
    });
    let http_state = capture_state.clone();
    let ws_state = capture_state.clone();

    let lan_ip = gateway::detect(args.transparent_port)
        .map(|config| config.lan_ip)
        .unwrap_or_else(|_| "127.0.0.1".to_string());
    let quic_policy = std::fs::read_to_string(data_dir.join("quic-policy"))
        .map(|value| value.trim().to_string())
        .unwrap_or_else(|_| "direct".to_string());
    let setup = web::SetupInfo {
        quic_policy,
        ca_url: format!("http://{lan_ip}:{}/ca.crt", args.ca_port),
        ca_page_url: format!("http://{lan_ip}:{}/", args.ca_port),
        lan_ip: lan_ip.clone(),
        proxy_port: args.proxy_port,
        transparent_port: args.transparent_port,
        web_port: args.web_port,
        ca_port: args.ca_port,
        gateway_active: args.gateway,
    };
    let web_state = web::WebState {
        store: store.clone(),
        events: events.clone(),
        rules: rules.clone(),
        rules_path: rules_path.clone(),
        ca_pem,
        setup: Arc::new(setup),
    };

    // Explicit proxy (device Wi-Fi HTTP proxy) + transparent adapter (gateway).
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let proxy_addr: SocketAddr = ([0, 0, 0, 0], args.proxy_port).into();
    let proxy = hudsucker::Proxy::builder()
        .with_addr(proxy_addr)
        .with_ca(authority)
        .with_rustls_connector(hudsucker::rustls::crypto::aws_lc_rs::default_provider())
        .with_http_handler(capture::CaptureHandler::new(http_state))
        .with_websocket_handler(capture::CaptureHandler::new(ws_state))
        .with_graceful_shutdown(async move {
            let _ = shutdown_rx.await;
        })
        .build()
        .context("build MITM proxy")?;
    tokio::spawn(async move {
        if let Err(error) = proxy.start().await {
            tracing::error!(%error, "MITM proxy stopped");
        }
    });

    let transparent_addr = transparent_bind;
    let explicit_addr: SocketAddr = ([127, 0, 0, 1], args.proxy_port).into();
    tokio::spawn(async move {
        if let Err(error) = transparent::serve(transparent_addr, explicit_addr).await {
            tracing::error!(%error, "transparent adapter stopped");
        }
    });

    let web_addr: SocketAddr = ([127, 0, 0, 1], args.web_port).into();
    let web_task = tokio::spawn(web::serve(web_state.clone(), web_addr));
    let ca_state = web_state.clone();
    let ca_addr: SocketAddr = ([0, 0, 0, 0], args.ca_port).into();
    let ca_task = tokio::spawn(web::serve_ca_page(ca_state, ca_addr));

    let mut gateway_installed = false;
    if args.gateway {
        eprintln!(
            "WARNING: --gateway is experimental on macOS. pf redirect delivery is unreliable\n\
             and a stopped/failed gateway can leave the test device without internet.\n\
             Prefer plain `proxybot start` with the device Wi-Fi HTTP proxy."
        );
        let mut config = gateway::detect(args.transparent_port)?;
        config.block_quic = args.block_quic;
        gateway::install(&config, &data_dir, false)?;
        gateway_installed = true;
    }

    let web_url = format!("http://127.0.0.1:{}", args.web_port);
    println!();
    println!("ProxyBot is running");
    println!("  Web UI          {web_url}");
    println!(
        "  Explicit proxy  {lan_ip}:{}  (device Wi-Fi HTTP proxy)",
        args.proxy_port
    );
    if args.gateway {
        println!("  Gateway         LEGACY pf side-router enabled (unsupported)");
    } else {
        println!("  Gateway Beta    sudo proxybot gateway start  (TUN + sing-box; root)");
    }
    if args.gateway && args.block_quic {
        println!("  QUIC            blocked for the device subnet (forces TCP fallback)");
    }
    println!(
        "  CA page         http://{lan_ip}:{}/  (open on the test device)",
        args.ca_port
    );
    println!("  Data            {}", data_dir.display());
    println!("  Passthrough     {}", passthrough_path.display());
    println!("Press Ctrl+C to stop.");
    println!();

    if !args.no_open {
        let _ = std::process::Command::new("open").arg(&web_url).status();
    }

    tokio::signal::ctrl_c().await?;
    println!("stopping...");
    let _ = shutdown_tx.send(());
    if gateway_installed {
        if let Err(error) = gateway::uninstall(&data_dir, false) {
            eprintln!("warning: failed to remove gateway rules: {error}");
        }
    }
    web_task.abort();
    ca_task.abort();
    Ok(())
}

fn ensure_port_available(addr: SocketAddr, label: &str) -> Result<()> {
    std::net::TcpListener::bind(addr).map(|listener| drop(listener)).with_context(|| {
        format!(
            "{label} port {addr} is already in use; stop the previous `proxybot` process (Ctrl+C or `pkill -f 'proxybot start'`) and retry"
        )
    })
}

pub(crate) fn default_data_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("PROXYBOT_DATA_DIR") {
        return PathBuf::from(dir);
    }
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Library/Application Support/ProxyBot")
}

fn rules_path(data_dir: &std::path::Path) -> PathBuf {
    data_dir.join("rules").join("apps.yaml")
}

/// Domain suffixes that are forwarded without TLS interception. Pinned apps
/// (CertificateUnknown) keep working with this list; their traffic is visible
/// as a TLS attempt but cannot be decrypted.
fn load_passthrough(path: &std::path::Path) -> Vec<String> {
    if !path.exists() {
        let _ = std::fs::write(
            path,
            "# Host suffixes to pass through WITHOUT TLS interception.\n             # Use this for certificate-pinned apps (e.g. douyin.com) so they keep\n             # working; traffic is still recorded as a TLS attempt, but not decrypted.\n             # One suffix per line; subdomains match automatically.\n             # douyin.com\n",
        );
    }
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_string)
        .collect()
}
