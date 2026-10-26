//! ProxyBot minimal: CLI entry point.
//!
//! One process owns the explicit proxy, the transparent adapter used by the pf
//! gateway redirect, and the local web UI.

mod ca;
mod capture;
mod gateway;
mod rules;
mod store;
mod transparent;
mod sniff;
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
    /// Start capture, the local web UI, and optionally the pf gateway.
    Start(StartArgs),
    /// Manage the macOS pf transparent gateway.
    Gateway(GatewayArgs),
    /// Show or reset app classification rules.
    Rules(RulesArgs),
    /// Print version information.
    Version,
    /// List every command with a detailed description and examples.
    List,
    /// Passive packet capture (flows, DNS, TLS SNI); requires root.
    Sniff(sniff::SniffArgs),
}

#[derive(Args)]
struct StartArgs {
    /// Explicit HTTP proxy port (device Wi-Fi HTTP proxy).
    #[arg(long, default_value_t = 8088)]
    proxy_port: u16,
    /// Transparent listener port used by the pf redirect.
    #[arg(long, default_value_t = 8089)]
    transparent_port: u16,
    /// Local web UI port (bound to 127.0.0.1).
    #[arg(long, default_value_t = 8899)]
    web_port: u16,
    /// LAN port that serves the CA page and certificate to the test device.
    #[arg(long, default_value_t = 8090)]
    ca_port: u16,
    /// Do not open the browser automatically.
    #[arg(long)]
    no_open: bool,
    /// EXPERIMENTAL: install pf gateway rules while capture runs (requires admin).
    #[arg(long)]
    gateway: bool,
    /// With --gateway, also block QUIC (UDP 80/443) to force TCP capture.
    #[arg(long)]
    block_quic: bool,
    /// Data directory for the CA, database, and rules.
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
    /// EXPERIMENTAL: install the pf redirect/NAT anchor and enable IP forwarding.
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
    /// Remove the pf anchor and restore forwarding state.
    Uninstall {
        #[arg(long)]
        dry_run: bool,
    },
    /// Show forwarding state and currently loaded anchor rules.
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
        Command::Version => {
            println!("proxybot {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Command::List => {
            print_command_list();
            Ok(())
        }
        Command::Sniff(args) => sniff::run(args),
    }
}

