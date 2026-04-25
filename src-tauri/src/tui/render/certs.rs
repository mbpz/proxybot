//! Certs tab renderer.
//!
//! Shows CA certificate information and management controls.

use ratatui::{
    Frame, layout::Rect,
    widgets::{Block, Borders, Paragraph, Wrap},
    style::Color,
};
use ratatui::text::{Line, Span};

use crate::tui::TuiApp;

/// Render the Certs tab.
pub fn render(f: &mut Frame, area: Rect, app: &TuiApp) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Certificates");

    // Gather cert info from cert_manager
    let fingerprint = app.cert_manager.get_ca_fingerprint();
    let (expiry_date, days) = app.cert_manager.get_ca_expiry();

    // Determine status badge
    let (status_text, status_color) = if days < 0 {
        ("Unknown".to_string(), Color::Gray)
    } else if days == 0 {
        ("Expired".to_string(), Color::Red)
    } else if days <= 30 {
        ("Expiring Soon".to_string(), Color::Yellow)
    } else {
        ("Valid".to_string(), Color::Green)
    };

    let ca_meta = app.cert_manager.get_ca_metadata();
    let created_at = ca_meta.as_ref().map(|m| {
        let secs = m.created_at;
        let hours = (secs / 3600) % 24;
        let mins = (secs % 3600) / 60;
        let secs_in_day = secs % 86400;
        let days_since_epoch = secs / 86400;
        format!("Day {} + {:02}:{:02}:{:02}", days_since_epoch, hours, mins, secs_in_day % 60)
    }).unwrap_or_else(|| "Unknown".to_string());

    let serial_str = ca_meta.as_ref().map(|m| m.serial.clone()).unwrap_or_else(String::new);

    // Build content lines
    let mut lines = Vec::new();

    // CA Info block
    lines.push(Line::from(vec![Span::raw("CA Certificate Info")]));
    lines.push(Line::from(vec![Span::raw("─".repeat(40))]));
    lines.push(Line::from(vec![Span::raw("Fingerprint (SHA1): "), Span::raw(&fingerprint).style(Color::Yellow)]));
    lines.push(Line::from(vec![Span::raw("Expiry: "), Span::raw(&expiry_date).style(Color::Cyan)]));
    lines.push(Line::from(vec![Span::raw("Created: "), Span::raw(&created_at).style(Color::Gray)]));
    lines.push(Line::from(vec![Span::raw("Status: "), Span::raw(&status_text).style(status_color)]));
    lines.push(Line::from(vec![Span::raw("Days until expiry: "), Span::raw(format!("{}", days)).style(status_color)]));

    if !serial_str.is_empty() {
        lines.push(Line::from(vec![Span::raw("Serial: "), Span::raw(&serial_str).style(Color::Gray)]));
    }

    lines.push(Line::from(vec![]));
    lines.push(Line::from(vec![Span::raw("─".repeat(40))]));
    lines.push(Line::from(vec![Span::raw("Actions")]));
    lines.push(Line::from(vec![Span::raw("─".repeat(40))]));
    lines.push(Line::from(vec![Span::raw("[r] Regenerate CA")]));
    lines.push(Line::from(vec![Span::raw("[e] Export CA PEM to ~/.proxybot/ca.crt")]));

    // Show regenerate status if any
    if let Some(ref status) = app.certs.regenerate_status {
        lines.push(Line::from(vec![]));
        lines.push(Line::from(vec![Span::raw("Regenerate: "), Span::raw(status).style(Color::Yellow)]));
    }

    // Show export path if any
    if let Some(ref path) = app.certs.export_path {
        lines.push(Line::from(vec![Span::raw("Export: "), Span::raw(path).style(Color::Green)]));
    }

    lines.push(Line::from(vec![]));
    lines.push(Line::from(vec![Span::raw("─".repeat(40))]));
    lines.push(Line::from(vec![Span::raw("Key bindings: r=regenerate, e=export, q=quit").style(Color::Gray)]));

    let content = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: true });

    f.render_widget(content, area);
}