//! Certs tab renderer.
//!
//! Shows certificate information and management.

use ratatui::{Frame, layout::Rect, widgets::{Block, Borders, Paragraph}};

use crate::tui::TuiApp;

/// Render the Certs tab.
pub fn render(f: &mut Frame, area: Rect, _app: &TuiApp) {
    let content = Paragraph::new("Certificate management (placeholder)\n\nShows CA cert status, expiration, and regeneration controls.\nPress 'r' to regenerate CA certificate.");
    f.render_widget(content.block(Block::default().borders(Borders::ALL).title("Certificates")), area);
}