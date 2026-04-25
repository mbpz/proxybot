//! Async key input handler for the TUI.
//!
//! Provides keyboard handling with tab navigation support.
//! Uses crossterm's event::poll for non-blocking input.

use crossterm::event::{self, KeyCode, KeyEventKind};
use std::time::Duration;

use super::Tab;

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
    /// Toggle pf.
    TogglePf,
    /// Toggle DNS server.
    ToggleDns,
    /// Focus search.
    FocusSearch,
    /// Clear search / filters.
    ClearSearch,
    /// Add a new rule (Rules tab).
    AddRule,
    /// Edit the selected rule (Rules tab).
    EditRule,
    /// Delete the selected rule (Rules tab).
    DeleteRule,
    /// Save the rule being edited (modal).
    SaveRule,
    /// Cancel/close the modal without saving.
    CancelModal,
    /// Regenerate CA certificate (Certs tab).
    RegenerateCert,
    /// Export CA certificate (Certs tab).
    ExportCert,
    /// Toggle blocklist (DNS tab).
    ToggleBlocklist,
    /// Cycle upstream DNS type (DNS tab).
    CycleUpstream,
    /// Acknowledge the selected alert (Alerts tab).
    AckAlert,
    /// Clear all acknowledged alerts (Alerts tab).
    ClearAlerts,
    /// Start replay for selected target (Replay tab).
    StartReplay,
    /// Stop replay (Replay tab).
    StopReplay,
    /// Export traffic to HAR file (Replay tab).
    ExportHar,
    /// Show diff for replay results (Replay tab).
    ShowDiff,
    /// No action.
    None,
}

/// Handle a single key event and return the appropriate action.
pub fn handle_key_event(key: &event::KeyEvent, current_tab: Tab) -> InputAction {
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
        KeyCode::Char('r') if current_tab != Tab::Certs => InputAction::StartProxy,
        KeyCode::Char('S') => InputAction::StopProxy,

        // Clear (only when not on Alerts tab - 'c' is used for ClearAlerts there)
        KeyCode::Char('c') if current_tab != Tab::Alerts => InputAction::Clear,

        // pf/DNS controls
        KeyCode::Char('p') => InputAction::TogglePf,
        KeyCode::Char('n') => InputAction::ToggleDns,

        // Search
        KeyCode::Char('/') => InputAction::FocusSearch,

        // Alerts tab: a=acknowledge selected, c=clear all acknowledged
        KeyCode::Char('a') if current_tab == Tab::Alerts => InputAction::AckAlert,
        KeyCode::Char('c') if current_tab == Tab::Alerts => InputAction::ClearAlerts,

        // Replay tab: s=start, x=stop, e=export HAR, d=show diff
        KeyCode::Char('s') if current_tab == Tab::Replay => InputAction::StartReplay,
        KeyCode::Char('x') if current_tab == Tab::Replay => InputAction::StopReplay,
        KeyCode::Char('e') if current_tab == Tab::Replay => InputAction::ExportHar,
        KeyCode::Char('d') if current_tab == Tab::Replay => InputAction::ShowDiff,

        // Rules tab: a=add, e=edit, d=delete
        KeyCode::Char('a') => InputAction::AddRule,
        KeyCode::Char('e') if current_tab != Tab::Certs => InputAction::EditRule,
        KeyCode::Char('d') => InputAction::DeleteRule,
        KeyCode::Char('s') if current_tab != Tab::Dns && current_tab != Tab::Replay => InputAction::SaveRule,

        // Certs tab: r=regenerate, e=export
        KeyCode::Char('r') if current_tab == Tab::Certs => InputAction::RegenerateCert,
        KeyCode::Char('e') if current_tab == Tab::Certs || current_tab == Tab::Replay => InputAction::ExportCert,

        // DNS tab: s=toggle DNS server, b=toggle blocklist, u=cycle upstream
        KeyCode::Char('s') if current_tab == Tab::Dns => InputAction::ToggleDns,
        KeyCode::Char('b') if current_tab == Tab::Dns => InputAction::ToggleBlocklist,
        KeyCode::Char('u') if current_tab == Tab::Dns => InputAction::CycleUpstream,

        // Clear search (x is also used for stop replay on Replay tab)
        KeyCode::Char('x') if current_tab != Tab::Replay => InputAction::ClearSearch,

        // List navigation
        KeyCode::Up | KeyCode::Char('k') => InputAction::Up,
        KeyCode::Down | KeyCode::Char('j') => InputAction::Down,
        KeyCode::Enter => InputAction::Enter,

        _ => InputAction::None,
    }
}

/// Poll for input with the given timeout.
/// Returns the action for the first key event received, or None if timeout elapsed.
pub fn poll_input(timeout: Duration, current_tab: Tab) -> Option<InputAction> {
    if event::poll(timeout).ok()? {
        if let event::Event::Key(key) = event::read().ok()? {
            let action = handle_key_event(&key, current_tab);
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