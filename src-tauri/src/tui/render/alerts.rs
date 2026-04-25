//! Alerts tab renderer.
//!
//! Shows anomaly detection alerts with severity badges and baseline info.

use ratatui::{
    Frame,
    layout::{Rect, Constraint, Layout, Direction},
    widgets::{Block, Borders, Paragraph, Table, Row, Cell},
    style::{Color, Stylize, Style},
};

use crate::tui::TuiApp;
use crate::anomaly::AlertSeverity;

/// Render the Alerts tab with header stats and alert list.
pub fn render(f: &mut Frame, area: Rect, app: &TuiApp) {
    // Layout: header stats (2 lines), alert list (flex), controls (1 line)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // header with count + baseline stats
            Constraint::Min(10),    // alert list
            Constraint::Length(1),  // controls bar
        ])
        .split(area);

    render_header(f, chunks[0], app);
    render_alert_list(f, chunks[1], app);
    render_controls(f, chunks[2], app);
}

/// Render header with alert count and baseline info.
fn render_header(f: &mut Frame, area: Rect, app: &TuiApp) {
    let alerts = &app.alerts.alerts_list;
    let unack_count = alerts.iter().filter(|a| !a.acknowledged).count();
    let baseline = app.alerts.baseline_info.as_ref();

    let domain_count = baseline.map(|b| b.domains.len()).unwrap_or(0);
    let new_domain_alerts = alerts.iter().filter(|a| {
        matches!(a.alert_type, crate::anomaly::AlertType::NewDomain) && !a.acknowledged
    }).count();

    let header_text = format!(
        " Alerts: {} active | Baseline: {} domains | New domain alerts: {} [j/k] navigate [a] ack [c] clear all acknowledged",
        unack_count.to_string().red(),
        domain_count.to_string().cyan(),
        new_domain_alerts.to_string().yellow()
    );

    let para = Paragraph::new(header_text)
        .block(Block::default().borders(Borders::ALL).title("Alerts Summary"))
        .style(Color::White);

    f.render_widget(para, area);
}

/// Render the scrollable alert list with severity badges.
fn render_alert_list(f: &mut Frame, area: Rect, app: &TuiApp) {
    let alerts = &app.alerts.alerts_list;

    if alerts.is_empty() {
        let empty = Paragraph::new("  No alerts. New domains/IPs will trigger alerts here.")
            .block(Block::default().borders(Borders::ALL).title("Alerts"));
        f.render_widget(empty, area);
        return;
    }

    // Build table rows
    let selected = app.alerts.selected.min(alerts.len().saturating_sub(1));

    let rows: Vec<Row> = alerts.iter().enumerate().map(|(idx, alert)| {
        let sev_color = match alert.severity {
            AlertSeverity::Info => Color::Cyan,
            AlertSeverity::Warning => Color::Yellow,
            AlertSeverity::Critical => Color::Red,
        };
        let sev_badge = match alert.severity {
            AlertSeverity::Info => "SEV3",
            AlertSeverity::Warning => "SEV2",
            AlertSeverity::Critical => "SEV1",
        };
        let ack_marker = if alert.acknowledged { "*" } else { " " };

        let severity_cell = Cell::from(format!("[{}]", sev_badge)).style(sev_color);
        let ts_cell = Cell::from(alert.created_at.chars().take(19).collect::<String>());
        let desc_cell = Cell::from(alert.details.chars().take(60).collect::<String>());
        let type_cell = Cell::from(format!("{:?}", alert.alert_type));

        let row = Row::new(vec![severity_cell, ts_cell, desc_cell, type_cell])
            .style(if idx == selected {
                Style::default().bg(Color::DarkGray)
            } else if alert.acknowledged {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default()
            });

        row
    }).collect();

    let widths = [
        Constraint::Length(10),  // severity badge
        Constraint::Length(20),  // timestamp
        Constraint::Length(60),  // description
        Constraint::Length(15),  // type
    ];
    let table = Table::new(rows, widths)
        .block(Block::default().borders(Borders::ALL).title("Alerts"))
        .highlight_style(Color::Cyan);

    let mut list_state = ratatui::widgets::TableState::default().with_selected(Some(selected));
    f.render_stateful_widget(table, area, &mut list_state);
}

/// Render bottom controls bar.
fn render_controls(f: &mut Frame, area: Rect, _app: &TuiApp) {
    let controls = Paragraph::new("[j/k] up/down  [a] ack  [c] clear all  [Enter] view detail");
    f.render_widget(controls, area);
}