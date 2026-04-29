//! Traffic tab renderer.
//!
//! Shows intercepted HTTP/HTTPS requests in a scrollable list with:
//! - Filter bar (method, host, status, app_tag)
//! - Regex search bar
//! - Split pane: request list (top 60%) + detail panel (bottom 40%)

use ratatui::{Frame, layout::{Rect, Constraint, Layout, Direction}, widgets::{Block, Borders, List, Paragraph}, style::{Color, Stylize}, text::Line};

use crate::tui::{TuiApp, input::format_ts, input::fmt_duration};
use crate::db::RecentRequest;
use crate::proxy::InterceptedRequest;

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

/// Render animated skeleton loading rows.
fn render_skeleton(f: &mut Frame, area: Rect, app: &TuiApp) {
    use ratatui::style::Color;
    use ratatui::widgets::Paragraph;

    // Advance animation frame
    let frame = app.traffic.loading_frame;
    let spinner_chars = ['|', '/', '-', '\\'];
    let spinner = spinner_chars[frame % 4].to_string();

    let lines = vec![
        Line::raw(format!(" {} Capturing traffic...", spinner)),
        Line::raw("   ──────────────────────────────────────────"),
        Line::raw("   ████████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░"),
        Line::raw("   Waiting for requests from device..."),
        Line::raw(""),
        Line::raw("   Configure your device to use proxy port 8088"),
    ];

    let content = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("Intercepted Traffic"))
        .style(Color::Cyan);

    f.render_widget(content, area);
}
fn render_request_list(f: &mut Frame, area: Rect, app: &TuiApp) {
    use ratatui::widgets::List;

    let filtered: Vec<&RecentRequest> = app.traffic.filtered_requests();
    let selected = app.traffic.selected.min(filtered.len().saturating_sub(1));

    if filtered.is_empty() {
        // If proxy running but no requests, show skeleton loading
        if app.proxy_running.load(std::sync::atomic::Ordering::SeqCst) {
            render_skeleton(f, area, app);
        } else {
            let empty = Paragraph::new("  No requests captured. Start proxy to begin.")
                .block(Block::default().borders(Borders::ALL).title("Intercepted Traffic"));
            f.render_widget(empty, area);
        }
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

/// Render the detail panel with sub-tabs: Headers / Body / WS Frames.
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

    // Split area: tab bar (1 line) + content
    let chunks = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // sub-tab bar
            Constraint::Min(1),   // tab content
        ])
        .split(area);

    // Sub-tab bar
    let tabs = ["Headers", "Body", "WS Frames"];
    let ws_available = detail.is_websocket && detail.ws_frames.as_ref().map(|f| !f.is_empty()).unwrap_or(false);
    let active_tab = if app.traffic.detail_tab >= tabs.len() { 0 } else { app.traffic.detail_tab };

    let mut tab_line = String::new();
    for (i, tab) in tabs.iter().enumerate() {
        let ws_tab = tab == &"WS Frames" && !ws_available;
        if ws_tab {
            tab_line.push_str(&format!(" {} ", tab).dim().to_string());
        } else if i == active_tab {
            tab_line.push_str(&format!("[{}] ", tab).cyan().to_string());
        } else {
            tab_line.push_str(&format!(" {} ", tab).dim().to_string());
        }
    }
    tab_line.push_str(&format!(" [1/2/3] switch tab").dim().to_string());
    let tab_para = Paragraph::new(tab_line);
    f.render_widget(tab_para, chunks[0]);

    // Tab content
    match active_tab {
        0 => render_headers_tab(f, chunks[1], detail),
        1 => render_body_tab(f, chunks[1], detail),
        2 => render_ws_frames_tab(f, chunks[1], detail),
        _ => {}
    }
}

