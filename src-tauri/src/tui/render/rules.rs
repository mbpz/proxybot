//! Rules tab renderer.
//!
//! Shows the active rule set and allows navigation.
//! Supports add/edit/delete via modal overlay.

use ratatui::{
    Frame, layout::{Rect, Constraint, Direction, Layout, Alignment},
    widgets::{Block, Borders, Paragraph, Table, Row, Cell, BorderType},
    style::{Style, Color},
    text::{Line, Span},
};
use crate::tui::TuiApp;
use crate::rules::{RuleAction, RulePattern};

/// Render the Rules tab with a table of rules and optional modal editor.
pub fn render(f: &mut Frame, area: Rect, app: &TuiApp) {
    // Fetch rules from engine
    let rules_list = app.rules_engine.get_rules();
    let watcher_active = app.rules.watcher_active;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // header / hot-reload status
            Constraint::Min(10),
        ])
        .split(area);

    // Header with hot-reload status
    let watcher_str = if watcher_active { "ACTIVE" } else { "INACTIVE" };
    let watcher_color = if watcher_active { Color::Green } else { Color::Red };

    let header_text = Line::from(vec![
        Span::raw(format!(" Rules ({} rules) | Hot-reload: ", rules_list.len())),
        Span::styled(watcher_str, Style::new().fg(watcher_color)),
        Span::raw(" | [a]dd [e]dit [d]elete | j/k navigate"),
    ]);
    let header = Paragraph::new(header_text)
        .style(Style::new().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL).title("Rules"));
    f.render_widget(header, chunks[0]);

    // Build table rows
    let table_rows: Vec<Row> = rules_list
        .iter()
        .enumerate()
        .map(|(i, rule)| {
            let action_str = match &rule.action {
                RuleAction::Direct => "DIRECT".to_string(),
                RuleAction::Proxy => "PROXY".to_string(),
                RuleAction::Reject => "REJECT".to_string(),
                RuleAction::MapRemote(t) => format!("MAPREMOTE:{}", t),
                RuleAction::MapLocal(t) => format!("MAPLOCAL:{}", t),
                RuleAction::Breakpoint(t) => format!("BREAKPOINT:{:?}", t),
            };
            let action_color = match &rule.action {
                RuleAction::Direct => Color::Green,
                RuleAction::Proxy => Color::Yellow,
                RuleAction::Reject => Color::Red,
                RuleAction::MapRemote(_) => Color::Blue,
                RuleAction::MapLocal(_) => Color::Cyan,
                RuleAction::Breakpoint(_) => Color::Magenta,
            };
            let pattern_str = match rule.pattern {
                RulePattern::Domain => "DOMAIN",
                RulePattern::DomainSuffix => "DOMAIN-SUFFIX",
                RulePattern::DomainKeyword => "DOMAIN-KEYWORD",
                RulePattern::IpCidr => "IP-CIDR",
                RulePattern::Geoip => "GEOIP",
                RulePattern::RuleSet => "RULE-SET",
            };
            let selected = i == app.rules.selected;
            let row_style = if selected {
                Style::new().bg(Color::Blue).fg(Color::White)
            } else {
                Style::new()
            };

            Row::new(vec![
                Cell::from(rule.value.clone()),
                Cell::from(pattern_str),
                Cell::from(action_str).style(Style::new().fg(action_color)),
            ])
            .style(row_style)
        })
        .collect();

    let table = Table::new(
        table_rows,
        [
            Constraint::Percentage(50),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ],
    )
    .block(Block::default().borders(Borders::ALL).title("Rule List"))
    .column_spacing(1);

    f.render_widget(table, chunks[1]);

    // Modal overlay if open
    if app.rules.modal_open {
        render_rule_modal(f, area, app);
    }
}

/// Render the inline rule editor modal overlay.
fn render_rule_modal(f: &mut Frame, area: Rect, app: &TuiApp) {
    let (name, pattern, action) = &app.rules.edit_buffer;
    let modal_width = 50.min(area.width.saturating_sub(10));
    let modal_height = 10.min(area.height.saturating_sub(6));
    let x = (area.width.saturating_sub(modal_width)) / 2;
    let y = (area.height.saturating_sub(modal_height)) / 2;

    let modal_area = Rect::new(x, y, modal_width, modal_height);

    let border_style = Style::new().fg(Color::Cyan).bg(Color::Black);
    let block = Block::default()
        .border_type(BorderType::Double)
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(if app.rules.modal_mode == "add" { " Add Rule " } else { " Edit Rule " });

    // Build modal content lines
    let mode_label = if app.rules.modal_mode == "add" { "ADD RULE" } else { "EDIT RULE" };
    let lines = vec![
        format!("  {}  (press s to save, Esc/q to cancel)", mode_label),
        format!(""),
        format!("  Pattern: {}  (DOMAIN, DOMAIN-SUFFIX, DOMAIN-KEYWORD, IP-CIDR)", pattern),
        format!("  Value:   {}", name),
        format!("  Action:  {}  (DIRECT, PROXY, REJECT)", action),
        format!(""),
        format!("  Use Tab to cycle: Pattern -> Value -> Action"),
    ];

    let text = lines.join("\n");
    let para = Paragraph::new(text)
        .block(block)
        .alignment(Alignment::Left);

    f.render_widget(para, modal_area);
}