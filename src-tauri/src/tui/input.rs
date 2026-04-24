//! Async key input handler for the TUI.
//!
//! Provides keyboard handling with tab navigation support.
//! Uses crossterm's event::poll for non-blocking input.

use crossterm::event::{self, KeyCode, KeyEventKind};
use std::time::Duration;

/// Input action returned by the input handler.
#[derive(Debug)]
pub enum InputAction {
    /// Quit the application.
    Quit,
    /// Switch to next tab.
    NextTab,
    /// Switch to previous tab.
    PrevTab,
    /// Start proxy.
    StartProxy,
    /// Stop proxy.
    StopProxy,
    /// Clear requests / data.
    Clear,
    /// Navigate up.
    Up,
    /// Navigate down.
    Down,
    /// Enter / select.
    Enter,
    /// No action.
    None,
}

/// Handle a single key event and return the appropriate action.
pub fn handle_key_event(key: &event::KeyEvent) -> InputAction {
    if key.kind != KeyEventKind::Press {
        return InputAction::None;
    }

    match key.code {
        // Navigation
        KeyCode::Tab => InputAction::NextTab,
        KeyCode::BackTab => InputAction::PrevTab,
        KeyCode::Left => InputAction::PrevTab,
        KeyCode::Right => InputAction::NextTab,

        // Tab switching with Shift+Tab
        KeyCode::Char('h') => InputAction::PrevTab,
        KeyCode::Char('l') => InputAction::NextTab,

        // Quit
        KeyCode::Char('q') | KeyCode::Esc => InputAction::Quit,

        // Proxy control
        KeyCode::Char('r') => InputAction::StartProxy,
        KeyCode::Char('s') => InputAction::StopProxy,

        // Clear
        KeyCode::Char('c') => InputAction::Clear,

        // List navigation
        KeyCode::Up | KeyCode::Char('k') => InputAction::Up,
        KeyCode::Down | KeyCode::Char('j') => InputAction::Down,
        KeyCode::Enter => InputAction::Enter,

        _ => InputAction::None,
    }
}

/// Poll for input with the given timeout.
/// Returns the action for the first key event received, or None if timeout elapsed.
pub fn poll_input(timeout: Duration) -> Option<InputAction> {
    if event::poll(timeout).ok()? {
        if let event::Event::Key(key) = event::read().ok()? {
            let action = handle_key_event(&key);
            if matches!(action, InputAction::None) {
                return None;
            }
            return Some(action);
        }
    }
    None
}

/// Format timestamp for display (HH:MM:SS.ms).
pub fn format_ts(ts: &str) -> String {
    if ts.contains('.') {
        if let Ok(secs) = ts.split('.').next().unwrap_or("0").parse::<u64>() {
            let hours = (secs / 3600) % 24;
            let mins = (secs % 3600) / 60;
            let secs = secs % 60;
            return format!("{:02}:{:02}:{:02}", hours, mins, secs);
        }
    }
    if ts.len() >= 19 {
        return ts[11..19].to_string();
    }
    ts.chars().take(12).collect()
}

/// Format duration in ms.
pub fn fmt_duration(ms: Option<i64>) -> String {
    match ms {
        Some(v) if v < 1000 => format!("{}ms", v),
        Some(v) => format!("{:.1}s", v as f64 / 1000.0),
        None => "-".to_string(),
    }
}