fn print_command_list() {
    let data_dir = default_data_dir();
    println!("ProxyBot {} — command reference", env!("CARGO_PKG_VERSION"));
    println!();
    println!("USAGE");
    println!("  proxybot [COMMAND] [OPTIONS]");
    println!("  proxybot                       same as `proxybot start`");
    println!();
    println!("DEFAULTS");
    println!("  data directory  {}", data_dir.display());
    println!("  web UI          http://127.0.0.1:8899  (loopback only)");
    println!("  CA page         http://<Mac LAN IP>:8090  (open on the test device)");
    println!();
    println!("CAPTURE");
    println!("  start");
    println!("      Start the explicit HTTP proxy, the local web UI and (with --gateway)");
    println!("      the pf side-router. Opens the web UI unless --no-open is set.");
    println!();
    println!("      --proxy-port <PORT>        device Wi-Fi HTTP proxy port (default 8088)");
    println!("      --transparent-port <PORT>  pf redirect target (default 8089)");
    println!("      --web-port <PORT>          loopback Web UI port (default 8899)");
    println!("      --ca-port <PORT>           LAN CA page port (default 8090)");
    println!("      --gateway                  install pf rules while running (requires admin);");
    println!("                                 transparent adapter listens on 0.0.0.0:<transparent-port>");
    println!("      --block-quic               with --gateway, drop QUIC (UDP 80/443) so apps");
    println!("                                 fall back to TCP/TLS and become capturable");
    println!("      --no-open                  do not open a browser");
    println!("      --data-dir <DIR>           CA, SQLite and rules directory");
    println!();
    println!("      Examples:");
    println!("        proxybot start                              # HTTP proxy mode (no admin)");
    println!("        proxybot start --gateway --block-quic       # side-router mode (admin)");
    println!("        proxybot start --proxy-port 9090 --web-port 9999");
    println!("        proxybot start --no-open --data-dir /tmp/proxybot");
    println!();
    println!("GATEWAY (EXPERIMENTAL — macOS pf; install/uninstall ask for admin)");
    println!("  gateway install [EXPERIMENTAL]");
    println!("      Write the pf anchor, enable IP forwarding and load redirect/NAT rules.");
    println!("      Not recommended yet: pf redirect delivery is unreliable on macOS and a");
    println!("      stopped gateway can leave the device without internet.");
    println!("      --dry-run                  print generated rules and commands only");
    println!("      --transparent-port <PORT>  redirect target port (default 8089)");
    println!("      --interface <NAME>         override detected LAN interface (e.g. en0)");
    println!("      --lan-cidr <CIDR>          override detected LAN subnet (e.g. 192.168.1.0/24)");
    println!("      --block-quic               also block UDP 80/443 for the LAN subnet");
    println!();
    println!("  gateway uninstall [--dry-run]");
    println!("      Remove rules, flush the anchor and restore the previous IP-forwarding value.");
    println!();
    println!("  gateway status");
    println!("      Print IP forwarding state and the currently loaded anchor rules.");
    println!();
    println!("      Examples:");
    println!("        proxybot gateway install --dry-run");
    println!("        proxybot gateway install --block-quic");
    println!("        proxybot gateway status");
    println!("        proxybot gateway uninstall");
    println!();
    println!("RULES (app classification)");
    println!("  rules path    Print the active YAML rules path.");
    println!("  rules show    Print the active YAML rules.");
    println!("  rules reset   Restore the built-in rules file (restart to reload).");
    println!();
    println!("      Examples:");
    println!("        proxybot rules path");
    println!("        proxybot rules show | head -40");
    println!("        proxybot rules reset");
    println!();
    println!("SNIFF (EXPERIMENTAL; passive observation, requires sudo)");
    println!("  sniff");
    println!("      Capture packets on the default-route interface and print flow,");
    println!("      DNS query and TLS SNI metadata. Does not proxy or modify traffic.");
    println!("      --interface <NAME>     capture interface (default: default route)");
    println!("      --device <IP>          only traffic for this device IP");
    println!("      --duration <SECONDS>   stop automatically (default: until Ctrl+C)");
    println!("      --log <FILE>           append JSON-lines events to a file");
    println!();
    println!("      Examples:");
    println!("        sudo proxybot sniff --device 192.168.3.20 --duration 30");
    println!("        sudo proxybot sniff --interface en1 --log /tmp/proxybot-sniff.jsonl");
    println!();
    println!("MISC");
    println!("  version       Print the CLI version.");
    println!("  list          Print this reference.");
    println!();
    println!("ENVIRONMENT");
    println!("  PROXYBOT_DATA_DIR=<DIR>          same as --data-dir");
    println!("  PROXYBOT_TRANSPARENT_BIND=all    dev/test override: bind the transparent");
    println!("                                   adapter on 0.0.0.0 outside --gateway mode");
    println!("  Installer (minimal/install.sh):");
    println!("    PROXYBOT_VERSION=0.1.10         pin a CLI release");
    println!("    PROXYBOT_PREFIX=/usr/local/bin install prefix (default ~/.local/bin)");
    println!("    PROXYBOT_LOCAL_BINARY=<path>   install an already-built binary");
    println!("    PROXYBOT_DOWNLOAD_BASE=<url>   mirror serving proxybot-min-*.tar.gz");
    println!("    PROXYBOT_REPO=owner/repo       override the GitHub repository");
    println!();
    println!("NOTES");
    println!("  * HTTP proxy mode: device Wi-Fi HTTP proxy = <Mac IP>:8088; no admin.");
    println!("  * Gateway mode: device Wi-Fi router/gateway = <Mac IP>, HTTP proxy OFF;");
    println!("    set a manual IP in the same subnet and keep DNS on the router.");
    println!("  * Gateway mode is experimental; HTTP proxy mode is the supported path.");
    println!("  * Install/trust the CA from http://<Mac IP>:8090 on the test device.");
    println!("  * Certificate-pinned apps may reject the CA (TLS tab shows SNI attempts);");
    println!("    add their domains to <data>/rules/passthrough.txt to keep them working.");
    println!("  * Ctrl+C stops capture and, in --gateway mode, removes the pf rules.");
    println!("  * The Web UI is loopback-only; the CA page and transparent adapter are on the LAN.");
}


fn run_gateway(args: GatewayArgs) -> Result<()> {
    let data_dir = default_data_dir();
    match args.command {
        GatewayCommand::Install {
            dry_run,
            transparent_port,
            interface,
            lan_cidr,
            block_quic,
        } => {
            eprintln!(
                "WARNING: gateway mode is experimental on macOS. Verify the device keeps\n\
                 internet access and use `proxybot gateway uninstall` afterwards."
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
        GatewayCommand::Status => {
            println!("{}", gateway::status()?);
            Ok(())
        }
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
    });
    let http_state = capture_state.clone();
    let ws_state = capture_state.clone();

    let lan_ip = gateway::detect(args.transparent_port)
        .map(|config| config.lan_ip)
        .unwrap_or_else(|_| "127.0.0.1".to_string());
    let setup = web::SetupInfo {
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
    println!(
        "  Gateway mode    device Wi-Fi gateway = {lan_ip}  (pf redirect :80/:443 -> :{})",
        args.transparent_port
    );
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

fn default_data_dir() -> PathBuf {
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
