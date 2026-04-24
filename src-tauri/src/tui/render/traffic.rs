//! Traffic tab renderer.
//!
//! Shows intercepted HTTP/HTTPS requests in a scrollable list with:
//! - Filter bar (method, host, status, app_tag)
//! - Regex search bar
//! - Split pane: request list (top 60%) + detail panel (bottom 40%)

use ratatui::{Frame, layout::{Rect, Constraint, Layout, Direction}, widgets::{Block, Borders, List, Paragraph}, style::{Color, Stylize}, text::Line};

use crate::tui::{TuiApp, input::format_ts, input::fmt_duration};
use crate::db::RecentRequest;

/// Render the Traffic tab with filters, split pane, and controls.
pub fn render(f: &mut Frame, area: Rect, app: &TuiApp) {
    // Split: top filter bar, middle content (list + detail), bottom controls
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // filter bar
            Constraint::Min(10),   // content area
            Constraint::Length(1), // controls bar
        ])
        .split(area);

    render_filter_bar(f, chunks[0], app);
    render_content(f, chunks[1], app);
    render_controls_bar(f, chunks[2], app);
}

/// Render the filter bar with method dropdown, host filter, status filter, app_tag filter, search.
fn render_filter_bar(f: &mut Frame, area: Rect, app: &TuiApp) {
    use ratatui::style::Color;
    use ratatui::widgets::Paragraph;

    let traffic = &app.traffic;

    // Filter indicators
    let method_str = traffic.filters.method.as_deref().unwrap_or("*");
    let host_str = traffic.filters.host_pattern.as_deref().unwrap_or("");
    let status_str = traffic.filters.status_class.as_deref().unwrap_or("*");
    let app_tag_str = traffic.filters.app_tag.as_deref().unwrap_or("");
    let search_str = if traffic.search_input.is_empty() {
        "/regex/".dim().to_string()
    } else {
        format!("/{}/", traffic.search_input).yellow().to_string()
    };

    let filter_line = format!(
        " Method:[{}] Host:[{:<15}] Status:[{}] App:[{:<10}] {} [press letter to set, / for search, Esc to clear]",
        method_str.yellow(),
        host_str.chars().take(15).collect::<String>().yellow(),
        status_str.green(),
        app_tag_str.chars().take(10).collect::<String>().cyan(),
        search_str,
    );

    let para = Paragraph::new(filter_line)
        .block(Block::default().borders(Borders::ALL).title("Filters"))
        .style(Color::White);

    f.render_widget(para, area);
}

/// Render the split content: request list (top 60%) + detail panel (bottom 40%).
fn render_content(f: &mut Frame, area: Rect, app: &TuiApp) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(60),
            Constraint::Percentage(40),
        ])
        .split(area);

    render_request_list(f, chunks[0], app);
    render_detail_panel(f, chunks[1], app);
}

/// Render the scrollable request list.
fn render_request_list(f: &mut Frame, area: Rect, app: &TuiApp) {
    use ratatui::widgets::List;

    let filtered: Vec<&RecentRequest> = app.traffic.filtered_requests();
    let selected = app.traffic.selected.min(filtered.len().saturating_sub(1));

    if filtered.is_empty() {
        let empty = Paragraph::new("  No requests match filters. Configure your device to use this proxy.")
            .block(Block::default().borders(Borders::ALL).title("Intercepted Traffic"));
        f.render_widget(empty, area);
    } else {
        let items: Vec<Line> = filtered.iter().map(|req| {
            let method_color = match req.method.as_str() {
                "GET" => Color::Green,
                "POST" => Color::Cyan,
                "PUT" => Color::Yellow,
                "DELETE" => Color::Red,
                "PATCH" => Color::Magenta,
                _ => Color::White,
            };
            let status_str = match req.status {
                Some(200..=299) => format!("{}", req.status.unwrap()).green(),
                Some(300..=399) => format!("{}", req.status.unwrap()).cyan(),
                Some(400..=499) => format!("{}", req.status.unwrap()).red(),
                Some(500..=599) => format!("{}", req.status.unwrap()).red(),
                Some(s) => format!("{}", s).red(),
                None => "-".yellow(),
            };
            let app_tag = req.app_tag.as_deref().unwrap_or("");
            let line = format!(
                " {}  {:<6}  {:<25} {:<30} {:>5} {:>8} {}",
                format_ts(&req.timestamp),
                req.method,
                req.host.chars().take(25).collect::<String>(),
                req.path.chars().take(30).collect::<String>(),
                status_str,
                fmt_duration(req.duration_ms),
                app_tag
            );
            Line::raw(line).style(method_color)
        }).collect();

        // Render with state to show selection cursor
        let mut list_state = ratatui::widgets::ListState::default().with_selected(Some(selected));
        f.render_stateful_widget(
            List::new(items)
                .block(Block::default().borders(Borders::ALL).title("Intercepted Traffic"))
                .highlight_style(Color::Cyan),
            area,
            &mut list_state,
        );
    }
}

