//! Replay tab renderer.
//!
//! Shows replay targets with status, controls for start/stop/replay,
//! HAR export, and diff view panel.

use ratatui::{
    Frame,
    layout::{Rect, Constraint, Layout, Direction},
    widgets::{Block, Borders, Paragraph, Table, Row, Cell},
    style::{Color, Stylize, Style},
    text::Line,
};

use crate::tui::TuiApp;

/// Render the Replay tab with targets list, status, and diff view.
pub fn render(f: &mut Frame, area: Rect, app: &TuiApp) {
    // Layout: targets table (40%), diff panel (40%), controls (20%)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(40),  // targets list
            Constraint::Percentage(40),  // diff output or status
            Constraint::Length(1),        // controls bar
        ])
        .split(area);

    render_targets(f, chunks[0], app);
    render_diff_or_status(f, chunks[1], app);
    render_controls(f, chunks[2], app);
}

/// Render the targets table with status indicators.
fn render_targets(f: &mut Frame, area: Rect, app: &TuiApp) {
    let targets = &app.replay.targets_list;

    if targets.is_empty() {
        let empty = Paragraph::new("  No replay targets. Targets appear after traffic is recorded.")
            .block(Block::default().borders(Borders::ALL).title("Replay Targets"));
        f.render_widget(empty, area);
        return;
    }

    let selected = app.replay.selected.min(targets.len().saturating_sub(1));

    let rows: Vec<Row> = targets.iter().enumerate().map(|(idx, target)| {
        let host_cell = Cell::from(target.host.chars().take(30).collect::<String>());
        let count_cell = Cell::from(format!("{} requests", target.request_count));
        let path_cell = Cell::from(format!("{} paths", target.path_count));

        // Status shown based on running state
        let status_text = "idle".dim();
        let status_cell = Cell::from(status_text);

        let row = Row::new(vec![host_cell, count_cell, path_cell, status_cell])
            .style(if idx == selected {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            });

        row
    }).collect();

    let widths = [
        Constraint::Length(30),  // host
        Constraint::Length(15),  // request count
        Constraint::Length(15),  // path count
        Constraint::Length(10),  // status
    ];
    let table = Table::new(rows, widths)
        .block(Block::default().borders(Borders::ALL).title("Replay Targets"))
        .highlight_style(Color::Cyan);

    let mut table_state = ratatui::widgets::TableState::default().with_selected(Some(selected));
    f.render_stateful_widget(table, area, &mut table_state);
}

/// Render diff output panel or status message.
fn render_diff_or_status(f: &mut Frame, area: Rect, app: &TuiApp) {
    if let Some(ref diff) = app.replay.diff_output {
        // Show diff view
        let diff_lines: Vec<Line> = diff.lines()
            .map(|line| {
                if line.starts_with('+') {
                    Line::raw(line).style(Color::Green)
                } else if line.starts_with('-') {
                    Line::raw(line).style(Color::Red)
                } else {
                    Line::raw(line)
                }
            })
            .collect();

        let para = Paragraph::new(diff_lines)
            .block(Block::default().borders(Borders::ALL).title("Diff View"))
            .scroll((0, 0));

        f.render_widget(para, area);
    } else {
        // Show status
        let status_text = if let Some(ref export_status) = app.replay.har_export_status {
            format!(" HAR export: {}", export_status)
        } else {
            " Select a target and press [s] to start replay, [x] to stop, [e] to export HAR".to_string()
        };

        let para = Paragraph::new(status_text)
            .block(Block::default().borders(Borders::ALL).title("Replay Status"))
            .style(Color::White);

        f.render_widget(para, area);
    }
}

/// Render bottom controls bar.
fn render_controls(f: &mut Frame, area: Rect, _app: &TuiApp) {
    let controls = Paragraph::new("[j/k] navigate  [s] start  [x] stop  [e] export HAR  [d] show diff");
    f.render_widget(controls, area);
}