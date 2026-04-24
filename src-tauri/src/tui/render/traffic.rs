//! Traffic tab renderer.
//!
//! Shows intercepted HTTP/HTTPS requests in a scrollable list.

use ratatui::{Frame, layout::Rect};
use ratatui::style::{Color, Stylize};
use ratatui::widgets::{Block, Borders, List, Paragraph};
use ratatui::text::Line;

use crate::tui::{TuiApp, input::format_ts, input::fmt_duration};

/// Render the Traffic tab.
pub fn render(f: &mut Frame, area: Rect, app: &TuiApp) {
    use ratatui::style::Color;
    use ratatui::widgets::List;
    use ratatui::text::Line;

    if app.traffic.requests.is_empty() {
        let empty = Paragraph::new("  No requests yet. Configure your device to use this proxy.")
            .block(Block::default().borders(Borders::ALL).title("Intercepted Traffic"));
        f.render_widget(empty, area);
    } else {
        let items: Vec<Line> = app.traffic.requests.iter().map(|req| {
            let method_color = match req.method.as_str() {
                "GET" => Color::Green,
                "POST" => Color::Cyan,
                "PUT" => Color::Yellow,
                "DELETE" => Color::Red,
                _ => Color::White,
            };
            let status_str = match req.status {
                Some(200..=299) => format!("{}", req.status.unwrap()).green(),
                Some(s) => format!("{}", s).red(),
                None => "-".yellow(),
            };
            let app_tag = req.app_tag.as_deref().unwrap_or("");
            let line = format!(
                " {}  {:<6}  {:<20} {:<30} {:>5} {:>8} {}",
                format_ts(&req.timestamp),
                req.method,
                req.host.chars().take(20).collect::<String>(),
                req.path.chars().take(30).collect::<String>(),
                status_str,
                fmt_duration(req.duration_ms),
                app_tag
            );
            Line::raw(line).style(method_color)
        }).collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Intercepted Traffic"));

        f.render_widget(list, area);
    }
}