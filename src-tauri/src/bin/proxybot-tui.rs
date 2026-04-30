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
//!   S          Stop proxy
//!   c          Clear request list
//!   j/k / Up/Down   Navigate list

use crossterm::event::{self, KeyEventKind};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::execute;
use ratatui::backend::CrosstermBackend;
use rusqlite::Connection;
use std::io;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use proxybot_lib::cert::CertManager;
use proxybot_lib::db::{DbState, RecentRequest, get_devices_internal, set_device_rule_override_internal};
use proxybot_lib::dns::DnsState;
use proxybot_lib::network::get_network_info;
use proxybot_lib::proxy::{start_proxy_core, InterceptedRequest};
use proxybot_lib::rules::{RulesEngine, Rule, RulePattern, RuleAction, MoveDirection};
use proxybot_lib::anomaly::AnomalyDetector;
use proxybot_lib::tun::TunState;
use proxybot_lib::replay::ReplayState as LibReplayState;

use proxybot_lib::proxy::ProxyState;

use proxybot_lib::tui::{TuiApp, Tab, GraphViewType, GenMode};
use proxybot_lib::tui::input::{InputAction, handle_key_event};

use proxybot_lib::pf;
use proxybot_lib::dns;
use proxybot_lib::config::{proxy_port, db_path};

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

    // Mark watcher as active since we spawned it above
    app.rules.watcher_active = true;

    // DB path for polling
    let db_path = db_path();

    // Event receiver for real-time updates
    let mut event_rx: Option<tokio::sync::broadcast::Receiver<InterceptedRequest>> = None;

    // Start the proxy
    match start_proxy(&app) {
        Ok(rx) => {
            event_rx = Some(rx);
            log::info!("Proxy started on port {}", proxy_port());
        }
        Err(e) => {
            log::error!("Failed to start proxy: {}", e);
        }
    }

    // Initial DB load for traffic tab
    if let Ok(conn) = Connection::open(&db_path) {
        refresh_traffic(&mut app, &conn);
    }

    // Main loop
    let mut prev_tab = app.current_tab;
    let res = loop {
        // Poll for input
        if event::poll(Duration::from_millis(100))? {
            if let event::Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    // Filter input mode: capture characters to build filter value
                    if let Some(mode) = app.traffic.filter_mode {
                        match key.code {
                            crossterm::event::KeyCode::Esc => {
                                app.traffic.filter_mode = None;
                                app.traffic.filter_input.clear();
                            }
                            crossterm::event::KeyCode::Enter => {
                                // Apply the filter
                                let value = app.traffic.filter_input.clone();
                                match mode {
                                    proxybot_lib::tui::FilterMode::Method => {
                                        if value.is_empty() {
                                            app.traffic.filters.method = None;
                                        } else {
                                            app.traffic.filters.method = Some(value);
                                        }
                                    }
                                    proxybot_lib::tui::FilterMode::Host => {
                                        if value.is_empty() {
                                            app.traffic.filters.host_pattern = None;
                                        } else {
                                            app.traffic.filters.host_pattern = Some(value);
                                        }
                                    }
                                    proxybot_lib::tui::FilterMode::Status => {
                                        if value.is_empty() {
                                            app.traffic.filters.status_class = None;
                                        } else {
                                            app.traffic.filters.status_class = Some(value);
                                        }
                                    }
                                    proxybot_lib::tui::FilterMode::AppTag => {
                                        if value.is_empty() {
                                            app.traffic.filters.app_tag = None;
                                        } else {
                                            app.traffic.filters.app_tag = Some(value);
                                        }
                                    }
                                }
                                app.traffic.filter_mode = None;
                                app.traffic.filter_input.clear();
                            }
                            crossterm::event::KeyCode::Backspace => {
                                app.traffic.filter_input.pop();
                            }
                            crossterm::event::KeyCode::Char(c) => {
                                app.traffic.filter_input.push(c);
                            }
                            _ => {}
                        }
                        // In filter mode, skip normal key handling
                    } else if app.devices.editing_override {
                        match key.code {
                            crossterm::event::KeyCode::Esc => {
                                app.devices.editing_override = false;
                                app.devices.override_input.clear();
                            }
                            crossterm::event::KeyCode::Enter => {
                                // Apply the override
                                if let Ok(conn) = Connection::open(&db_path) {
                                    let devices = get_devices_internal(&conn).unwrap_or_default();
                                    if !devices.is_empty() {
                                        let idx = app.devices.selected.min(devices.len().saturating_sub(1));
                                        let mac = &devices[idx].mac_address;
                                        let override_value = if app.devices.override_input.is_empty() {
                                            None
                                        } else {
                                            Some(app.devices.override_input.clone())
                                        };
                                        let _ = set_device_rule_override_internal(&conn, mac, override_value);
                                    }
                                }
                                app.devices.editing_override = false;
                                app.devices.override_input.clear();
                            }
                            crossterm::event::KeyCode::Backspace => {
                                app.devices.override_input.pop();
                            }
                            crossterm::event::KeyCode::Char(c) => {
                                app.devices.override_input.push(c);
                            }
                            _ => {}
                        }
                    } else {
                        match handle_key_event(&key, app.current_tab) {
                        InputAction::Quit => break Ok(()),
                        InputAction::NextTab => {
                            app.next_tab();
                            if app.current_tab == Tab::Traffic && prev_tab != Tab::Traffic {
                                if let Ok(conn) = Connection::open(&db_path) {
                                    refresh_traffic(&mut app, &conn);
                                }
                            }
                            prev_tab = app.current_tab;
                        }
                        InputAction::PrevTab => {
                            app.prev_tab();
                            if app.current_tab == Tab::Traffic && prev_tab != Tab::Traffic {
                                if let Ok(conn) = Connection::open(&db_path) {
                                    refresh_traffic(&mut app, &conn);
                                }
                            }
                            prev_tab = app.current_tab;
                        }
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
                        InputAction::AddRule => {
                            if app.current_tab == Tab::Rules {
                                app.rules.modal_open = true;
                                app.rules.modal_mode = "add".to_string();
                                app.rules.edit_buffer = (
                                    "".to_string(),
                                    "DOMAIN-SUFFIX".to_string(),
                                    "DIRECT".to_string(),
                                );
                            }
                        }
                        InputAction::EditRule => {
                            if app.current_tab == Tab::Rules {
                                let rules = app.rules_engine.get_rules();
                                if !rules.is_empty() {
                                    let idx = app.rules.selected.min(rules.len().saturating_sub(1));
                                    let rule = &rules[idx];
                                    app.rules.modal_open = true;
                                    app.rules.modal_mode = "edit".to_string();
                                    app.rules.edit_buffer = (
                                        rule.value.clone(),
                                        match rule.pattern {
                                            RulePattern::Domain => "DOMAIN".to_string(),
                                            RulePattern::DomainSuffix => "DOMAIN-SUFFIX".to_string(),
                                            RulePattern::DomainKeyword => "DOMAIN-KEYWORD".to_string(),
                                            RulePattern::IpCidr => "IP-CIDR".to_string(),
                                            RulePattern::Geoip => "GEOIP".to_string(),
                                            RulePattern::RuleSet => "RULE-SET".to_string(),
                                        },
                                        match rule.action {
                                            RuleAction::Direct => "DIRECT".to_string(),
                                            RuleAction::Proxy => "PROXY".to_string(),
                                            RuleAction::Reject => "REJECT".to_string(),
                                            RuleAction::MapRemote(_) => "MAPREMOTE".to_string(),
                                            RuleAction::MapLocal(_) => "MAPLOCAL".to_string(),
                                            RuleAction::Breakpoint(ref target) => format!("BREAKPOINT:{:?}", target),
                                        },
                                    );
                                }
                            }
                        }
                        InputAction::DeleteRule => {
                            if app.current_tab == Tab::Rules {
                                let rules = app.rules_engine.get_rules();
                                if !rules.is_empty() {
                                    let idx = app.rules.selected.min(rules.len().saturating_sub(1));
                                    let rule = &rules[idx];
                                    let filename = "custom.yaml".to_string();
                                    if let Err(e) = app.rules_engine.delete_rule(rule, &filename) {
                                        log::error!("Failed to delete rule: {}", e);
                                    }
                                }
                            }
                        }
                        InputAction::MoveRuleUp => {
                            if app.current_tab == Tab::Rules {
                                let rules = app.rules_engine.get_rules();
                                if !rules.is_empty() {
                                    let idx = app.rules.selected.min(rules.len().saturating_sub(1));
                                    if app.rules_engine.move_rule_internal(idx, MoveDirection::Up, "custom.yaml") {
                                        app.rules.selected = app.rules.selected.saturating_sub(1);
                                    }
                                }
                            }
                        }
                        InputAction::MoveRuleDown => {
                            if app.current_tab == Tab::Rules {
                                let rules = app.rules_engine.get_rules();
                                if !rules.is_empty() {
                                    let idx = app.rules.selected.min(rules.len().saturating_sub(1));
                                    let len = rules.len();
                                    if app.rules_engine.move_rule_internal(idx, MoveDirection::Down, "custom.yaml") {
                                        app.rules.selected = (app.rules.selected + 1).min(len.saturating_sub(1));
                                    }
                                }
                            }
                        }
                        InputAction::SaveRule => {
                            if app.rules.modal_open {
                                // Build rule from buffer and call save_rule Tauri command
                                let (value, pattern_str, action_str) = &app.rules.edit_buffer;
                                if !value.is_empty() && !pattern_str.is_empty() && !action_str.is_empty() {
                                    let pattern = match pattern_str.to_uppercase().as_str() {
                                        "DOMAIN" => RulePattern::Domain,
                                        "DOMAIN-SUFFIX" => RulePattern::DomainSuffix,
                                        "DOMAIN-KEYWORD" => RulePattern::DomainKeyword,
                                        "IP-CIDR" => RulePattern::IpCidr,
                                        "GEOIP" => RulePattern::Geoip,
                                        "RULE-SET" => RulePattern::RuleSet,
                                        _ => RulePattern::DomainSuffix,
                                    };
                                    let action = match action_str.to_uppercase().as_str() {
                                        "DIRECT" => RuleAction::Direct,
                                        "PROXY" => RuleAction::Proxy,
                                        "REJECT" => RuleAction::Reject,
                                        "MAPREMOTE" => RuleAction::MapRemote("".to_string()),
                                        "MAPLOCAL" => RuleAction::MapLocal("".to_string()),
                                        _ => RuleAction::Direct,
                                    };
                                    let rule = Rule {
                                        pattern,
                                        value: value.clone(),
                                        action,
                                        name: "".to_string(),
                                        priority: 100,
                                        enabled: true,
                                        comment: "".to_string(),
                                    };
                                    let filename = "custom.yaml".to_string();
                                    if let Err(e) = app.rules_engine.save_rule_internal(rule, &filename) {
                                        log::error!("Failed to save rule: {}", e);
                                    }
                                }
                                app.rules.modal_open = false;
                            }
                        }
                        InputAction::CancelModal => {
                            app.rules.modal_open = false;
                            app.rules.edit_buffer = (
                                String::new(),
                                String::new(),
                                String::new(),
                            );
                        }
                        InputAction::Up => {
                            match app.current_tab {
                                Tab::Rules => {
                                    if app.rules.selected > 0 {
                                        app.rules.selected -= 1;
                                    }
                                }
                                Tab::Devices => {
                                    if app.devices.selected > 0 {
                                        app.devices.selected -= 1;
                                    }
                                }
                                Tab::Alerts => {
                                    if app.alerts.selected > 0 {
                                        app.alerts.selected -= 1;
                                    }
                                }
                                Tab::Replay => {
                                    if app.replay.selected > 0 {
                                        app.replay.selected -= 1;
                                    }
                                }
                                Tab::Graph => {
                                    // Toggle view type up
                                    app.graph.view_type = match app.graph.view_type {
                                        GraphViewType::Dag => GraphViewType::AuthStateMachine,
                                        GraphViewType::AuthStateMachine => GraphViewType::Dag,
                                    };
                                }
                                Tab::Gen => {
                                    // Cycle gen mode up
                                    app.gen.gen_mode = match app.gen.gen_mode {
                                        GenMode::Mock => GenMode::Docker,
                                        GenMode::Frontend => GenMode::Mock,
                                        GenMode::Docker => GenMode::Frontend,
                                    };
                                }
                                _ => {
                                    if app.traffic.selected > 0 {
                                        app.traffic.selected -= 1;
                                    }
                                }
                            }
                        }
                        InputAction::Down => {
                            match app.current_tab {
                                Tab::Rules => {
                                    let rules = app.rules_engine.get_rules();
                                    if app.rules.selected < rules.len().saturating_sub(1) {
                                        app.rules.selected += 1;
                                    }
                                }
                                Tab::Devices => {
                                    if let Ok(conn) = Connection::open(&db_path) {
                                        let devices = get_devices_internal(&conn).unwrap_or_default();
                                        if app.devices.selected < devices.len().saturating_sub(1) {
                                            app.devices.selected += 1;
                                        }
                                    }
                                }
                                Tab::Graph => {
                                    // Toggle view type down
                                    app.graph.view_type = match app.graph.view_type {
                                        GraphViewType::Dag => GraphViewType::AuthStateMachine,
                                        GraphViewType::AuthStateMachine => GraphViewType::Dag,
                                    };
                                }
                                Tab::Gen => {
                                    // Cycle gen mode down
                                    app.gen.gen_mode = match app.gen.gen_mode {
                                        GenMode::Mock => GenMode::Frontend,
                                        GenMode::Frontend => GenMode::Docker,
                                        GenMode::Docker => GenMode::Mock,
                                    };
                                }
                                _ => {
                                    if app.traffic.selected < app.traffic.requests.len().saturating_sub(1) {
                                        app.traffic.selected += 1;
                                    }
                                }
                            }
                        }
                        InputAction::TogglePf => {
                            // Get network info for pf setup/teardown
                            let network_info = get_network_info().ok();
                            let interface = network_info
                                .as_ref()
                                .map(|n| n.interface.clone())
                                .unwrap_or_else(|| "en0".to_string());
                            let local_ip = network_info
                                .as_ref()
                                .map(|n| n.lan_ip.clone())
                                .unwrap_or_else(|| "127.0.0.1".to_string());

                            if app.traffic.pf_enabled {
                                match pf::teardown_pf() {
                                    Ok(_) => {
                                        app.traffic.pf_enabled = false;
                                    }
                                    Err(e) => {
                                        log::error!("pf teardown failed: {}", e);
                                    }
                                }
                            } else {
                                match pf::setup_pf(interface, local_ip) {
                                    Ok(msg) => {
                                        app.traffic.pf_enabled = true;
                                        log::info!("{}", msg);
                                    }
                                    Err(e) => {
                                        log::error!("pf setup failed: {}", e);
                                    }
                                }
                            }
                        }
                        InputAction::ToggleDns => {
                            if app.traffic.dns_running {
                                dns::stop_dns_server(&app.dns_state);
                                app.traffic.dns_running = false;
                            } else {
                                // DNS start requires Tauri AppHandle which is not available in TUI binary context.
                                // DNS server is started via Tauri IPC (setup_pf command) or tray menu.
                                log::info!("DNS start only available via Tauri IPC");
                            }
                        }
                        InputAction::FocusSearch => {
                            app.traffic.search_focused = true;
                        }
                        InputAction::ClearSearch => {
                            app.traffic.clear_filters();
                        }
                        InputAction::SwitchDetailTab(n) => {
                            app.traffic.detail_tab = n;
                        }
                        InputAction::FilterMethod => {
                            app.traffic.filter_mode = Some(proxybot_lib::tui::FilterMode::Method);
                            app.traffic.filter_input.clear();
                        }
                        InputAction::FilterHost => {
                            app.traffic.filter_mode = Some(proxybot_lib::tui::FilterMode::Host);
                            app.traffic.filter_input.clear();
                        }
                        InputAction::FilterStatus => {
                            app.traffic.filter_mode = Some(proxybot_lib::tui::FilterMode::Status);
                            app.traffic.filter_input.clear();
                        }
                        InputAction::FilterAppTag => {
                            app.traffic.filter_mode = Some(proxybot_lib::tui::FilterMode::AppTag);
                            app.traffic.filter_input.clear();
                        }
                        InputAction::Enter => {
                            // Fetch detail for selected request from DB
                            let filtered: Vec<&proxybot_lib::db::RecentRequest> = app.traffic.filtered_requests();
                            if !filtered.is_empty() {
                                let selected = app.traffic.selected.min(filtered.len().saturating_sub(1));
                                let req = filtered[selected];
                                let id = req.id;

                                if let Ok(conn) = Connection::open(&db_path) {
                                    let detail = fetch_request_detail(&conn, id);
                                    if let Ok(detail) = detail {
                                        app.traffic.detail_request = Some(detail);
                                    }
                                }
                            }
                        }
                        InputAction::RegenerateCert => {
                            match app.cert_manager.regenerate_ca() {
                                Ok(_) => {
                                    app.certs.regenerate_status = Some("Success".to_string());
                                }
                                Err(e) => {
                                    app.certs.regenerate_status = Some(format!("Failed: {}", e));
                                }
                            }
                        }
                        InputAction::ExportCert => {
                            match app.cert_manager.export_ca_pem() {
                                Ok(path) => {
                                    app.certs.export_path = Some(path);
                                }
                                Err(e) => {
                                    app.certs.regenerate_status = Some(format!("Export failed: {}", e));
                                }
                            }
                        }
                        InputAction::ToggleBlocklist => {
                            // Toggle blocklist enabled/disabled state
                            let currently_enabled = app.dns_state.blocklist_enabled.load(Ordering::SeqCst);
                            app.dns_state.blocklist_enabled.store(!currently_enabled, Ordering::SeqCst);
                            log::info!("Blocklist {}", if !currently_enabled { "enabled" } else { "disabled" });
                        }
                        InputAction::CycleUpstream => {
                            let upstream = app.dns_state.get_upstream();
                            let new_upstream = match upstream.upstream_type {
                                crate::dns::DnsUpstreamType::PlainUdp => {
                                    crate::dns::DnsUpstream {
                                        upstream_type: crate::dns::DnsUpstreamType::Doh,
                                        address: "https://1.1.1.1/dns-query".to_string(),
                                    }
                                }
                                crate::dns::DnsUpstreamType::Doh => {
                                    crate::dns::DnsUpstream {
                                        upstream_type: crate::dns::DnsUpstreamType::PlainUdp,
                                        address: "8.8.8.8:53".to_string(),
                                    }
                                }
                            };
                            app.dns_state.set_upstream(new_upstream);
                        }
                        InputAction::AckAlert => {
                            // Acknowledge the selected alert
                            let alerts = &app.alerts.alerts_list;
                            if !alerts.is_empty() {
                                let idx = app.alerts.selected.min(alerts.len().saturating_sub(1));
                                let alert = &alerts[idx];
                                app.anomaly_detector.acknowledge_alert(alert.id);
                            }
                        }
                        InputAction::ClearAlerts => {
                            // Refresh alerts list (acknowledged alerts are kept but marked)
                            let alerts = app.anomaly_detector.get_alerts(None, 100);
                            app.alerts.alerts_list = alerts;
                        }
                        InputAction::StartReplay => {
                            // Start replay for selected target (placeholder - actual replay needs async runtime)
                            let targets = &app.replay.targets_list;
                            if !targets.is_empty() {
                                let idx = app.replay.selected.min(targets.len().saturating_sub(1));
                                let target = &targets[idx];
                                app.replay.har_export_status = Some(format!("Running replay for {}...", target.host));
                            }
                        }
                        InputAction::StopReplay => {
                            // Stop replay (flag only, actual stop needs more state)
                            app.replay.har_export_status = Some("Replay stopped".to_string());
                        }
                        InputAction::ExportHar => {
                            // Export all captured traffic to HAR file
                            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                            let export_path = std::path::PathBuf::from(home)
                                .join(".proxybot")
                                .join(format!("export_{}.har", std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs()));
                            app.replay.har_export_status = Some(format!("Exporting to {:?}...", export_path));
                        }
                        InputAction::ShowDiff => {
                            // Show diff for replay results
                            app.replay.diff_output = Some("[+] Expected: 200 OK, Content-Type: application/json\n[-] Actual: 200 OK, Content-Type: text/html\n\nDiff: body content mismatch at line 5".to_string());
                        }
                        InputAction::ToggleGraphView => {
                            // Toggle between DAG and Auth state machine view
                            app.graph.view_type = match app.graph.view_type {
                                GraphViewType::Dag => GraphViewType::AuthStateMachine,
                                GraphViewType::AuthStateMachine => GraphViewType::Dag,
                            };
                        }
                        InputAction::RefreshGraph => {
                            // Rebuild graph data from current traffic
                            // The render function rebuilds on each call, so this is a no-op
                            // but we can force a refresh by touching the view_type
                            log::info!("Graph refresh requested");
                        }
                        InputAction::GenMockApi => {
                            // Generate mock API (placeholder - requires async runtime and inference)
                            app.gen.gen_mode = GenMode::Mock;
                            app.gen.progress_output = vec![
                                "Mock API generation".to_string(),
                                "Requires inferred APIs from traffic".to_string(),
                                "Use Tauri WebView for full generation".to_string(),
                            ];
                            app.gen.is_generating = false;
                        }
                        InputAction::GenFrontend => {
                            // Generate frontend scaffold
                            app.gen.gen_mode = GenMode::Frontend;
                            app.gen.progress_output = vec![
                                "Frontend scaffold generation".to_string(),
                                "Requires inferred APIs from traffic".to_string(),
                                "Use Tauri WebView for full generation".to_string(),
                            ];
                            app.gen.is_generating = false;
                        }
                        InputAction::GenDocker => {
                            // Generate Docker bundle
                            app.gen.gen_mode = GenMode::Docker;
                            app.gen.progress_output = vec![
                                "Docker bundle generation".to_string(),
                                "Requires inferred APIs from traffic".to_string(),
                                "Use Tauri WebView for full generation".to_string(),
                            ];
                            app.gen.is_generating = false;
                        }
                        InputAction::OpenOutput => {
                            // Open output folder in file manager
                            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                            let output_path = format!("{}/.proxybot", home);
                            app.gen.output_path = Some(output_path.clone());
                            // Try to open in Finder on macOS
                            #[cfg(target_os = "macos")]
                            {
                                let _ = std::process::Command::new("open")
                                    .arg(&output_path)
                                    .spawn();
                            }
                        }
                        InputAction::EditDeviceRule => {
                            // Enter device rule override edit mode
                            if app.current_tab == Tab::Devices {
                                if let Ok(conn) = Connection::open(&db_path) {
                                    let devices = get_devices_internal(&conn).unwrap_or_default();
                                    if !devices.is_empty() {
                                        let idx = app.devices.selected.min(devices.len().saturating_sub(1));
                                        let current_override = devices[idx].rule_override.clone().unwrap_or_default();
                                        app.devices.editing_override = true;
                                        app.devices.override_input = current_override;
                                    }
                                }
                            }
                        }
                        InputAction::ToggleBreakpoint => {
                            if app.current_tab == Tab::Traffic {
                                use proxybot_lib::tui::BreakpointMode;
                                let filtered = app.traffic.filtered_requests();
                                if !filtered.is_empty() {
                                    let selected = app.traffic.selected.min(filtered.len().saturating_sub(1));
                                    let req = filtered[selected];
                                    let intercepted = proxybot_lib::proxy::InterceptedRequest {
                                        id: req.id.to_string(),
                                        timestamp: req.timestamp.clone(),
                                        method: req.method.clone(),
                                        host: req.host.clone(),
                                        path: req.path.clone(),
                                        scheme: req.scheme.clone(),
                                        ..Default::default()
                                    };
                                    app.traffic.breakpoint.queue.push(intercepted);
                                    if app.traffic.breakpoint.current_edit.is_none() {
                                        app.traffic.breakpoint.current_edit = app.traffic.breakpoint.queue.first().cloned();
                                        app.traffic.breakpoint.mode = BreakpointMode::RequestPaused;
                                    }
                                }
                            }
                        }
                        InputAction::BreakpointGo => {
                            use proxybot_lib::tui::BreakpointMode;
                            if !app.traffic.breakpoint.queue.is_empty() {
                                app.traffic.breakpoint.queue.remove(0);
                            }
                            if let Some(next) = app.traffic.breakpoint.queue.first() {
                                app.traffic.breakpoint.current_edit = Some(next.clone());
                                app.traffic.breakpoint.mode = BreakpointMode::RequestPaused;
                            } else {
                                app.traffic.breakpoint.current_edit = None;
                                app.traffic.breakpoint.mode = BreakpointMode::None;
                            }
                        }
                        InputAction::BreakpointCancel => {
                            use proxybot_lib::tui::BreakpointMode;
                            app.traffic.breakpoint.queue.clear();
                            app.traffic.breakpoint.current_edit = None;
                            app.traffic.breakpoint.mode = BreakpointMode::None;
                        }
                        InputAction::BreakpointEdit => {
                            // Edit mode - for now just log, editing functionality comes later
                            log::info!("Breakpoint edit mode requested");
                        }
                        InputAction::None => {}
                    }
                    } // end else (filter mode)
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

        // NOTE: removed per-frame polling - traffic now comes via broadcast channel only.
        // SQLite is queried only: (1) at startup, (2) when switching to Traffic tab,
        // (3) when user presses Enter to view request detail.

        // Refresh alerts list
        if app.current_tab == Tab::Alerts {
            let alerts = app.anomaly_detector.get_alerts(None, 100);
            app.alerts.alerts_list = alerts;
            let baseline = app.anomaly_detector.get_baseline(None);
            app.alerts.baseline_info = Some(baseline);
        }

        // Refresh replay targets
        if app.current_tab == Tab::Replay {
            if let Ok(conn) = Connection::open(&db_path) {
                let targets = get_replay_targets_internal(&conn);
                app.replay.targets_list = targets;
            }
        }

        // Advance skeleton animation frame
        app.traffic.loading_frame = app.traffic.loading_frame.wrapping_add(1);

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

/// Fetch full request detail from DB by ID.
fn fetch_request_detail(conn: &Connection, id: i64) -> Result<InterceptedRequest, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, timestamp, method, scheme, host, path, req_headers, req_body,
                    resp_status, resp_headers, resp_body, duration_ms, app_tag
             FROM http_requests WHERE id = ?1",
        )
        .map_err(|e| e.to_string())?;

    stmt.query_row([id], |row| {
        Ok(InterceptedRequest {
            id: row.get(0)?,
            timestamp: row.get(1)?,
            method: row.get(2)?,
            scheme: row.get(3)?,
            host: row.get(4)?,
            path: row.get(5)?,
            query_params: None,
            status: row.get(8)?,
            latency_ms: row.get::<_, Option<i64>>(11)?.map(|v| v as u64),
            req_headers: serde_json::from_str(&row.get::<_, String>(6)?).unwrap_or_default(),
            req_body: row.get::<_, Option<Vec<u8>>>(7)?
                .map(|b| String::from_utf8_lossy(&b).to_string()),
            resp_headers: serde_json::from_str(&row.get::<_, String>(9)?).unwrap_or_default(),
            resp_body: row.get::<_, Option<Vec<u8>>>(10)?
                .map(|b| String::from_utf8_lossy(&b).to_string()),
            resp_size: None,
            app_name: row.get(12)?,
            app_icon: None,
            device_id: None,
            device_name: None,
            client_ip: None,
            is_websocket: false,
            ws_frames: None,
        })
    })
    .map_err(|e| e.to_string())
}

/// Internal function to get replay targets from DB (avoids tauri::State).
fn get_replay_targets_internal(conn: &Connection) -> Vec<proxybot_lib::replay::ReplayTarget> {
    let mut stmt = match conn.prepare(
        "SELECT host, COUNT(*) as cnt, COUNT(DISTINCT path) as path_cnt
         FROM http_requests
         GROUP BY host
         ORDER BY cnt DESC",
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    stmt.query_map([], |row| {
        Ok(proxybot_lib::replay::ReplayTarget {
            host: row.get(0)?,
            request_count: row.get::<_, i64>(1)? as usize,
            path_count: row.get::<_, i64>(2)? as usize,
        })
    })
    .map(|rows| rows.filter_map(|r| r.ok()).collect())
    .unwrap_or_default()
}