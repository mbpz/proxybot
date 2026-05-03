//! Devices tab renderer.
//!
//! Shows connected devices and their stats.

use ratatui::{
    Frame, layout::{Rect, Constraint, Direction, Layout},
    widgets::{Block, Borders, Paragraph, Table, Row, Cell},
    style::{Style, Color},
    text::Line,
};
use crate::tui::TuiApp;
use crate::db::get_devices_internal;
use crate::adb::AdbDevice;

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

    // Fetch ADB devices if ADB is enabled
    let adb_devices: Vec<AdbDevice> = if app.devices.adb_enabled {
        crate::adb::list_devices()
    } else {
        Vec::new()
    };

    let total_devices = devices_list.len();
    let total_up: i64 = devices_list.iter().map(|d| d.upload_bytes).sum();
    let total_down: i64 = devices_list.iter().map(|d| d.download_bytes).sum();

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(65),
            Constraint::Percentage(35),
        ])
        .split(area);

    // Left: stats header + device table
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // stats header
            Constraint::Min(10),
        ])
        .split(chunks[0]);

    // Stats header
    let stats_text = if app.devices.editing_override {
        let current = app.devices.override_input.as_str();
        format!("Rule override: [{}] | Enter=confirm Esc=cancel", if current.is_empty() { "(none)" } else { current })
    } else if app.devices.adb_enabled {
        format!(
            " Devices: {} | Total Up: {} | Total Down: {} | [a] toggle ADB | j/k navigate [e] edit rule",
            total_devices,
            format_bytes(total_up),
            format_bytes(total_down),
        )
    } else {
        format!(
            " Devices: {} | Total Up: {} | Total Down: {} | j/k navigate [e] edit rule",
            total_devices,
            format_bytes(total_up),
            format_bytes(total_down),
        )
    };
    let stats = Paragraph::new(stats_text)
        .style(Style::new().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL).title("Devices"));
    f.render_widget(stats, left_chunks[0]);

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

            let rule_badge = dev.rule_override.as_deref().unwrap_or("-");
            Row::new(vec![
                Cell::from(dev.name.clone()),
                Cell::from(dev.mac_address.clone()),
                Cell::from(dev.last_seen_at.clone()),
                Cell::from(format_bytes(dev.upload_bytes)),
                Cell::from(format_bytes(dev.download_bytes)),
                Cell::from(rule_badge.to_string()),
            ])
            .style(row_style)
        })
        .collect();

    let table = Table::new(
        table_rows,
        [
            Constraint::Percentage(18),
            Constraint::Percentage(18),
            Constraint::Percentage(22),
            Constraint::Percentage(14),
            Constraint::Percentage(14),
            Constraint::Percentage(14),
        ],
    )
    .block(Block::default().borders(Borders::ALL).title("Device List"))
    .column_spacing(1);

    f.render_widget(table, left_chunks[1]);

    // Right: ASCII topology diagram
    render_topology(f, chunks[1], &devices_list, &adb_devices);
}

/// Render ASCII topology diagram showing device connections.
fn render_topology(f: &mut Frame, area: Rect, devices: &[crate::db::DeviceInfo], adb_devices: &[AdbDevice]) {
    use ratatui::widgets::Paragraph;

    let mut lines: Vec<Line> = Vec::new();

    // Server node
    lines.push(Line::raw("       ┌─────────────────────┐").style(Color::Cyan));
    lines.push(Line::raw("       │    ProxyBot Server   │").style(Color::Cyan));
    lines.push(Line::raw("       │    (This PC)         │").style(Color::Cyan));
    lines.push(Line::raw("       └──────────┬──────────┘").style(Color::Cyan));
    lines.push(Line::raw("                  │").style(Color::Cyan));

    if devices.is_empty() && adb_devices.is_empty() {
        lines.push(Line::raw("        (no devices connected)").style(Color::DarkGray));
        lines.push(Line::raw("".to_string()));
        lines.push(Line::raw(" Configure device gateway:").style(Color::Yellow));
        lines.push(Line::raw("  • Set proxy to this PC".to_string()));
        lines.push(Line::raw("  • Port: 8088".to_string()));
        lines.push(Line::raw("  • Install CA certificate".to_string()));
        lines.push(Line::raw("  • Or use USB with [a] toggle ADB".to_string()));
    } else {
        let max_display = devices.len().min(4);
        for (i, dev) in devices.iter().take(max_display).enumerate() {
            let name = dev.name.chars().take(15).collect::<String>();

            if i == 0 {
                lines.push(Line::raw("       └───┬───────────────┘".to_string()));
                lines.push(Line::raw(format!("           │ {}", dev.mac_address.chars().take(12).collect::<String>())));
                lines.push(Line::raw(format!("           └──[{}]", name)).style(Color::Green));
            } else {
                lines.push(Line::raw("           ┌─┴───────────────┐".to_string()));
                lines.push(Line::raw(format!("           │ {}", dev.mac_address.chars().take(12).collect::<String>())));
                lines.push(Line::raw(format!("           └──[{}]", name)).style(Color::Green));
            }
        }

        if devices.len() > max_display {
            lines.push(Line::raw(format!("           ... and {} more", devices.len() - max_display)).style(Color::DarkGray));
        }
    }

    // Show ADB devices section
    if !adb_devices.is_empty() {
        lines.push(Line::raw("".to_string()));
        lines.push(Line::raw("USB ADB Devices:").style(Color::Cyan));
        for dev in adb_devices.iter().take(3) {
            let model = dev.model.as_deref().unwrap_or("unknown");
            lines.push(Line::raw(format!("  • {} ({})", dev.serial.chars().take(12).collect::<String>(), model)).style(Color::Cyan));
        }
    }

    lines.push(Line::raw("".to_string()));
    lines.push(Line::raw("Legend:").style(Color::Yellow));
    lines.push(Line::raw(" [name] = device name".to_string()));
    lines.push(Line::raw(" (app)  = detected app".to_string()));
    lines.push(Line::raw(" MAC    = device address".to_string()));

    let content = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("Network Topology"))
        .style(Style::new().fg(Color::White));

    f.render_widget(content, area);
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