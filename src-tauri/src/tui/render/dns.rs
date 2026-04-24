//! DNS tab renderer.
//!
//! Shows DNS query log and upstream configuration.

use ratatui::{Frame, layout::Rect, widgets::{Block, Borders, Paragraph}};

use crate::tui::TuiApp;

/// Render the DNS tab.
pub fn render(f: &mut Frame, area: Rect, _app: &TuiApp) {
    let content = Paragraph::new("DNS query log (placeholder)\n\nShows DNS queries from connected devices with responses.\nUse j/k to scroll, 'c' to clear.");
    f.render_widget(content.block(Block::default().borders(Borders::ALL).title("DNS Queries")), area);
}