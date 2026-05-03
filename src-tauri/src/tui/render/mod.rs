//! Render module for per-tab rendering.
//!
//! Dispatches to the appropriate tab renderer based on the current tab.

pub mod traffic;
pub mod rules;
pub mod devices;
pub mod certs;
pub mod dns;
pub mod alerts;
pub mod replay;
pub mod graph;
pub mod gen;

use ratatui::{Frame, layout::{Rect, Constraint, Direction, Layout}, widgets::Paragraph};
use ratatui::style::{Stylize, Color};

use super::{Tab, TuiApp, CertWizard};
use super::wizard::render_wizard;

/// Render the tab bar at the top of the screen.
pub fn render_tab_bar(f: &mut Frame, area: Rect, current_tab: Tab) {
    // First row: Traffic, Rules, Devices, Certs, DNS
    // Second row: Alerts, Replay, Graph, Gen

    let tabs_row1 = [Tab::Traffic, Tab::Rules, Tab::Devices, Tab::Certs, Tab::Dns];
    let tabs_row2 = [Tab::Alerts, Tab::Replay, Tab::Graph, Tab::Gen];

    let width = area.width as usize;
    let row1_width: usize = tabs_row1.iter().map(|t| t.label().len() + 3).sum();
    let row2_width: usize = tabs_row2.iter().map(|t| t.label().len() + 3).sum();

    let mut row1_text = String::new();
    for tab in &tabs_row1 {
        if *tab == current_tab {
            row1_text.push_str(&format!("[{}] ", tab.label()).cyan().to_string());
        } else {
            row1_text.push_str(&format!(" {}  ", tab.label()).dim().to_string());
        }
    }

    let mut row2_text = String::new();
    for tab in &tabs_row2 {
        if *tab == current_tab {
            row2_text.push_str(&format!("[{}] ", tab.label()).cyan().to_string());
        } else {
            row2_text.push_str(&format!(" {}  ", tab.label()).dim().to_string());
        }
    }

    // Calculate centered positions
    let row1_start = (width.saturating_sub(row1_width)) / 2;
    let row2_start = (width.saturating_sub(row2_width)) / 2;

    let padding = " ".repeat(row1_start);
    let line1 = Paragraph::new(format!("{}{}", padding, row1_text));
    let padding2 = " ".repeat(row2_start);
    let line2 = Paragraph::new(format!("{}{}", padding2, row2_text));

    f.render_widget(line1, Rect::new(area.x, area.y, area.width, 1));
    f.render_widget(line2, Rect::new(area.x, area.y + 1, area.width, 1));
}

/// Render the header bar with logo and status info.
pub fn render_header(f: &mut Frame, area: Rect, app: &TuiApp) {
    let proxy_running = app.proxy_running.load(std::sync::atomic::Ordering::SeqCst);
    let proxy_str = if proxy_running { "RUNNING" } else { "STOPPED" };

    // Check CA status from cert_manager
    let ca_installed = app.cert_manager.get_ca_metadata().is_some();
    let ca_status = if ca_installed { "CA: INSTALLED" } else { "CA: NOT INSTALLED" };

    // Build header line
    let header_text = format!(
        " PB ProxyBot v0.4.0 | Proxy: {} | {} | Requests: {} ",
        proxy_str,
        ca_status,
        app.traffic.requests.len()
    );

    let para = Paragraph::new(header_text)
        .style(ratatui::style::Style::new().fg(Color::White));

    f.render_widget(para, area);
}

/// Render the status bar at the bottom of the screen.
pub fn render_status_bar(f: &mut Frame, area: Rect, app: &TuiApp) {
    let proxy_status = if app.proxy_running.load(std::sync::atomic::Ordering::SeqCst) {
        "RUNNING"
    } else {
        "STOPPED"
    };

    let status_text = format!(
        "[q]uit [r]start [s]stop [c]lear | Tab: {:?} | Proxy: {} | Requests: {}",
        app.current_tab, proxy_status, app.traffic.requests.len()
    );

    let para = Paragraph::new(status_text);
    f.render_widget(para, area);
}

/// Dispatch render to the appropriate tab renderer.
pub fn render(app: &TuiApp, f: &mut Frame) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // tab bar (two rows)
            Constraint::Length(1),  // header bar
            Constraint::Min(10),    // content
            Constraint::Length(3), // status bar
        ])
        .split(f.size());

    // Tab bar
    render_tab_bar(f, chunks[0], app.current_tab);

    // Header with logo and status
    render_header(f, chunks[1], app);

    // Check if wizard is open and render it as an overlay
    if let Some(ref wizard) = app.wizard {
        // Render wizard in content area
        render_wizard(f, chunks[2], wizard);
    } else {
        // Normal tab rendering
        match app.current_tab {
            Tab::Traffic => traffic::render(f, chunks[2], app),
            Tab::Rules => rules::render(f, chunks[2], app),
            Tab::Devices => devices::render(f, chunks[2], app),
            Tab::Certs => certs::render(f, chunks[2], app),
            Tab::Dns => dns::render(f, chunks[2], app),
            Tab::Alerts => alerts::render(f, chunks[2], app),
            Tab::Replay => replay::render(f, chunks[2], app),
            Tab::Graph => graph::render(f, chunks[2], app),
            Tab::Gen => gen::render(f, chunks[2], app),
        }
    }

    // Status bar
    render_status_bar(f, chunks[3], app);
}
