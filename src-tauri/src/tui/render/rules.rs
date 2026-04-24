//! Rules tab renderer.
//!
//! Shows the active rule set and allows navigation.

use ratatui::{Frame, layout::Rect, widgets::{Block, Borders, Paragraph}};

use crate::tui::TuiApp;

/// Render the Rules tab.
pub fn render(f: &mut Frame, area: Rect, _app: &TuiApp) {
    let content = Paragraph::new("Rule management (placeholder)\n\nRules will be listed here with enable/disable toggles.\nUse h/l or arrow keys to navigate, Enter to toggle.");
    f.render_widget(content.block(Block::default().borders(Borders::ALL).title("Rules")), area);
}