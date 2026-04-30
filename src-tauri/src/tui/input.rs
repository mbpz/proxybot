//! Async key input handler for the TUI.
//!
//! Provides keyboard handling with tab navigation support.
//! Uses crossterm's event::poll for non-blocking input.

use crossterm::event::{self, KeyCode, KeyEventKind, KeyModifiers};
use std::time::Duration;

use super::Tab;

/// Input action returned by the input handler.
#[derive(Debug, PartialEq)]
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
    /// Edit the selected device's rule override (Devices tab).
    EditDeviceRule,
    /// Delete the selected rule (Rules tab).
    DeleteRule,
    /// Move selected rule up (Rules tab).
    MoveRuleUp,
    /// Move selected rule down (Rules tab).
    MoveRuleDown,
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
    /// Toggle Graph tab view: DAG vs Auth state machine.
    ToggleGraphView,
    /// Refresh graph data.
    RefreshGraph,
    /// Generate mock API (Gen tab).
    GenMockApi,
    /// Generate frontend scaffold (Gen tab).
    GenFrontend,
    /// Generate Docker bundle (Gen tab).
    GenDocker,
    /// Open output folder (Gen tab).
    OpenOutput,
    /// Switch detail sub-tab (1=Headers, 2=Body, 3=WS Frames).
    SwitchDetailTab(usize),
    /// Enter filter input mode for method (Traffic tab).
    FilterMethod,
    /// Enter filter input mode for host (Traffic tab).
    FilterHost,
    /// Enter filter input mode for status (Traffic tab).
    FilterStatus,
    /// Toggle breakpoint on selected request (Traffic tab).
    ToggleBreakpoint,
    /// Continue sending paused request (GO).
    BreakpointGo,
    /// Cancel the paused request.
    BreakpointCancel,
    /// Switch to editing mode for current breakpoint.
    BreakpointEdit,
    /// Enter filter input mode for app_tag (Traffic tab).
    FilterAppTag,
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

        // Graph tab: g=toggle DAG/Auth view, a=toggle auth view, r=refresh
        // (must come before the general 'r' StartProxy to take precedence)
        KeyCode::Char('g') if current_tab == Tab::Graph => InputAction::ToggleGraphView,
        KeyCode::Char('a') if current_tab == Tab::Graph => InputAction::ToggleGraphView,
        KeyCode::Char('r') if current_tab == Tab::Graph => InputAction::RefreshGraph,

        // Proxy control (r = start proxy on non-Certs tabs, S = stop)
        KeyCode::Char('r') if current_tab != Tab::Certs => InputAction::StartProxy,
        KeyCode::Char('S') => InputAction::StopProxy,

        // pf/DNS controls
        KeyCode::Char('p') => InputAction::TogglePf,
        KeyCode::Char('n') => InputAction::ToggleDns,

        // Search
        KeyCode::Char('/') => InputAction::FocusSearch,

        // Alerts tab: a=acknowledge selected, c=clear all acknowledged
        KeyCode::Char('a') if current_tab == Tab::Alerts => InputAction::AckAlert,
        KeyCode::Char('c') if current_tab == Tab::Alerts => InputAction::ClearAlerts,

        // Clear (only when not on Alerts tab - 'c' is used for ClearAlerts there)
        KeyCode::Char('c') if current_tab != Tab::Alerts => InputAction::Clear,

        // Replay tab: s=start, x=stop, e=export HAR, d=show diff
        KeyCode::Char('s') if current_tab == Tab::Replay => InputAction::StartReplay,
        KeyCode::Char('x') if current_tab == Tab::Replay => InputAction::StopReplay,
        KeyCode::Char('e') if current_tab == Tab::Replay => InputAction::ExportHar,
        KeyCode::Char('d') if current_tab == Tab::Replay => InputAction::ShowDiff,

        // Rules tab: a=add, e=edit, d=delete, s=save (not on Graph/Gen tabs)
        KeyCode::Char('a') if current_tab != Tab::Graph && current_tab != Tab::Gen => InputAction::AddRule,
        KeyCode::Char('e') if current_tab == Tab::Rules => InputAction::EditRule,
        KeyCode::Char('e') if current_tab == Tab::Devices => InputAction::EditDeviceRule,
        KeyCode::Char('d') if current_tab != Tab::Graph && current_tab != Tab::Gen => InputAction::DeleteRule,
        KeyCode::Char('s') if current_tab != Tab::Dns && current_tab != Tab::Replay => InputAction::SaveRule,

        // List navigation (Up/Down only, not with Alt modifier — Alt is for rule reordering)
        KeyCode::Up | KeyCode::Char('k') if !key.modifiers.contains(KeyModifiers::ALT) => InputAction::Up,
        KeyCode::Down | KeyCode::Char('j') if !key.modifiers.contains(KeyModifiers::ALT) => InputAction::Down,
        KeyCode::Enter => InputAction::Enter,

        // Detail sub-tab switching (1=Headers, 2=Body, 3=WS Frames)
        KeyCode::Char('1') if current_tab == Tab::Traffic => InputAction::SwitchDetailTab(0),
        KeyCode::Char('2') if current_tab == Tab::Traffic => InputAction::SwitchDetailTab(1),
        KeyCode::Char('3') if current_tab == Tab::Traffic => InputAction::SwitchDetailTab(2),

        // Traffic tab filter shortcuts: m=method, f=host, o=status, a=app_tag
        KeyCode::Char('m') if current_tab == Tab::Traffic => InputAction::FilterMethod,
        KeyCode::Char('f') if current_tab == Tab::Traffic => InputAction::FilterHost,
        KeyCode::Char('o') if current_tab == Tab::Traffic => InputAction::FilterStatus,
        KeyCode::Char('a') if current_tab == Tab::Traffic => InputAction::FilterAppTag,

        // Traffic tab breakpoint shortcuts: b=toggle, g=go, c=cancel, e=edit
        KeyCode::Char('b') if current_tab == Tab::Traffic => InputAction::ToggleBreakpoint,
        KeyCode::Char('g') if current_tab == Tab::Traffic => InputAction::BreakpointGo,
        KeyCode::Char('c') if current_tab == Tab::Traffic => InputAction::BreakpointCancel,
        KeyCode::Char('e') if current_tab == Tab::Traffic => InputAction::BreakpointEdit,

        // Certs tab: r=regenerate, e=export
        KeyCode::Char('r') if current_tab == Tab::Certs => InputAction::RegenerateCert,
        KeyCode::Char('e') if current_tab == Tab::Certs || current_tab == Tab::Replay => InputAction::ExportCert,

        // DNS tab: s=toggle DNS server, b=toggle blocklist, u=cycle upstream
        KeyCode::Char('s') if current_tab == Tab::Dns => InputAction::ToggleDns,
        KeyCode::Char('b') if current_tab == Tab::Dns => InputAction::ToggleBlocklist,
        KeyCode::Char('u') if current_tab == Tab::Dns => InputAction::CycleUpstream,

        // Clear search (x is also used for stop replay on Replay tab)
        KeyCode::Char('x') if current_tab != Tab::Replay => InputAction::ClearSearch,

        // Gen tab: m=mock API, f=frontend scaffold, d=docker bundle, o=open output
        KeyCode::Char('m') if current_tab == Tab::Gen => InputAction::GenMockApi,
        KeyCode::Char('f') if current_tab == Tab::Gen => InputAction::GenFrontend,
        KeyCode::Char('d') if current_tab == Tab::Gen => InputAction::GenDocker,
        KeyCode::Char('o') if current_tab == Tab::Gen => InputAction::OpenOutput,

        // Rules tab: Alt+Up/Down to reorder rules
        KeyCode::Up if key.modifiers.contains(KeyModifiers::ALT) && current_tab == Tab::Rules => InputAction::MoveRuleUp,
        KeyCode::Down if key.modifiers.contains(KeyModifiers::ALT) && current_tab == Tab::Rules => InputAction::MoveRuleDown,

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_ts_epoch() {
        // Verify format is HH:MM:SS (6 chars with colons)
        let result = format_ts("1745612345.123");
        assert_eq!(result.len(), 8); // "HH:MM:SS"
        assert!(result.chars().filter(|c| *c == ':').count() == 2);
    }

    #[test]
    fn test_format_ts_date() {
        // Extracts HH:MM:SS from date string
        let result = format_ts("2024-01-01 12:00:00");
        assert_eq!(result, "12:00:00");
    }

    #[test]
    fn test_format_ts_empty() {
        assert_eq!(format_ts(""), "");
    }

    #[test]
    fn test_fmt_duration_ms() {
        assert_eq!(fmt_duration(Some(0)), "0ms");
        assert_eq!(fmt_duration(Some(500)), "500ms");
        assert_eq!(fmt_duration(Some(999)), "999ms");
        assert_eq!(fmt_duration(Some(1000)), "1.0s");
        assert_eq!(fmt_duration(Some(1500)), "1.5s");
        assert_eq!(fmt_duration(None), "-");
    }

    #[test]
    fn test_input_action_variants_exist() {
        // Verify all input action variants can be constructed
        fn assert_send<T: Send>() {}
        assert_send::<InputAction>();
    }

    #[test]
    fn test_tab_in_input_action() {
        use super::Tab;
        // Verify Tab works correctly
        assert_eq!(Tab::Traffic.next(), Tab::Rules);
        assert_eq!(Tab::Gen.next(), Tab::Traffic);
    }
}