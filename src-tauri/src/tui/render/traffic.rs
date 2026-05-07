//! Traffic tab renderer.

use ratatui::{Frame, layout::{Rect, Constraint, Layout, Direction}, widgets::{Block, Borders, Paragraph}, style::{Stylize, Style}, text::Line};

use crate::tui::{TuiApp, input::format_ts, input::fmt_duration, FilterMode};
use crate::tui::i18n::{I18nKey as K, t};
use crate::db::RecentRequest;
use crate::proxy::InterceptedRequest;

/// Render the Traffic tab with filters, split pane, and controls.
pub fn render(f: &mut Frame, area: Rect, app: &TuiApp) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(1),
        ])
        .split(area);

    render_filter_bar(f, chunks[0], app);
    render_content(f, chunks[1], app);
    render_controls_bar(f, chunks[2], app);
}

/// Render the filter bar.
fn render_filter_bar(f: &mut Frame, area: Rect, app: &TuiApp) {
    use ratatui::style::Color;
    use ratatui::widgets::Paragraph;

    let traffic = &app.traffic;

    if let Some(mode) = traffic.filter_mode {
        let label = match mode {
            FilterMode::Method => t(K::TrafficFilterMethod),
            FilterMode::Host => t(K::TrafficFilterHost),
            FilterMode::Status => t(K::TrafficFilterStatus),
            FilterMode::AppTag => t(K::TrafficFilterAppTag),
        };
        let prompt = format!("{} {} [Enter=confirm, Esc=cancel]", label, traffic.filter_input);
        let para = Paragraph::new(prompt.yellow().to_string())
            .block(Block::default().borders(Borders::ALL).title("Filter Input"))
            .style(Color::Yellow);
        f.render_widget(para, area);
        return;
    }

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
        " {}:[{}] {}:[{:<15}] {}:[{}] {}:[{:<10}] {} [m]{} [f]{} [o]{} [a]{}",
        t(K::TrafficFilterMethod), method_str.yellow(),
        t(K::TrafficFilterHost), host_str.chars().take(15).collect::<String>().yellow(),
        t(K::TrafficFilterStatus), status_str.green(),
        t(K::TrafficFilterAppTag), app_tag_str.chars().take(10).collect::<String>().cyan(),
        search_str,
        t(K::TrafficFilterMethod), t(K::TrafficFilterHost), t(K::TrafficFilterStatus), t(K::TrafficFilterAppTag),
    );

    let title = t(K::TrafficTitle);
    let para = Paragraph::new(filter_line)
        .block(Block::default().borders(Borders::ALL).title(title.as_str()))
        .style(Color::White);

    f.render_widget(para, area);
}

fn render_content(f: &mut Frame, area: Rect, app: &TuiApp) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(60),
            Constraint::Percentage(40),
        ])
        .split(area);

    render_request_list(f, chunks[0], app);
    if app.traffic.breakpoint.mode != crate::tui::BreakpointMode::None {
        render_breakpoint_editor(f, chunks[1], app);
    } else {
        render_detail_panel(f, chunks[1], app);
    }
}

fn render_skeleton(f: &mut Frame, area: Rect, app: &TuiApp) {
    use ratatui::style::Color;
    use ratatui::widgets::Paragraph;

    let frame = app.traffic.loading_frame;
    let spinner_chars = ['|', '/', '-', '\\'];
    let spinner = spinner_chars[frame % 4].to_string();

    let title = t(K::TrafficTitle);
    let lines = vec![
        Line::raw(format!(" {} {}", spinner, t(K::TrafficCapturing))),
        Line::raw("   ──────────────────────────────────────────"),
        Line::raw("   ████████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░"),
        Line::raw(format!("   {}", t(K::TrafficWaiting))),
        Line::raw(""),
        Line::raw(format!("   {}", t(K::TrafficConfigurePort))),
    ];

    let content = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(title.as_str()))
        .style(Color::Cyan);

    f.render_widget(content, area);
}

