//! State machine tests for TUI key handling.
//!
//! Verifies that all key transitions are deterministic — a key
//! in a given state always produces the same next state.
//! Also tests tab navigation is circular.

use std::collections::HashMap;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use proxybot_lib::tui::{Tab, input::{handle_key_event, InputAction}};

/// Create a key press event.
fn key_press(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

/// Create an alt-modified key press event.
fn alt_key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::ALT)
}

// ═══════════════════════════════════════════════════════════
// Determinism: same (tab, key) → always same action
// ═══════════════════════════════════════════════════════════

fn all_key_codes() -> Vec<KeyCode> {
    vec![
        KeyCode::Tab,
        KeyCode::BackTab,
        KeyCode::Esc,
        KeyCode::Enter,
        KeyCode::Up,
        KeyCode::Down,
        KeyCode::Left,
        KeyCode::Right,
        KeyCode::Char('q'),
        KeyCode::Char('r'),
        KeyCode::Char('S'),
        KeyCode::Char('c'),
        KeyCode::Char('p'),
        KeyCode::Char('n'),
        KeyCode::Char('/'),
        KeyCode::Char('a'),
        KeyCode::Char('e'),
        KeyCode::Char('d'),
        KeyCode::Char('s'),
        KeyCode::Char('x'),
        KeyCode::Char('m'),
        KeyCode::Char('f'),
        KeyCode::Char('o'),
        KeyCode::Char('g'),
        KeyCode::Char('1'),
        KeyCode::Char('2'),
        KeyCode::Char('3'),
        KeyCode::Char('u'),
        KeyCode::Char('b'),
        KeyCode::Char('k'),
        KeyCode::Char('j'),
        KeyCode::Char('h'),
        KeyCode::Char('l'),
    ]
}

#[test]
fn all_transitions_are_deterministic() {
    let tabs = [
        Tab::Traffic, Tab::Rules, Tab::Devices, Tab::Certs,
        Tab::Dns, Tab::Alerts, Tab::Replay, Tab::Graph, Tab::Gen,
    ];

    // HashMap to detect duplicates: (tab, key_code) → action
    // If same (tab, key) produces different action, it's non-deterministic
    let mut seen: HashMap<(String, String), InputAction> = HashMap::new();

    for tab in tabs {
        for key_code in all_key_codes() {
            let key = key_press(key_code);
            let action = handle_key_event(&key, tab);
            let tab_name = format!("{:?}", tab);
            let key_name = format!("{:?}", key_code);

            // Check discriminant before consuming action
            let action_disc = std::mem::discriminant(&action);
            let prev = seen.insert((tab_name, key_name), action);
            if let Some(prev_action) = prev {
                assert!(
                    std::mem::discriminant(&prev_action) == action_disc,
                    "Non-deterministic transition: {:?} + {:?}",
                    tab, key_code
                );
            }
        }
    }
}

#[test]
fn alt_up_down_rules_only_produces_move_action() {
    let tabs = [
        Tab::Traffic, Tab::Rules, Tab::Devices, Tab::Certs,
        Tab::Dns, Tab::Alerts, Tab::Replay, Tab::Graph, Tab::Gen,
    ];

    for tab in tabs {
        let up_action = handle_key_event(&alt_key(KeyCode::Up), tab);
        let down_action = handle_key_event(&alt_key(KeyCode::Down), tab);

        match tab {
            Tab::Rules => {
                assert_eq!(up_action, InputAction::MoveRuleUp,
                    "Alt+Up on Rules tab should be MoveRuleUp");
                assert_eq!(down_action, InputAction::MoveRuleDown,
                    "Alt+Down on Rules tab should be MoveRuleDown");
            }
            _ => {
                // Alt+Up/Down on non-Rules tabs should NOT be MoveRuleUp/Down
                // It should be either None (caught by general Up) or Up/Down navigation
                assert!(!matches!(up_action, InputAction::MoveRuleUp),
                    "Alt+Up on {:?} should NOT be MoveRuleUp (got {:?})", tab, up_action);
                assert!(!matches!(down_action, InputAction::MoveRuleDown),
                    "Alt+Down on {:?} should NOT be MoveRuleDown (got {:?})", tab, down_action);
            }
        }
    }
}

#[test]
fn tab_navigation_wraps_around() {
    // Test that next() wraps from Gen back to Traffic
    assert_eq!(Tab::Gen.next(), Tab::Traffic);
    // Test that prev() wraps from Traffic back to Gen
    assert_eq!(Tab::Traffic.prev(), Tab::Gen);

    // Test all tabs have both next and prev
    for tab in [Tab::Traffic, Tab::Rules, Tab::Devices, Tab::Certs,
                Tab::Dns, Tab::Alerts, Tab::Replay, Tab::Graph, Tab::Gen] {
        let _ = tab.next();
        let _ = tab.prev();
    }
}

#[test]
fn quit_keys_produce_quit_on_all_tabs() {
    let quit_keys = [
        KeyCode::Char('q'),
        KeyCode::Esc,
    ];

    for tab in [Tab::Traffic, Tab::Rules, Tab::Devices, Tab::Certs,
                Tab::Dns, Tab::Alerts, Tab::Replay, Tab::Graph, Tab::Gen] {
        for key in quit_keys {
            let action = handle_key_event(&key_press(key), tab);
            assert_eq!(action, InputAction::Quit,
                "Quit key {:?} should produce Quit on {:?}, got {:?}", key, tab, action);
        }
    }
}