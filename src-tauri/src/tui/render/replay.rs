//! Replay tab renderer.
//!
//! Shows recorded requests and replay controls.

use ratatui::{Frame, layout::Rect, widgets::{Block, Borders, Paragraph}};

use crate::tui::TuiApp;

/// Render the Replay tab.
pub fn render(f: &mut Frame, area: Rect, _app: &TuiApp) {
    let content = Paragraph::new("Request replay (placeholder)\n\nRecorded requests for replay testing.\nSelect a request and press Enter to replay.");
    f.render_widget(content.block(Block::default().borders(Borders::ALL).title("Replay")), area);
}