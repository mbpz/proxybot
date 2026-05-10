//! Certs tab renderer.
//!
//! Shows CA certificate information and management controls.

use ratatui::text::{Line, Span};
use ratatui::{
    layout::Rect,
    style::Color,
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::tui::i18n::{t as tr, I18nKey as K};
use crate::tui::TuiApp;

/// Render the Certs tab.
pub fn render(f: &mut Frame, area: Rect, app: &TuiApp) {
    let certs_title = tr(K::CertsTitle);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(certs_title.as_str());

    // Gather cert info from cert_manager
    let fingerprint = app.cert_manager.get_ca_fingerprint();
    let (expiry_date, days) = app.cert_manager.get_ca_expiry();

    // Determine status badge
    let (status_text, status_color) = if days < 0 {
        (tr(K::CertsUnknown), Color::Gray)
    } else if days == 0 {
        (tr(K::CertsExpired), Color::Red)
    } else if days <= 30 {
        (tr(K::CertsExpiringSoon), Color::Yellow)
    } else {
        (tr(K::CertsValid), Color::Green)
    };

    let ca_meta = app.cert_manager.get_ca_metadata();
    let created_at = ca_meta
        .as_ref()
        .map(|m| {
            let secs = m.created_at;
            let hours = (secs / 3600) % 24;
            let mins = (secs % 3600) / 60;
            let secs_in_day = secs % 86400;
            let days_since_epoch = secs / 86400;
            format!(
                "Day {} + {:02}:{:02}:{:02}",
                days_since_epoch,
                hours,
                mins,
                secs_in_day % 60
            )
        })
        .unwrap_or_else(|| tr(K::CertsUnknown));

    let serial_str = ca_meta
        .as_ref()
        .map(|m| m.serial.clone())
        .unwrap_or_else(String::new);

    // Build content lines
    let mut lines = Vec::new();

    // CA Info block
    lines.push(Line::from(vec![Span::raw(tr(K::CertsCaInfo))]));
    lines.push(Line::from(vec![Span::raw("─".repeat(40))]));
    lines.push(Line::from(vec![
        Span::raw(tr(K::CertsFingerprintLabel)),
        Span::raw(&fingerprint).style(Color::Yellow),
    ]));
    lines.push(Line::from(vec![
        Span::raw(tr(K::CertsExpiryLabel)),
        Span::raw(&expiry_date).style(Color::Cyan),
    ]));
    lines.push(Line::from(vec![
        Span::raw(tr(K::CertsCreatedLabel)),
        Span::raw(&created_at).style(Color::Gray),
    ]));
    lines.push(Line::from(vec![
        Span::raw(tr(K::CertsStatusLabel)),
        Span::raw(&status_text).style(status_color),
    ]));
    lines.push(Line::from(vec![
        Span::raw(tr(K::CertsDaysUntilExpiry)),
        Span::raw(format!("{}", days)).style(status_color),
    ]));

    if !serial_str.is_empty() {
        lines.push(Line::from(vec![
            Span::raw(tr(K::CertsSerialLabel)),
            Span::raw(&serial_str).style(Color::Gray),
        ]));
    }

    lines.push(Line::from(vec![]));
    lines.push(Line::from(vec![Span::raw("─".repeat(40))]));
    lines.push(Line::from(vec![Span::raw(tr(K::CertsActions))]));
    lines.push(Line::from(vec![Span::raw("─".repeat(40))]));
    lines.push(Line::from(vec![Span::raw(tr(K::CertsRegenerate))]));
    lines.push(Line::from(vec![Span::raw(tr(K::CertsExport))]));

    // Show regenerate status if any
    if let Some(ref status) = app.certs.regenerate_status {
        lines.push(Line::from(vec![]));
        lines.push(Line::from(vec![
            Span::raw(tr(K::CertsRegenerateStatus)),
            Span::raw(status).style(Color::Yellow),
        ]));
    }

    // Show export path if any
    if let Some(ref path) = app.certs.export_path {
        lines.push(Line::from(vec![
            Span::raw(tr(K::CertsExportPath)),
            Span::raw(path).style(Color::Green),
        ]));
    }

    lines.push(Line::from(vec![]));
    lines.push(Line::from(vec![Span::raw("─".repeat(40))]));
    lines.push(Line::from(vec![
        Span::raw(tr(K::CertsKeyBinding)).style(Color::Gray)
    ]));

    let content = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });

    f.render_widget(content, area);
}
