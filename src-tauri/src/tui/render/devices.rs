//! Devices tab renderer.
//!
//! Shows connected devices and their stats.

use ratatui::{
    Frame, layout::{Rect, Constraint, Direction, Layout},
    widgets::{Block, Borders, Paragraph, Table, Row, Cell},
    style::{Style, Color},
};
use crate::tui::TuiApp;
use crate::db::get_devices_internal;

/// Render the Devices tab.
pub fn render(f: &mut Frame, area: Rect, app: &TuiApp) {
    // Fetch devices from DB
    let devices_list = {
        if let Ok(conn) = app.db_state.conn.lock() {
            get_devices_internal(&conn).unwrap_or_default()
        } else {
            Vec::new()
        }
    };

    let total_devices = devices_list.len();
    let total_up: i64 = devices_list.iter().map(|d| d.upload_bytes).sum();
    let total_down: i64 = devices_list.iter().map(|d| d.download_bytes).sum();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // stats header
            Constraint::Min(10),
        ])
        .split(area);

    // Stats header
    let stats_text = format!(
        " Devices: {} | Total Up: {} | Total Down: {} | j/k navigate",
        total_devices,
        format_bytes(total_up),
        format_bytes(total_down),
    );
    let stats = Paragraph::new(stats_text)
        .style(Style::new().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL).title("Devices"));
    f.render_widget(stats, chunks[0]);

    // Device table
    let table_rows: Vec<Row> = devices_list
        .iter()
        .enumerate()
        .map(|(i, dev)| {
            let selected = i == app.devices.selected;
            let row_style = if selected {
                Style::new().bg(Color::Blue).fg(Color::White)
            } else {
                Style::new()
            };

            Row::new(vec![
                Cell::from(dev.name.clone()),
                Cell::from(dev.mac_address.clone()),
                Cell::from(dev.last_seen_at.clone()),
                Cell::from(format_bytes(dev.upload_bytes)),
                Cell::from(format_bytes(dev.download_bytes)),
            ])
            .style(row_style)
        })
        .collect();

    let table = Table::new(
        table_rows,
        [
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(25),
            Constraint::Percentage(17),
            Constraint::Percentage(18),
        ],
    )
    .block(Block::default().borders(Borders::ALL).title("Device List"))
    .column_spacing(1);

    f.render_widget(table, chunks[1]);
}

/// Format bytes into human-readable string.
fn format_bytes(b: i64) -> String {
    if b < 1024 {
        format!("{} B", b)
    } else if b < 1024 * 1024 {
        format!("{:.1} KB", b as f64 / 1024.0)
    } else if b < 1024 * 1024 * 1024 {
        format!("{:.1} MB", b as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", b as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}