fn render_request_list(f: &mut Frame, area: Rect, app: &TuiApp) {
    use ratatui::style::Color;
    use ratatui::widgets::List;

    let filtered: Vec<&RecentRequest> = app.traffic.filtered_requests();
    let selected = app.traffic.selected.min(filtered.len().saturating_sub(1));
    let title = t(K::TrafficTitle);

    if filtered.is_empty() {
        if app.proxy_running.load(std::sync::atomic::Ordering::SeqCst) {
            render_skeleton(f, area, app);
        } else {
            let empty = Paragraph::new(format!("  {}", t(K::TrafficNoRequests)))
                .block(Block::default().borders(Borders::ALL).title(title.as_str()));
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

        let mut list_state = ratatui::widgets::ListState::default().with_selected(Some(selected));
        f.render_stateful_widget(
            List::new(items)
                .block(Block::default().borders(Borders::ALL).title(title.as_str()))
                .highlight_style(Color::Cyan),
            area,
            &mut list_state,
        );
    }
}

fn render_detail_panel(f: &mut Frame, area: Rect, app: &TuiApp) {
    let filtered: Vec<&RecentRequest> = app.traffic.filtered_requests();

    if filtered.is_empty() || app.traffic.detail_request.is_none() {
        let hint = format!(" {}", t(K::TrafficNoSelected));
        let para = Paragraph::new(hint.dim())
            .block(Block::default().borders(Borders::ALL).title("Request Detail"));
        f.render_widget(para, area);
        return;
    }

    let detail = app.traffic.detail_request.as_ref().unwrap();

    let chunks = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .split(area);

    let tabs = [t(K::TrafficSubTabsHeaders), t(K::TrafficSubTabsBody), t(K::TrafficSubTabsWsFrames)];
    let ws_available = detail.is_websocket && detail.ws_frames.as_ref().map(|f| !f.is_empty()).unwrap_or(false);
    let active_tab = if app.traffic.detail_tab >= tabs.len() { 0 } else { app.traffic.detail_tab };
    let ws_frames_label = t(K::TrafficSubTabsWsFrames);

    let mut tab_line = String::new();
    for (i, tab) in tabs.iter().enumerate() {
        let ws_tab = tab == &ws_frames_label && !ws_available;
        if ws_tab {
            tab_line.push_str(&format!(" {} ", tab).dim().to_string());
        } else if i == active_tab {
            tab_line.push_str(&format!("[{}] ", tab).cyan().to_string());
        } else {
            tab_line.push_str(&format!(" {} ", tab).dim().to_string());
        }
    }
    tab_line.push_str(&format!(" {}", t(K::TrafficSubTabsSwitchTab)).dim().to_string());
    let tab_para = Paragraph::new(tab_line);
    f.render_widget(tab_para, chunks[0]);

    match active_tab {
        0 => render_headers_tab(f, chunks[1], detail),
        1 => render_body_tab(f, chunks[1], detail),
        2 => render_ws_frames_tab(f, chunks[1], detail),
        _ => {}
    }
}

fn render_headers_tab(f: &mut Frame, area: Rect, detail: &InterceptedRequest) {
    use ratatui::style::Color;

    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::raw(format!(
        " {} {} {} -> {} ({})",
        detail.method, detail.scheme, detail.host, detail.path,
        detail.status.map(|s| s.to_string()).unwrap_or_else(|| "-".to_string())
    )).fg(Color::White).underlined());

    lines.push(Line::raw(t(K::TrafficRequestHeaders).to_string()).style(Color::Yellow));
    if detail.req_headers.is_empty() {
        lines.push(Line::raw(t(K::TrafficEmptyBody).to_string()).fg(Color::DarkGray));
    } else {
        for (k, v) in &detail.req_headers {
            lines.push(Line::raw(format!("  {}: {}", k, v)).style(Color::White));
        }
    }

    lines.push(Line::raw(t(K::TrafficResponseHeaders).to_string()).style(Color::Green));
    if detail.resp_headers.is_empty() {
        lines.push(Line::raw(t(K::TrafficEmptyBody).to_string()).fg(Color::DarkGray));
    } else {
        for (k, v) in &detail.resp_headers {
            lines.push(Line::raw(format!("  {}: {}", k, v)).style(Color::White));
        }
    }

    if let (Some(ref app_name), Some(ref device_name)) = (&detail.app_name, &detail.device_name) {
        lines.push(Line::raw(format!("App: {} | Device: {}", app_name, device_name)).style(Color::Magenta));
    }

    let para = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("Headers"));
    f.render_widget(para, area);
}

fn render_body_tab(f: &mut Frame, area: Rect, detail: &InterceptedRequest) {
    use ratatui::style::Color;

    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::raw(t(K::TrafficRequestBody).to_string()).style(Color::Yellow));
    if let Some(ref body) = detail.req_body {
        if body.is_empty() {
            lines.push(Line::raw(t(K::TrafficEmptyBody).to_string()).fg(Color::DarkGray));
        } else {
            let display = if body.len() > 1000 { format!("{}...", &body[..1000]) } else { body.clone() };
            lines.push(Line::raw(format_json(&display)).style(Color::Cyan));
        }
    } else {
        lines.push(Line::raw(t(K::TrafficNoBody).to_string()).fg(Color::DarkGray));
    }

    lines.push(Line::raw(""));

    lines.push(Line::raw(t(K::TrafficResponseBody).to_string()).style(Color::Green));
    if let Some(ref body) = detail.resp_body {
        if body.is_empty() {
            lines.push(Line::raw(t(K::TrafficEmptyBody).to_string()).fg(Color::DarkGray));
        } else {
            let display = if body.len() > 1000 { format!("{}...", &body[..1000]) } else { body.clone() };
            lines.push(Line::raw(format_json(&display)).style(Color::Cyan));
        }
    } else {
        lines.push(Line::raw(t(K::TrafficNoBody).to_string()).fg(Color::DarkGray));
    }

    let para = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("Body"))
        .scroll((0, 0));
    f.render_widget(para, area);
}

