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
    /// Install pf gateway rules while capture runs (requires admin).
    #[arg(long)]
    gateway: bool,
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
    /// Install the pf redirect/NAT anchor and enable IP forwarding.
    Install {
        #[arg(long)]
        dry_run: bool,
        #[arg(long, default_value_t = 8089)]
        transparent_port: u16,
        #[arg(long)]
        interface: Option<String>,
        #[arg(long)]
        lan_cidr: Option<String>,
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
    }
}

fn run_gateway(args: GatewayArgs) -> Result<()> {
    let data_dir = default_data_dir();
    match args.command {
        GatewayCommand::Install {
            dry_run,
            transparent_port,
            interface,
            lan_cidr,
        } => {
            let mut config = gateway::detect(transparent_port)?;
            if let Some(interface) = interface {
                config.interface = interface;
            }
            if let Some(lan_cidr) = lan_cidr {
                config.lan_cidr = lan_cidr;
            }
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
    let data_dir = args.data_dir.clone().unwrap_or_else(default_data_dir);
    std::fs::create_dir_all(data_dir.join("rules"))?;

    let rules_path = rules_path(&data_dir);
    rules::Rules::ensure_file(&rules_path)?;
    let rules = Arc::new(RwLock::new(rules::Rules::load_or_default(&rules_path)));

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

    let transparent_addr: SocketAddr = ([127, 0, 0, 1], args.transparent_port).into();
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
        let config = gateway::detect(args.transparent_port)?;
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
    println!(
        "  CA page         http://{lan_ip}:{}/  (open on the test device)",
        args.ca_port
    );
    println!("  Data            {}", data_dir.display());
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