/// Render the detail panel for the selected request.
fn render_detail_panel(f: &mut Frame, area: Rect, app: &TuiApp) {
    use ratatui::style::Color;

    let filtered: Vec<&RecentRequest> = app.traffic.filtered_requests();
    let selected = app.traffic.selected.min(filtered.len().saturating_sub(1));

    if filtered.is_empty() || app.traffic.detail_request.is_none() {
        let hint = if app.traffic.detail_request.is_none() && !filtered.is_empty() {
            " Press Enter on a request to load detail..."
        } else {
            " No request selected."
        };
        let para = Paragraph::new(hint.dim())
            .block(Block::default().borders(Borders::ALL).title("Request Detail"));
        f.render_widget(para, area);
        return;
    }

    let detail = app.traffic.detail_request.as_ref().unwrap();

    // Build detail lines
    let mut lines: Vec<Line> = Vec::new();

    // Summary line
    let summary = format!(
        " {} {} {} -> {} ({})",
        detail.method,
        detail.scheme,
        detail.host,
        detail.path,
        detail.status.map(|s| s.to_string()).unwrap_or_else(|| "-".to_string())
    );
    lines.push(Line::raw(summary).fg(Color::White).underlined());

    // Separator
    lines.push(Line::raw("--- Request Headers ---").style(Color::Yellow));
    for (k, v) in &detail.req_headers {
        lines.push(Line::raw(format!("  {}: {}", k, v)).style(Color::White));
    }

    // Request body
    lines.push(Line::raw("--- Request Body ---").style(Color::Yellow));
    if let Some(ref body) = detail.req_body {
        let body_display = if body.len() > 500 {
            format!("{}...", &body[..500])
        } else {
            body.clone()
        };
        lines.push(Line::raw(format_json(&body_display)).style(Color::Cyan));
    } else {
        lines.push(Line::raw("(empty)").fg(Color::DarkGray));
    }

    // Response status
    lines.push(Line::raw("--- Response ---").style(Color::Green));
    for (k, v) in &detail.resp_headers {
        lines.push(Line::raw(format!("  {}: {}", k, v)).style(Color::White));
    }

    // Response body
    lines.push(Line::raw("--- Response Body ---").style(Color::Green));
    if let Some(ref body) = detail.resp_body {
        let body_display = if body.len() > 500 {
            format!("{}...", &body[..500])
        } else {
            body.clone()
        };
        lines.push(Line::raw(format_json(&body_display)).style(Color::Cyan));
    } else {
        lines.push(Line::raw("(empty)").fg(Color::DarkGray));
    }

    // App info
    if let (Some(ref app_name), Some(ref device_name)) = (&detail.app_name, &detail.device_name) {
        lines.push(Line::raw(format!("App: {} | Device: {}", app_name, device_name)).style(Color::Magenta));
    }

    let para = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("Request Detail"))
        .scroll((app.traffic.detail_scroll.unwrap_or(0) as u16, 0));

    f.render_widget(para, area);
}

/// Simple JSON formatter - adds basic indentation for objects/arrays.
fn format_json(s: &str) -> String {
    let mut result = String::new();
    let mut indent: usize = 0;

    for ch in s.chars() {
        match ch {
            '{' | '[' => {
                result.push(ch);
                result.push('\n');
                indent += 2;
                result.push_str(&" ".repeat(indent));
            }
            '}' | ']' => {
                result.push('\n');
                indent = indent.saturating_sub(2);
                result.push_str(&" ".repeat(indent));
                result.push(ch);
            }
            ',' => {
                result.push(ch);
                result.push('\n');
                result.push_str(&" ".repeat(indent));
            }
            _ => result.push(ch),
        }
    }
    result
}

/// Render the bottom controls bar with pf/DNS buttons and status.
fn render_controls_bar(f: &mut Frame, area: Rect, app: &TuiApp) {
    use ratatui::widgets::Paragraph;
    use ratatui::style::Stylize;

    let pf_status = if app.traffic.pf_enabled {
        "[p]f: ON ".green().to_string()
    } else {
        "[p]f: OFF ".red().to_string()
    };

    let dns_status = if app.traffic.dns_running {
        "[d]ns: ON ".green().to_string()
    } else {
        "[d]ns: OFF ".red().to_string()
    };

    let controls = Paragraph::new(format!(
        "{} {} | [Enter] select  [/] search  [Esc] clear search/filters",
        pf_status, dns_status
    ));

    f.render_widget(controls, area);
}
