//! ProxyBot TUI - Terminal UI for the HTTPS MITM proxy.
//!
//! Run with: cargo run --bin proxybot-tui --release
//!
//! Keyboard shortcuts:
//!   q / Esc    Quit
//!   Tab        Next tab
//!   Shift+Tab  Previous tab
//!   h/l        Previous/next tab
//!   r          Start proxy (if not running)
//!   s          Stop proxy
//!   c          Clear request list
//!   j/k / Up/Down   Navigate list

use crossterm::event::{self, KeyCode, KeyEventKind};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::execute;
use ratatui::backend::CrosstermBackend;
use ratatui::Frame;
use rusqlite::Connection;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use proxybot_lib::cert::CertManager;
use proxybot_lib::db::{DbState, RecentRequest};
use proxybot_lib::dns::DnsState;
use proxybot_lib::network::get_network_info;
use proxybot_lib::proxy::{start_proxy_core, InterceptedRequest};
use proxybot_lib::rules::RulesEngine;
use proxybot_lib::anomaly::AnomalyDetector;
use proxybot_lib::tun::TunState;
use proxybot_lib::replay::ReplayState as LibReplayState;

use proxybot_lib::proxy::ProxyState;

use proxybot_lib::tui::{TuiApp, Tab};
use proxybot_lib::tui::input::{InputAction, handle_key_event};

const PROXY_PORT: u16 = 8080;

/// Start the proxy using proxybot_lib's start_proxy_core.
fn start_proxy(
    app: &TuiApp,
) -> Result<tokio::sync::broadcast::Receiver<InterceptedRequest>, String> {
    if app.proxy_running.swap(true, Ordering::SeqCst) {
        return Err("Proxy already running".to_string());
    }

    let (event_tx, shutdown_tx) = start_proxy_core(
        app.cert_manager.clone(),
        app.dns_state.clone(),
        app.db_state.clone(),
    )?;

    // Store shutdown sender
    *app.shutdown_tx.lock().unwrap() = Some(shutdown_tx);

    Ok(event_tx)
}

