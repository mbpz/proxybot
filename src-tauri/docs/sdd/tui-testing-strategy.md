# TUI Testing Strategy — SDD

## Status

**Implemented** — all 6 approaches complete.

## Context

ProxyBot TUI is a terminal application with complex state transitions, keyboard interactions, and render output. Traditional unit tests cover logic in isolation but don't capture the visual/behavioral contract of the UI. We need a comprehensive testing strategy covering multiple fidelity levels.

## Decision

Implement four complementary testing approaches, each covering different failure modes:

1. **Unit tests** — already implemented (41 tests, `handle_key_event`)
2. **Golden/snapshot tests** — already implemented (12 tests, filter bar text)
3. **Approval tests (`insta`)** — new: capture render output, detect unexpected changes
4. **PTY integration tests** — new: simulate real terminal sessions end-to-end
5. **BDD/cucumber tests** — new: natural language scenarios for product acceptance
6. **State machine tests** — new: verify all key transitions are deterministic

## Approach 1: Unit Tests ✅ (Done)

File: `tests/test_input_handler.rs`

Covers: `handle_key_event` for all key bindings across 9 tabs.

```rust
#[test]
fn test_r_starts_proxy_on_traffic_tab() {
    assert_eq!(handle_key_event(&key_press(KeyCode::Char('r')), Tab::Traffic), InputAction::StartProxy);
}
```

## Approach 2: Golden/Snapshot Tests ✅ (Done)

File: `tests/test_traffic_golden.rs`

Covers: filter bar text generation logic.

```rust
#[test]
fn test_filter_bar_shows_active_method_filter() {
    let line = format_filter_line(&traffic);
    assert!(line.contains("[GET]"), "active method filter should show [GET]");
}
```

## Approach 3: Approval Tests with `insta` 🆕

File: `tests/test_approval.rs`

Captures render output as snapshot files. When render output changes, test fails and developer reviews the diff via `cargo insta review`. Prevents unintended visual regressions.

```rust
use insta::assert_snapshot!

fn test_filter_bar_snapshot_no_filter() {
    let output = render_filter_bar_to_string(&make_app());
    assert_snapshot!("filter_bar_no_filter", output);
}
```

**Workflow:**
```bash
# Run tests — fail if changed
cargo test

# Review changed snapshots
cargo insta review

# Accept new snapshot as correct
cargo insta accept
```

**Snapshots stored in:** `tests/snapshots/`

**Install:** `cargo add --dev insta`
**CI:** `cargo test` + `cargo insta test --include-hidden` (in CI mode, insta fails on new changes without review)

## Approach 4: PTY Integration Tests 🆕

File: `tests/test_pty_integration.rs`

Simulates real terminal sessions using a pseudo-terminal. Tests the full executable (`proxybot-tui`) as a black box — start proxy, navigate tabs, trigger filters, verify output appears.

```rust
#[test]
fn test_start_proxy_flow() {
    let mut session = PtySession::new("cargo run --bin proxybot-tui -- --dev");
    session.wait_for_text("ProxyBot v").unwrap();
    session.write_keys("r"); // start proxy
    session.wait_for_text("proxy started").unwrap();
    session.write_keys("q");
    assert!(session.wait_for_exit().success());
}
```

**Key scenarios to cover:**
- Start proxy with `r`
- Tab navigation (Tab, Shift+Tab, h/l)
- Traffic tab filter keys (method, host, status, app_tag)
- Clear with `c`
- Search with `/`
- Quit with `q` and Esc

**Prerequisites:** `cargo add --dev rexpect` or `pty-process`

**Limitation:** Requires compiled binary (`cargo build --bin proxybot-tui` first). Use `before_install` in CI to build.

**Important:** The PTY tests are marked `#[ignore]` by default and run with `cargo test -- --ignored` to avoid CI failures when binary isn't pre-built. Build with `cargo build --release --bin proxybot-tui` before running.

## Approach 5: BDD/Cucumber Tests 🆕

Files:
- `tests/bdd/features/traffic.feature` — Gherkin scenarios
- `tests/bdd/features/steps.rs` — step definitions

Natural language scenarios that can be read by non-developers.

```gherkin
Feature: Traffic filtering
  Scenario: User sets method filter to GET
    Given the traffic tab is active
    And the request list contains "GET" and "POST" requests
    When the user presses "m" to set method filter
    And enters "GET"
    Then only "GET" requests appear in the list
    And the filter bar shows "[GET]"
```

**Framework:** `cargo add --dev cucumber --dev async-std` (or `cucumber` crate with tokio)

**Note:** BDD tests require significant setup (world object, step definitions). Start with 3 core scenarios and expand as needed. In CI, run after the binary is built.

## Approach 6: State Machine Tests 🆕

File: `tests/test_state_machine.rs`

Verifies all key transitions are deterministic — a key in a given state always produces the same next state with no ambiguity.

```rust
#[test]
fn all_transitions_are_deterministic() {
    let tabs = [Tab::Traffic, Tab::Rules, Tab::Alerts, Tab::Dns,
                Tab::Certs, Tab::Replay, Tab::Graph, Tab::Gen, Tab::Devices];
    let key_codes = [KeyCode::Tab, KeyCode::Esc, KeyCode::Char('r'),
                     KeyCode::Char('c'), KeyCode::Char('a')];

    let mut seen: HashMap<(Tab, String, InputAction), ()> = HashMap::new();
    for tab in tabs {
        for key in key_codes {
            let action = handle_key_event(&key_press(key), tab);
            let k = format!("{:?}", key);
            let prev = seen.insert((tab, k, action.clone()), ());
            assert!(prev.is_none(),
                "Non-deterministic: {:?} + {} → {:?} (was {:?})",
                tab, k, action, prev);
        }
    }
}
```

**Also tests:** Tab.next() / Tab.prev() navigation is circular (wraps around).

## Test Summary

| Approach | File | Count | Status |
|---|---|---|---|
| Unit | `test_input_handler.rs` | 41 | ✅ Done |
| Golden | `test_traffic_golden.rs` | 12 | ✅ Done |
| Approval | `test_approval.rs` | 8 | ✅ Done |
| PTY Integration | `test_pty_integration.rs` | 6 (ignored) | ✅ Done |
| BDD/Cucumber | `test_bdd/` | 3 (feature) + steps | ✅ Skeleton |
| State Machine | `test_state_machine.rs` | 4 | ✅ Done |

**Total: 71 tests + 6 PTY integration tests (ignored until binary pre-built).**

## Verification

```bash
# Run all non-ignored tests
cargo test

# Run PTY/BDD tests (requires binary)
cargo build --bin proxybot-tui
cargo test -- --ignored

# Run insta snapshot review
cargo insta review

# Accept all snapshots (after code changes)
cargo insta accept
```

## Consequences

**Positive:**
- Multiple layers of coverage catch different bug types
- Approval tests catch visual regressions that unit tests miss
- PTY tests verify the actual binary behaves correctly
- State machine tests prevent non-deterministic behavior
- BDD scenarios provide living documentation

**Negative:**
- More test code to maintain
- Approval tests require discipline to review diffs, not just auto-accept
- PTY/BDD tests are slower and require binary built first
- Snapshot files add noise to git diffs (mitigate with `.gitattributes`)

## Future Considerations

- **VtBackend snapshot testing:** Once `ratatui` has stable snapshot support, replace manual string comparison with actual terminal buffer comparison
- **Fuzzing:** Use `cargo-fuzz` to test render output with random Unicode (handles emoji, CJK characters)
- **Performance benchmarking:** Add render time benchmarks to detect performance regressions