//! Replay tab renderer.
//!
//! Shows replay targets with status, controls for start/stop/replay,
//! HAR export, and diff view panel.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

use crate::tui::i18n::{t as tr, I18nKey as K};
use crate::tui::TuiApp;

/// Render the Replay tab with targets list, status, and diff view.
pub fn render(f: &mut Frame, area: Rect, app: &TuiApp) {
    // Layout: targets table (40%), diff panel (40%), controls (20%)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(40), // targets list
            Constraint::Percentage(40), // diff output or status
            Constraint::Length(1),      // controls bar
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
        let replay_targets_title = tr(K::ReplayTargets);
        let empty = Paragraph::new(format!("  {}", tr(K::ReplayEmpty))).block(
            Block::default()
                .borders(Borders::ALL)
                .title(replay_targets_title.as_str()),
        );
        f.render_widget(empty, area);
        return;
    }

    let selected = app.replay.selected.min(targets.len().saturating_sub(1));

    let rows: Vec<Row> = targets
        .iter()
        .enumerate()
        .map(|(idx, target)| {
            let host_cell = Cell::from(target.host.chars().take(30).collect::<String>());
            let count_cell = Cell::from(format!(
                "{} {}",
                target.request_count,
                tr(K::ReplayRequests)
            ));
            let path_cell = Cell::from(format!("{} {}", target.path_count, tr(K::ReplayPaths)));

            // Status shown based on running state
            let status_text = tr(K::ReplayStatusIdle).dim();
            let status_cell = Cell::from(status_text);

            let row = Row::new(vec![host_cell, count_cell, path_cell, status_cell]).style(
                if idx == selected {
                    Style::default().bg(Color::DarkGray)
                } else {
                    Style::default()
                },
            );

            row
        })
        .collect();

    let widths = [
        Constraint::Length(30), // host
        Constraint::Length(15), // request count
        Constraint::Length(15), // path count
        Constraint::Length(10), // status
    ];
    let replay_targets_title = tr(K::ReplayTargets);
    let table = Table::new(rows, widths)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(replay_targets_title.as_str()),
        )
        .highlight_style(Color::Cyan);

    let mut table_state = ratatui::widgets::TableState::default().with_selected(Some(selected));
    f.render_stateful_widget(table, area, &mut table_state);
}

/// Render diff output panel or status message.
fn render_diff_or_status(f: &mut Frame, area: Rect, app: &TuiApp) {
    if let Some(ref diff) = app.replay.diff_output {
        // Show diff view
        let diff_lines: Vec<Line> = diff
            .lines()
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

        let diff_view_title = tr(K::ReplayDiffView);
        let para = Paragraph::new(diff_lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(diff_view_title.as_str()),
            )
            .scroll((0, 0));

        f.render_widget(para, area);
    } else {
        // Show status
        let status_text = if let Some(ref export_status) = app.replay.har_export_status {
            format!(" {}: {}", tr(K::ReplayStatus), export_status)
        } else {
            tr(K::ReplaySelectTarget)
        };

        let replay_status_title = tr(K::ReplayStatus);
        let para = Paragraph::new(status_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(replay_status_title.as_str()),
            )
            .style(Color::White);

        f.render_widget(para, area);
    }
}

/// Render bottom controls bar.
fn render_controls(f: &mut Frame, area: Rect, _app: &TuiApp) {
    let controls_text = format!(
        "{}  {}  {}  {}  {}",
        tr(K::ReplayNavigate),
        tr(K::ReplayStartStop),
        tr(K::ReplayExport),
        tr(K::ReplayExportHar),
        tr(K::ReplayShowDiff)
    );
    let controls = Paragraph::new(controls_text);
    f.render_widget(controls, area);
}