/// Render Headers sub-tab.
fn render_headers_tab(f: &mut Frame, area: Rect, detail: &InterceptedRequest) {
    use ratatui::style::Color;

    let mut lines: Vec<Line> = Vec::new();

    // Summary
    lines.push(Line::raw(format!(
        " {} {} {} -> {} ({})",
        detail.method, detail.scheme, detail.host, detail.path,
        detail.status.map(|s| s.to_string()).unwrap_or_else(|| "-".to_string())
    )).fg(Color::White).underlined());

    // Request headers
    lines.push(Line::raw("--- Request Headers ---").style(Color::Yellow));
    if detail.req_headers.is_empty() {
        lines.push(Line::raw("(empty)").fg(Color::DarkGray));
    } else {
        for (k, v) in &detail.req_headers {
            lines.push(Line::raw(format!("  {}: {}", k, v)).style(Color::White));
        }
    }

    // Response headers
    lines.push(Line::raw("--- Response Headers ---").style(Color::Green));
    if detail.resp_headers.is_empty() {
        lines.push(Line::raw("(empty)").fg(Color::DarkGray));
    } else {
        for (k, v) in &detail.resp_headers {
            lines.push(Line::raw(format!("  {}: {}", k, v)).style(Color::White));
        }
    }

    // App/Device info
    if let (Some(ref app_name), Some(ref device_name)) = (&detail.app_name, &detail.device_name) {
        lines.push(Line::raw(format!("App: {} | Device: {}", app_name, device_name)).style(Color::Magenta));
    }

    let para = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("Headers"));
    f.render_widget(para, area);
}

/// Render Body sub-tab with JSON formatting.
fn render_body_tab(f: &mut Frame, area: Rect, detail: &InterceptedRequest) {
    use ratatui::style::Color;

    let mut lines: Vec<Line> = Vec::new();

    // Request body
    lines.push(Line::raw("--- Request Body ---").style(Color::Yellow));
    if let Some(ref body) = detail.req_body {
        if body.is_empty() {
            lines.push(Line::raw("(empty)").fg(Color::DarkGray));
        } else {
            let display = if body.len() > 1000 { format!("{}...", &body[..1000]) } else { body.clone() };
            lines.push(Line::raw(format_json(&display)).style(Color::Cyan));
        }
    } else {
        lines.push(Line::raw("(none)").fg(Color::DarkGray));
    }

    lines.push(Line::raw(""));

    // Response body
    lines.push(Line::raw("--- Response Body ---").style(Color::Green));
    if let Some(ref body) = detail.resp_body {
        if body.is_empty() {
            lines.push(Line::raw("(empty)").fg(Color::DarkGray));
        } else {
            let display = if body.len() > 1000 { format!("{}...", &body[..1000]) } else { body.clone() };
            lines.push(Line::raw(format_json(&display)).style(Color::Cyan));
        }
    } else {
        lines.push(Line::raw("(none)").fg(Color::DarkGray));
    }

    let para = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("Body"))
        .scroll((0, 0));
    f.render_widget(para, area);
}

/// Render WebSocket Frames sub-tab.
fn render_ws_frames_tab(f: &mut Frame, area: Rect, detail: &InterceptedRequest) {
    use ratatui::style::Color;

    let mut lines: Vec<Line> = Vec::new();

    if !detail.is_websocket {
        lines.push(Line::raw("Not a WebSocket connection.").fg(Color::DarkGray));
    } else if let Some(ref frames) = detail.ws_frames {
        if frames.is_empty() {
            lines.push(Line::raw("No frames captured yet.").fg(Color::DarkGray));
        } else {
            lines.push(Line::raw(format!("{} WebSocket frames captured", frames.len())).style(Color::Cyan));
            lines.push(Line::raw("".to_string()));
            for frame in frames.iter().take(50) {
                let direction_color = if frame.direction == "in" { Color::Green } else { Color::Yellow };
                let dir_marker = if frame.direction == "in" { "◄" } else { "►" };
                let line_text = format!(
                    "{} [{}] {} ({} bytes)",
                    dir_marker,
                    frame.timestamp.chars().take(12).collect::<String>(),
                    frame.payload.chars().take(60).collect::<String>(),
                    frame.size
                );
                lines.push(Line::raw(line_text).style(direction_color));
            }
            if frames.len() > 50 {
                lines.push(Line::raw(format!("... and {} more frames", frames.len() - 50)).fg(Color::DarkGray));
            }
        }
    } else {
        lines.push(Line::raw("No frames captured.").fg(Color::DarkGray));
    }

    let para = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("WS Frames"));
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
        "{} {} | [Enter] select  [/] search  [1/2/3] detail tab  [Esc] clear filters",
        pf_status, dns_status
    ));

    f.render_widget(controls, area);
}
