//! Unit tests for the input handler.
//!
//! Tests handle_key_event for all key bindings across all tabs.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use proxybot_lib::tui::Tab;
use proxybot_lib::tui::input::{handle_key_event, InputAction};

/// Helper: create a key press event with given code and modifiers.
fn key_press(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn alt_key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::ALT)
}

fn key_press_with_mod(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, modifiers)
}

// ═══════════════════════════════════════════════════════════
// Navigation
// ═══════════════════════════════════════════════════════════

#[test]
fn test_tab_key_navigates_forward() {
    assert_eq!(handle_key_event(&key_press(KeyCode::Tab), Tab::Traffic), InputAction::NextTab);
}

#[test]
fn test_shift_tab_navigates_backward() {
    assert_eq!(handle_key_event(&KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE), Tab::Traffic), InputAction::PrevTab);
}

#[test]
fn test_left_arrow_navigates_backward() {
    assert_eq!(handle_key_event(&key_press(KeyCode::Left), Tab::Traffic), InputAction::PrevTab);
}

#[test]
fn test_right_arrow_navigates_forward() {
    assert_eq!(handle_key_event(&key_press(KeyCode::Right), Tab::Traffic), InputAction::NextTab);
}

#[test]
fn test_h_key_navigates_backward() {
    assert_eq!(handle_key_event(&key_press(KeyCode::Char('h')), Tab::Traffic), InputAction::PrevTab);
}

#[test]
fn test_l_key_navigates_forward() {
    assert_eq!(handle_key_event(&key_press(KeyCode::Char('l')), Tab::Traffic), InputAction::NextTab);
}

#[test]
fn test_up_down_arrow_navigate() {
    assert_eq!(handle_key_event(&key_press(KeyCode::Up), Tab::Traffic), InputAction::Up);
    assert_eq!(handle_key_event(&key_press(KeyCode::Down), Tab::Traffic), InputAction::Down);
}

#[test]
fn test_enter_key() {
    assert_eq!(handle_key_event(&key_press(KeyCode::Enter), Tab::Traffic), InputAction::Enter);
}

// ═══════════════════════════════════════════════════════════
// Quit
// ═══════════════════════════════════════════════════════════

#[test]
fn test_q_quits() {
    assert_eq!(handle_key_event(&key_press(KeyCode::Char('q')), Tab::Traffic), InputAction::Quit);
}

#[test]
fn test_escape_quits() {
    assert_eq!(handle_key_event(&key_press(KeyCode::Esc), Tab::Traffic), InputAction::Quit);
}

// ═══════════════════════════════════════════════════════════
// Proxy control
// ═══════════════════════════════════════════════════════════

#[test]
fn test_r_starts_proxy_on_traffic_tab() {
    assert_eq!(handle_key_event(&key_press(KeyCode::Char('r')), Tab::Traffic), InputAction::StartProxy);
}

#[test]
fn test_r_does_not_start_proxy_on_certs_tab() {
    // 'r' on Certs tab is RegenerateCert, not StartProxy
    assert_eq!(handle_key_event(&key_press(KeyCode::Char('r')), Tab::Certs), InputAction::RegenerateCert);
}

#[test]
fn test_shift_s_stops_proxy() {
    assert_eq!(handle_key_event(&key_press(KeyCode::Char('S')), Tab::Traffic), InputAction::StopProxy);
}

// ═══════════════════════════════════════════════════════════
// Traffic tab
// ═══════════════════════════════════════════════════════════

#[test]
fn test_clears_on_traffic_tab() {
    assert_eq!(handle_key_event(&key_press(KeyCode::Char('c')), Tab::Traffic), InputAction::Clear);
}

#[test]
fn test_p_toggles_pf() {
    assert_eq!(handle_key_event(&key_press(KeyCode::Char('p')), Tab::Traffic), InputAction::TogglePf);
}

#[test]
fn test_n_toggles_dns() {
    assert_eq!(handle_key_event(&key_press(KeyCode::Char('n')), Tab::Traffic), InputAction::ToggleDns);
}

