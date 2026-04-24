//! ProxyBot TUI - Terminal UI for the HTTPS MITM proxy.
//!
//! Run with: cargo run --bin proxybot-tui --release
//!
//! Keyboard shortcuts:
//!   q          Quit
//!   ↑/↓        Navigate request list
//!   r          Start proxy (if not running)
//!   s          Stop proxy
//!   c          Clear request list

use crossterm::event::{self, KeyCode, KeyEventKind};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::execute;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::Stylize;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;
use rusqlite::Connection;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

// Import from proxybot_lib
use proxybot_lib::cert::CertManager;
use proxybot_lib::db::{DbState, RecentRequest};
use proxybot_lib::dns::DnsState;
use proxybot_lib::network::get_network_info;
use proxybot_lib::proxy::{start_proxy_core, InterceptedRequest};
use proxybot_lib::rules::RulesEngine;

const PROXY_PORT: u16 = 8080;

static PROXY_RUNNING: AtomicBool = AtomicBool::new(false);
static SHUTDOWN_TX: Mutex<Option<tokio::sync::oneshot::Sender<()>>> = Mutex::new(None);

/// Format timestamp for display (HH:MM:SS.ms).
fn format_ts(ts: &str) -> String {
    // ts is like "1745432100.123" or "2024-01-01 12:00:00"
    if ts.contains('.') {
        if let Ok(secs) = ts.split('.').next().unwrap_or("0").parse::<u64>() {
            let hours = (secs / 3600) % 24;
            let mins = (secs % 3600) / 60;
            let secs = secs % 60;
            return format!("{:02}:{:02}:{:02}", hours, mins, secs);
        }
    }
    // Try parsing as date string
    if ts.len() >= 19 {
        return ts[11..19].to_string();
    }
    ts.chars().take(12).collect()
}

/// Format duration in ms.
fn fmt_duration(ms: Option<i64>) -> String {
    match ms {
        Some(v) if v < 1000 => format!("{}ms", v),
        Some(v) => format!("{:.1}s", v as f64 / 1000.0),
        None => "-".to_string(),
    }
}

/// App state for TUI.
struct App {
    requests: Vec<RecentRequest>,
    selected: usize,
    proxy_running: bool,
    total_requests: usize,
    last_id: i64,
}

impl App {
    fn new() -> Self {
        Self {
            requests: Vec::new(),
            selected: 0,
            proxy_running: false,
            total_requests: 0,
            last_id: 0,
        }
    }

    fn refresh(&mut self, conn: &Connection) {
        // Get all requests since last_id
        let query = if self.last_id == 0 {
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

        let rows: Vec<RecentRequest> = if self.last_id == 0 {
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
            stmt.query_map([self.last_id], |row| {
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
            self.last_id = rows.first().map(|r| r.id).unwrap_or(0);
            self.total_requests += rows.len();
            // Insert at front (newest first), but maintain scroll position
            let old_len = self.requests.len();
            self.requests.splice(0..0, rows);
            if self.selected >= old_len && old_len > 0 {
                self.selected = old_len.saturating_sub(1);
            } else if self.selected >= self.requests.len() {
                self.selected = self.requests.len().saturating_sub(1);
            }
        }
    }

    fn clear(&mut self) {
        self.requests.clear();
        self.selected = 0;
        self.total_requests = 0;
        self.last_id = 0;
    }

    /// Add request from event channel (real-time update).
    fn add_request(&mut self, req: &InterceptedRequest) {
        let recent = RecentRequest {
            id: 0, // Don't need DB id for display
            timestamp: req.timestamp.clone(),
            method: req.method.clone(),
            scheme: req.scheme.clone(),
            host: req.host.clone(),
            path: req.path.clone(),
            status: req.status,
            duration_ms: req.latency_ms.map(|v| v as i64),
            app_tag: req.app_name.clone(),
        };
        self.total_requests += 1;
        self.requests.insert(0, recent);
        // Keep only 1000 in memory
        if self.requests.len() > 1000 {
            self.requests.pop();
        }
    }
}

/// Render the UI.
fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(10),   // Request list
            Constraint::Length(3), // Status bar
        ])
        .split(frame.size());

    // Header
    let header_text = if app.proxy_running {
        format!(
            " ProxyBot TUI | Proxy: {}:{} | Requests: {} ",
            PROXY_PORT,
            "RUNNING".green(),
            app.total_requests
        )
    } else {
        format!(
            " ProxyBot TUI | Proxy: {}:{} | Requests: {} ",
            PROXY_PORT,
            "STOPPED".red(),
            app.total_requests
        )
    };
    let header = Paragraph::new(header_text)
        .block(Block::default().borders(Borders::ALL).title("ProxyBot"));
    frame.render_widget(header, chunks[0]);