/// Stop the proxy.
fn stop_proxy(app: &TuiApp) -> Result<(), String> {
    app.proxy_running.store(false, Ordering::SeqCst);
    if let Some(tx) = app.shutdown_tx.lock().unwrap().take() {
        let _ = tx.send(());
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    log::info!("Starting ProxyBot TUI");

    // Setup terminal
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    // Initialize subsystems
    let db_state = Arc::new(DbState::new().expect("Failed to initialize database"));
    let cert_manager = Arc::new(
        CertManager::new().expect("Failed to initialize certificate manager"),
    );
    let rules_engine = Arc::new(RulesEngine::new());
    let dns_state = Arc::new(
        DnsState::with_db(db_state.clone()).with_rules_engine(rules_engine.clone()),
    );
    let proxy_state = Arc::new(ProxyState::new());
    let anomaly_detector = Arc::new(AnomalyDetector::new());
    let tun_state = Arc::new(TunState::new());
    let replay_state = Arc::new(LibReplayState::default());

    // Get network info
    let network_info = get_network_info().ok();
    let _local_ip = network_info
        .as_ref()
        .map(|n| n.lan_ip.clone())
        .unwrap_or_else(|| "127.0.0.1".to_string());

    // Start file watcher in background
    let rules_engine2 = rules_engine.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
        rt.block_on(async {
            rules_engine2.start_watcher();
        });
    });

    // Create TUI app
    let mut app = TuiApp::new(
        db_state.clone(),
        cert_manager.clone(),
        rules_engine.clone(),
        dns_state.clone(),
        proxy_state,
        anomaly_detector,
        tun_state,
        replay_state,
    );

    // DB path for polling
    let db_path = {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        std::path::PathBuf::from(home).join(".proxybot").join("proxybot.db")
    };

    // Event receiver for real-time updates
    let mut event_rx: Option<tokio::sync::broadcast::Receiver<InterceptedRequest>> = None;

    // Start the proxy
    match start_proxy(&app) {
        Ok(rx) => {
            event_rx = Some(rx);
            log::info!("Proxy started on port {}", PROXY_PORT);
        }
        Err(e) => {
            log::error!("Failed to start proxy: {}", e);
        }
    }

    // Main loop
    let res = loop {
        // Poll for input
        if event::poll(Duration::from_millis(100))? {
            if let event::Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match handle_key_event(&key) {
                        InputAction::Quit => break Ok(()),
                        InputAction::NextTab => app.next_tab(),
                        InputAction::PrevTab => app.prev_tab(),
                        InputAction::StartProxy => {
                            if !app.proxy_running.load(Ordering::SeqCst) {
                                match start_proxy(&app) {
                                    Ok(rx) => {
                                        event_rx = Some(rx);
                                    }
                                    Err(e) => {
                                        log::error!("Failed to start proxy: {}", e);
                                    }
                                }
                            }
                        }
                        InputAction::StopProxy => {
                            if app.proxy_running.load(Ordering::SeqCst) {
                                let _ = stop_proxy(&app);
                                event_rx = None;
                            }
                        }
                        InputAction::Clear => {
                            app.traffic.requests.clear();
                            app.traffic.selected = 0;
                            app.traffic.last_id = 0;
                        }
                        InputAction::Up => {
                            if app.traffic.selected > 0 {
                                app.traffic.selected -= 1;
                            }
                        }
                        InputAction::Down => {
                            if app.traffic.selected < app.traffic.requests.len().saturating_sub(1) {
                                app.traffic.selected += 1;
                            }
                        }
                        InputAction::Enter | InputAction::None => {}
                    }
                }
            }
        }

        // Check for new events (non-blocking)
        if let Some(ref mut rx) = event_rx {
            while let Ok(req) = rx.try_recv() {
                let recent = RecentRequest {
                    id: 0,
                    timestamp: req.timestamp.clone(),
                    method: req.method.clone(),
                    scheme: req.scheme.clone(),
                    host: req.host.clone(),
                    path: req.path.clone(),
                    status: req.status,
                    duration_ms: req.latency_ms.map(|v| v as i64),
                    app_tag: req.app_name.clone(),
                };
                app.traffic.add_request(&recent);
            }
        }

        // Refresh DB for requests that came in via other channels
        if let Ok(conn) = Connection::open(&db_path) {
            refresh_traffic(&mut app, &conn);
        }

        // Render
        terminal.draw(|f| proxybot_lib::tui::render::render(&app, f))?;
    };

    // Cleanup
    let _ = stop_proxy(&app);
    terminal::disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    log::info!("ProxyBot TUI exited");

    res
}

/// Refresh traffic from database (polling pattern from original).
fn refresh_traffic(app: &mut TuiApp, conn: &Connection) {
    let query = if app.traffic.last_id == 0 {
        "SELECT id, timestamp, method, scheme, host, path, resp_status, duration_ms, app_tag
         FROM http_requests ORDER BY id DESC LIMIT 100"
    } else {
        "SELECT id, timestamp, method, scheme, host, path, resp_status, duration_ms, app_tag
         FROM http_requests WHERE id > ?1 ORDER BY id DESC LIMIT 100"
    };

    let mut stmt = match conn.prepare(query) {
        Ok(s) => s,
        Err(_) => return,
    };

    let rows: Vec<RecentRequest> = if app.traffic.last_id == 0 {
        stmt.query_map([], |row| {
            Ok(RecentRequest {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                method: row.get(2)?,
                scheme: row.get(3)?,
                host: row.get(4)?,
                path: row.get(5)?,
                status: row.get(6)?,
                duration_ms: row.get(7)?,
                app_tag: row.get(8)?,
            })
        })
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default()
    } else {
        stmt.query_map([app.traffic.last_id], |row| {
            Ok(RecentRequest {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                method: row.get(2)?,
                scheme: row.get(3)?,
                host: row.get(4)?,
                path: row.get(5)?,
                status: row.get(6)?,
                duration_ms: row.get(7)?,
                app_tag: row.get(8)?,
            })
        })
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default()
    };

    if !rows.is_empty() {
        app.traffic.last_id = rows.first().map(|r| r.id).unwrap_or(0);
        // Insert at front (newest first), maintain scroll position
        let old_len = app.traffic.requests.len();
        app.traffic.requests.splice(0..0, rows);
        if app.traffic.selected >= old_len && old_len > 0 {
            app.traffic.selected = old_len.saturating_sub(1);
        } else if app.traffic.selected >= app.traffic.requests.len() {
            app.traffic.selected = app.traffic.requests.len().saturating_sub(1);
        }
    }
}