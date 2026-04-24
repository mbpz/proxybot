//! Gen (Scaffold/Generate) tab renderer.
//!
//! Shows scaffold generation and API mocking controls.

use ratatui::{Frame, layout::Rect, widgets::{Block, Borders, Paragraph}};

use crate::tui::TuiApp;

/// Render the Gen tab.
pub fn render(f: &mut Frame, area: Rect, _app: &TuiApp) {
    let content = Paragraph::new("Scaffold generation (placeholder)\n\nGenerate mock API servers and scaffold projects.\nSelect an API pattern and press Enter to generate.");
    f.render_widget(content.block(Block::default().borders(Borders::ALL).title("Gen")), area);
}