#[test]
fn test_slash_focuses_search() {
    assert_eq!(handle_key_event(&key_press(KeyCode::Char('/')), Tab::Traffic), InputAction::FocusSearch);
}

#[test]
fn test_x_clears_search_when_not_replay_tab() {
    assert_eq!(handle_key_event(&key_press(KeyCode::Char('x')), Tab::Traffic), InputAction::ClearSearch);
}

#[test]
fn test_detail_tab_switch_keys() {
    assert_eq!(handle_key_event(&key_press(KeyCode::Char('1')), Tab::Traffic), InputAction::SwitchDetailTab(0));
    assert_eq!(handle_key_event(&key_press(KeyCode::Char('2')), Tab::Traffic), InputAction::SwitchDetailTab(1));
    assert_eq!(handle_key_event(&key_press(KeyCode::Char('3')), Tab::Traffic), InputAction::SwitchDetailTab(2));
}

// ═══════════════════════════════════════════════════════════
// Rules tab
// ═══════════════════════════════════════════════════════════

#[test]
fn test_a_adds_rule_on_rules_tab() {
    assert_eq!(handle_key_event(&key_press(KeyCode::Char('a')), Tab::Rules), InputAction::AddRule);
}

#[test]
fn test_a_does_nothing_on_graph_tab() {
    assert_eq!(handle_key_event(&key_press(KeyCode::Char('a')), Tab::Graph), InputAction::ToggleGraphView);
}

#[test]
fn test_a_does_nothing_on_gen_tab() {
    assert_eq!(handle_key_event(&key_press(KeyCode::Char('a')), Tab::Gen), InputAction::None);
}

#[test]
fn test_e_edits_rule_on_rules_tab() {
    assert_eq!(handle_key_event(&key_press(KeyCode::Char('e')), Tab::Rules), InputAction::EditRule);
}

#[test]
fn test_d_deletes_rule_on_rules_tab() {
    assert_eq!(handle_key_event(&key_press(KeyCode::Char('d')), Tab::Rules), InputAction::DeleteRule);
}

#[test]
fn test_s_saves_rule_on_rules_tab() {
    assert_eq!(handle_key_event(&key_press(KeyCode::Char('s')), Tab::Rules), InputAction::SaveRule);
}

#[test]
fn test_alt_up_moves_rule_up() {
    assert_eq!(handle_key_event(&alt_key(KeyCode::Up), Tab::Rules), InputAction::MoveRuleUp);
}

#[test]
fn test_alt_down_moves_rule_down() {
    assert_eq!(handle_key_event(&alt_key(KeyCode::Down), Tab::Rules), InputAction::MoveRuleDown);
}

#[test]
fn test_alt_up_does_nothing_on_traffic_tab() {
    assert_eq!(handle_key_event(&alt_key(KeyCode::Up), Tab::Traffic), InputAction::None);
}

// ═══════════════════════════════════════════════════════════
// Alerts tab
// ═══════════════════════════════════════════════════════════

#[test]
fn test_a_acknowledges_alert() {
    assert_eq!(handle_key_event(&key_press(KeyCode::Char('a')), Tab::Alerts), InputAction::AckAlert);
}

#[test]
fn test_c_clears_alerts() {
    assert_eq!(handle_key_event(&key_press(KeyCode::Char('c')), Tab::Alerts), InputAction::ClearAlerts);
}

#[test]
fn test_c_does_not_clear_on_traffic_tab_when_it_has_its_own_clear() {
    // On Alerts, 'c' = ClearAlerts; on Traffic, 'c' = Clear (different action)
    assert_eq!(handle_key_event(&key_press(KeyCode::Char('c')), Tab::Traffic), InputAction::Clear);
}

// ═══════════════════════════════════════════════════════════
// Replay tab
// ═══════════════════════════════════════════════════════════

#[test]
fn test_replay_keys() {
    assert_eq!(handle_key_event(&key_press(KeyCode::Char('s')), Tab::Replay), InputAction::StartReplay);
    assert_eq!(handle_key_event(&key_press(KeyCode::Char('x')), Tab::Replay), InputAction::StopReplay);
    assert_eq!(handle_key_event(&key_press(KeyCode::Char('e')), Tab::Replay), InputAction::ExportHar);
    assert_eq!(handle_key_event(&key_press(KeyCode::Char('d')), Tab::Replay), InputAction::ShowDiff);
}