fn render_ws_frames_tab(f: &mut Frame, area: Rect, detail: &InterceptedRequest) {
    use ratatui::style::Color;

    let mut lines: Vec<Line> = Vec::new();

    if !detail.is_websocket {
        lines.push(Line::raw(t(K::TrafficNotWs).to_string()).fg(Color::DarkGray));
    } else if let Some(ref frames) = detail.ws_frames {
        if frames.is_empty() {
            lines.push(Line::raw(t(K::TrafficNoFrames).to_string()).fg(Color::DarkGray));
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
        lines.push(Line::raw(t(K::TrafficNoFrames).to_string()).fg(Color::DarkGray));
    }

    let para = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("WS Frames"));
    f.render_widget(para, area);
}

fn render_breakpoint_editor(f: &mut Frame, area: Rect, app: &TuiApp) {
    use ratatui::layout::Alignment;
    use ratatui::widgets::{Block, Borders, Paragraph};
    use ratatui::style::Color;
    use crate::tui::{BreakpointMode, BreakpointEditMode, BreakpointField};

    let bp = &app.traffic.breakpoint;
    let req = match &bp.current_edit {
        Some(r) => r,
        None => return,
    };

    let modal_width = 60.min(area.width.saturating_sub(4));
    let modal_height = 20.min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(modal_width)) / 2;
    let y = (area.height.saturating_sub(modal_height)) / 2;
    let modal_area = Rect::new(x, y, modal_width, modal_height);

    let is_editing = !matches!(bp.edit_mode, BreakpointEditMode::None);

    let mode_label = match bp.mode {
        BreakpointMode::RequestPaused => t(K::TrafficBreakpointRequest),
        BreakpointMode::ResponsePaused => t(K::TrafficBreakpointResponse),
        _ => return,
    };

    let edit_indicator = if is_editing { "[EDIT]" } else { "" };
    let help_text = if is_editing { t(K::TrafficBreakpointNavHelp) } else { t(K::TrafficBreakpointEditHelp) };

    let mut lines: Vec<String> = vec![
        format!("  {} {} — {}", mode_label, edit_indicator, help_text),
        String::new(),
    ];

    let method_str = if is_editing && matches!(bp.selected_field, BreakpointField::Method) {
        format!("> Method:   [{}]", bp.method_input)
    } else {
        format!("  Method:   {}", req.method)
    };
    lines.push(method_str);

    let url_display = if is_editing && matches!(bp.selected_field, BreakpointField::Url) {
        format!("> URL:    [{}]", bp.url_input)
    } else {
        let host_trunc = req.host.chars().take(30).collect::<String>();
        let path_trunc = req.path.chars().take(40).collect::<String>();
        format!("  URL:    {}://{}{}", req.scheme, host_trunc, path_trunc)
    };
    lines.push(url_display);

    lines.push(String::new());
    lines.push(format!("  Headers: ({})", req.req_headers.len()));
    for (i, (k, v)) in req.req_headers.iter().enumerate() {
        let is_selected = is_editing && matches!(bp.selected_field, BreakpointField::Headers);
        let is_editing_this = is_editing && bp.editing_header_index == Some(i);
        let prefix = if is_selected { "> " } else { "  " };
        let editing_indicator = if is_editing_this { "[EDITING]" } else { "" };
        let header_val = v.chars().take(40).collect::<String>();
        let line = format!("{}{}{}: {}", prefix, editing_indicator, k, header_val);
        lines.push(line);
    }

    lines.push(String::new());
    let body_preview = req.req_body.as_ref()
        .map(|s| s.chars().take(60).collect::<String>())
        .unwrap_or_else(|| t(K::TrafficEmptyBody));
    let body_str = if is_editing && matches!(bp.selected_field, BreakpointField::Body) {
        format!("> Body:   [{}...]", bp.body_input.chars().take(30).collect::<String>())
    } else {
        format!("  Body:   {}", body_preview)
    };
    lines.push(body_str);

    if let Some(idx) = bp.editing_header_index {
        let header_input_display = bp.header_input.chars().take(40).collect::<String>();
        let header_line = format!("\n  [Editing header {}: {}]  [Enter] confirm  [Esc] cancel", idx, header_input_display);
        lines.push(header_line);
    }

    let content = Paragraph::new(lines.join("\n"))
        .block(Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ", mode_label))
            .border_style(Style::new().fg(Color::Cyan)))
        .alignment(Alignment::Left);

    f.render_widget(content, modal_area);
}

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

/// Render the bottom controls bar.
fn render_controls_bar(f: &mut Frame, area: Rect, app: &TuiApp) {
    use ratatui::widgets::Paragraph;
    use ratatui::style::Stylize;

    let pf_status = if app.traffic.pf_enabled {
        format!("[p]f: ON ").green()
    } else {
        format!("[p]f: OFF ").red()
    };

    let dns_status = if app.traffic.dns_running {
        format!("[d]ns: ON ").green()
    } else {
        format!("[d]ns: OFF ").red()
    };

    let update_hint = if let Some(ref ver) = *app.update_available.lock().unwrap() {
        format!(" [Update: {} available] [u] download", ver).yellow().to_string()
    } else {
        String::new()
    };

    let controls = Paragraph::new(format!(
        "{} {}{} | {}",
        pf_status, dns_status, update_hint, t(K::TrafficFilterHint)
    ));

    f.render_widget(controls, area);
}