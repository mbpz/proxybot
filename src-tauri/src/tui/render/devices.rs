//! Devices tab renderer.
//!
//! Shows connected devices and their stats.

use ratatui::{Frame, layout::Rect, widgets::{Block, Borders, Paragraph}};

use crate::tui::TuiApp;

/// Render the Devices tab.
pub fn render(f: &mut Frame, area: Rect, _app: &TuiApp) {
    let content = Paragraph::new("Connected devices (placeholder)\n\nDevices will show IP, MAC, name, and traffic stats.\nUse j/k to navigate.");
    f.render_widget(content.block(Block::default().borders(Borders::ALL).title("Devices")), area);
}