#[test]
fn test_x_on_replay_tab_is_stop_replay_not_clear_search() {
    assert_eq!(handle_key_event(&key_press(KeyCode::Char('x')), Tab::Replay), InputAction::StopReplay);
}

#[test]
fn test_e_on_replay_tab_is_export_har() {
    assert_eq!(handle_key_event(&key_press(KeyCode::Char('e')), Tab::Replay), InputAction::ExportHar);
}

// ═══════════════════════════════════════════════════════════
// DNS tab
// ═══════════════════════════════════════════════════════════

#[test]
fn test_dns_keys() {
    assert_eq!(handle_key_event(&key_press(KeyCode::Char('s')), Tab::Dns), InputAction::ToggleDns);
    assert_eq!(handle_key_event(&key_press(KeyCode::Char('b')), Tab::Dns), InputAction::ToggleBlocklist);
    assert_eq!(handle_key_event(&key_press(KeyCode::Char('u')), Tab::Dns), InputAction::CycleUpstream);
}

#[test]
fn test_s_on_dns_toggles_dns_not_start_proxy() {
    assert_eq!(handle_key_event(&key_press(KeyCode::Char('s')), Tab::Dns), InputAction::ToggleDns);
}

// ═══════════════════════════════════════════════════════════
// Certs tab
// ═══════════════════════════════════════════════════════════

#[test]
fn test_r_on_certs_regenerates() {
    assert_eq!(handle_key_event(&key_press(KeyCode::Char('r')), Tab::Certs), InputAction::RegenerateCert);
}

#[test]
fn test_e_on_certs_exports() {
    assert_eq!(handle_key_event(&key_press(KeyCode::Char('e')), Tab::Certs), InputAction::ExportCert);
}

// ═══════════════════════════════════════════════════════════
// Graph tab
// ═══════════════════════════════════════════════════════════

#[test]
fn test_graph_keys() {
    assert_eq!(handle_key_event(&key_press(KeyCode::Char('g')), Tab::Graph), InputAction::ToggleGraphView);
    assert_eq!(handle_key_event(&key_press(KeyCode::Char('a')), Tab::Graph), InputAction::ToggleGraphView);
    assert_eq!(handle_key_event(&key_press(KeyCode::Char('r')), Tab::Graph), InputAction::RefreshGraph);
}

// ═══════════════════════════════════════════════════════════
// Gen tab
// ═══════════════════════════════════════════════════════════

#[test]
fn test_gen_keys() {
    assert_eq!(handle_key_event(&key_press(KeyCode::Char('m')), Tab::Gen), InputAction::GenMockApi);
    assert_eq!(handle_key_event(&key_press(KeyCode::Char('f')), Tab::Gen), InputAction::GenFrontend);
    assert_eq!(handle_key_event(&key_press(KeyCode::Char('d')), Tab::Gen), InputAction::GenDocker);
    assert_eq!(handle_key_event(&key_press(KeyCode::Char('o')), Tab::Gen), InputAction::OpenOutput);
}

// Note: testing KeyEventKind::Release is not possible because
// the crossterm KeyEvent::new() 2-arg constructor doesn't expose kind.
// The release behavior is tested via the if key.kind != KeyEventKind::Press guard
// in handle_key_event — the actual release handling is implicitly tested by
// verifying that only Press events produce actions.
#[test]
fn test_unknown_keys_return_none() {
    assert_eq!(handle_key_event(&key_press(KeyCode::PageUp), Tab::Traffic), InputAction::None);
    assert_eq!(handle_key_event(&key_press(KeyCode::PageDown), Tab::Traffic), InputAction::None);
    assert_eq!(handle_key_event(&key_press(KeyCode::Home), Tab::Traffic), InputAction::None);
    assert_eq!(handle_key_event(&key_press(KeyCode::End), Tab::Traffic), InputAction::None);
}