    // Request list
    if app.requests.is_empty() {
        let empty = Paragraph::new("  No requests yet. Configure your device to use this proxy.")
            .block(Block::default().borders(Borders::ALL).title("Intercepted Traffic"));
        frame.render_widget(empty, chunks[1]);
    } else {
        let items: Vec<ListItem> = app
            .requests
            .iter()
            .enumerate()
            .map(|(i, req)| {
                use ratatui::style::Color;
                let method_color = match req.method.as_str() {
                    "GET" => Color::Green,
                    "POST" => Color::Cyan,
                    "PUT" => Color::Yellow,
                    "DELETE" => Color::Red,
                    _ => Color::White,
                };
                let status_str = match req.status {
                    Some(200..=299) => format!("{}", req.status.unwrap()).green(),
                    Some(s) => format!("{}", s).red(),
                    None => "-".yellow(),
                };
                let app_tag = req.app_tag.as_deref().unwrap_or("");
                let line = format!(
                    " {}  {:<6}  {:<20} {:<30} {:>5} {:>8} {}",
                    format_ts(&req.timestamp),
                    req.method,
                    req.host.chars().take(20).collect::<String>(),
                    req.path.chars().take(30).collect::<String>(),
                    status_str,
                    fmt_duration(req.duration_ms),
                    app_tag
                );
                let mut item = ListItem::new(line);
                if i == app.selected {
                    item = item.fg(Color::Black).on_cyan();
                } else {
                    item = item.fg(method_color);
                }
                item
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Intercepted Traffic"))
            .highlight_style(ratatui::style::Style::new().reversed());
        frame.render_widget(list, chunks[1]);
    }

    // Status bar
    let status_text = format!("[q]uit [r]start [s]stop [c]lear | {} requests shown", app.requests.len());
    let status = Paragraph::new(status_text)
        .block(Block::default().borders(Borders::ALL).title("Controls"));
    frame.render_widget(status, chunks[2]);
}

/// Start the proxy using proxybot_lib's start_proxy_core.
fn start_proxy(
    db_state: Arc<DbState>,
    cert_manager: Arc<CertManager>,
    dns_state: Arc<DnsState>,
) -> Result<tokio::sync::broadcast::Receiver<InterceptedRequest>, String> {
    if PROXY_RUNNING.swap(true, Ordering::SeqCst) {
        return Err("Proxy already running".to_string());
    }

    let (event_tx, shutdown_tx) = start_proxy_core(cert_manager, dns_state, db_state)?;

    // Store shutdown sender
    *SHUTDOWN_TX.lock().unwrap() = Some(shutdown_tx);

    // Return receiver for events (subscribe to the broadcast)
    Ok(event_tx)
}

/// Stop the proxy.
fn stop_proxy() -> Result<(), String> {
    PROXY_RUNNING.store(false, Ordering::SeqCst);
    if let Some(tx) = SHUTDOWN_TX.lock().unwrap().take() {
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

    // Initialize state
    let db_state = Arc::new(DbState::new().expect("Failed to initialize database"));
    let cert_manager = Arc::new(
        CertManager::new().expect("Failed to initialize certificate manager"),
    );
    let rules_engine = Arc::new(RulesEngine::new());
    let dns_state = Arc::new(
        DnsState::with_db(db_state.clone())
            .with_rules_engine(rules_engine.clone()),
    );

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

    let mut app = App::new();

    // Poll DB for requests
    let db_path = {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        std::path::PathBuf::from(home).join(".proxybot").join("proxybot.db")
    };

    // Event receiver for real-time updates
    let mut event_rx: Option<tokio::sync::broadcast::Receiver<InterceptedRequest>> = None;

    // Start the proxy
    match start_proxy(db_state.clone(), cert_manager.clone(), dns_state.clone()) {
        Ok(rx) => {
            event_rx = Some(rx);
            app.proxy_running = true;
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
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break Ok(()),
                        KeyCode::Char('r') => {
                            if !app.proxy_running {
                                match start_proxy(db_state.clone(), cert_manager.clone(), dns_state.clone()) {
                                    Ok(rx) => {
                                        event_rx = Some(rx);
                                        app.proxy_running = true;
                                    }
                                    Err(e) => {
                                        log::error!("Failed to start proxy: {}", e);
                                    }
                                }
                            }
                        }
                        KeyCode::Char('s') => {
                            if app.proxy_running {
                                let _ = stop_proxy();
                                app.proxy_running = false;
                                event_rx = None;
                            }
                        }
                        KeyCode::Char('c') => {
                            app.clear();
                        }
                        KeyCode::Up => {
                            if app.selected > 0 {
                                app.selected -= 1;
                            }
                        }
                        KeyCode::Down => {
                            if app.selected < app.requests.len().saturating_sub(1) {
                                app.selected += 1;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // Check for new events (non-blocking)
        if let Some(ref mut rx) = event_rx {
            while let Ok(req) = rx.try_recv() {
                app.add_request(&req);
            }
        }

        // Refresh DB for requests that came in via other channels
        if let Ok(conn) = Connection::open(&db_path) {
            app.refresh(&conn);
        }

        // Render
        terminal.draw(|f| render(f, &app))?;
    };

    // Cleanup
    let _ = stop_proxy();
    terminal::disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    log::info!("ProxyBot TUI exited");

    res
}
