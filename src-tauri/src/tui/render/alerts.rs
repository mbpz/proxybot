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
use crate::tui::i18n::{I18nKey as K, t as tr};
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

    let alerts_title = tr(K::AlertsTitle);
    let header_text = format!(
        " {}: {} {} | {}: {} | {}: {} {} {} {}",
        alerts_title,
        unack_count.to_string().red(),
        tr(K::AlertsActive),
        tr(K::AlertsBaseline),
        domain_count.to_string().cyan(),
        tr(K::AlertsNewDomainAlerts),
        new_domain_alerts.to_string().yellow(),
        tr(K::AlertsNavigateHint),
        tr(K::AlertsAckHint),
        tr(K::AlertsClearHint)
    );

    let alerts_summary_title = tr(K::AlertsSummary);
    let para = Paragraph::new(header_text)
        .block(Block::default().borders(Borders::ALL).title(alerts_summary_title.as_str()))
        .style(Color::White);

    f.render_widget(para, area);
}

/// Render the scrollable alert list with severity badges.
fn render_alert_list(f: &mut Frame, area: Rect, app: &TuiApp) {
    let alerts = &app.alerts.alerts_list;

    if alerts.is_empty() {
        let alerts_title = tr(K::AlertsTitle);
        let empty = Paragraph::new(format!("  {}", tr(K::AlertsEmpty)))
            .block(Block::default().borders(Borders::ALL).title(alerts_title.as_str()));
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
            AlertSeverity::Info => tr(K::AlertsSev3),
            AlertSeverity::Warning => tr(K::AlertsSev2),
            AlertSeverity::Critical => tr(K::AlertsSev1),
        };
        let _ack_marker = if alert.acknowledged { "*" } else { " " };

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
    let alerts_title = tr(K::AlertsTitle);
    let table = Table::new(rows, widths)
        .block(Block::default().borders(Borders::ALL).title(alerts_title.as_str()))
        .highlight_style(Color::Cyan);

    let mut list_state = ratatui::widgets::TableState::default().with_selected(Some(selected));
    f.render_stateful_widget(table, area, &mut list_state);
}

/// Render bottom controls bar.
fn render_controls(f: &mut Frame, area: Rect, _app: &TuiApp) {
    let controls_text = format!("{}  {}  {}  {}", tr(K::AlertsNavigateHint), tr(K::AlertsAckHint), tr(K::AlertsClearHint), tr(K::AlertsEnterDetail));
    let controls = Paragraph::new(controls_text);
    f.render_widget(controls, area);
}