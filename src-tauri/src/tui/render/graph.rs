//! Graph tab renderer.
//!
//! Shows traffic DAG / dependency graph visualization.

use ratatui::{Frame, layout::Rect, widgets::{Block, Borders, Paragraph}};

use crate::tui::TuiApp;

/// Render the Graph tab.
pub fn render(f: &mut Frame, area: Rect, _app: &TuiApp) {
    let content = Paragraph::new("Traffic dependency graph (placeholder)\n\nShows API call relationships as a graph.\nUse arrow keys to navigate nodes.");
    f.render_widget(content.block(Block::default().borders(Borders::ALL).title("Graph")), area);
}