//! Alerts tab renderer.
//!
//! Shows anomaly detection alerts.

use ratatui::{Frame, layout::Rect, widgets::{Block, Borders, Paragraph}};

use crate::tui::TuiApp;

/// Render the Alerts tab.
pub fn render(f: &mut Frame, area: Rect, _app: &TuiApp) {
    let content = Paragraph::new("Anomaly alerts (placeholder)\n\nAlerts from the anomaly detector will appear here.\nUse j/k to navigate, Enter to acknowledge.");
    f.render_widget(content.block(Block::default().borders(Borders::ALL).title("Alerts")), area);
}