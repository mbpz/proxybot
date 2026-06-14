# Network Conditions Implementation Plan (2026-06-14 yolo)

**Goal:** Complete the Network Conditions feature per `docs/superpowers/specs/2026-05-10-network-conditions-design.md`.

**Status:** Most infrastructure already exists (`network/mod.rs`, `profile.rs`, `engine.rs`, `ConditionEffect`, `NetworkConditionEngine::apply()`, injection in `pipe_tcp_bidirectional` and `pipe_ws_bidirectional`). This plan fills the remaining gaps: `ConditionRule`, rule-list methods, Tauri commands, and tests.

---

## File Structure

```
src-tauri/src/
├── network/
│   ├── mod.rs          # existing — module exports + built-in presets
│   ├── profile.rs      # existing — NetworkProfile
│   └── engine.rs       # extend — add ConditionRule + add_rule/remove_rule/list_rules
└── commands/
    └── network_conditions.rs   # NEW — 3 Tauri commands
```

Modify: `src-tauri/src/lib.rs` (register commands)

---

## Tasks

### Task 1: Add `ConditionRule` + rule-list methods (TDD)

**Files:** `src-tauri/src/network/engine.rs`

Tests first (red), then implement (green):
- `test_add_rule_increments_id` — add 2 rules, ids 1, 2
- `test_list_rules_returns_added` — add, then list, contains it
- `test_remove_rule_deletes_by_id` — add, remove by id, list empty
- `test_rule_pattern_serialization` — RulePattern variants deserialize
- `test_rule_matching_disabled_skipped` — disabled rules ignored by matcher

Add `ConditionRule { id, pattern, profile, enabled }` reusing `crate::rules::RulePattern`. Methods: `add_rule`, `remove_rule`, `list_rules`, `match_profile_for_host(host)` (returns first matching rule's profile, or None).

### Task 2: 3 Tauri commands

**Files:** `src-tauri/src/commands/network_conditions.rs`, modify `lib.rs`

- `get_network_profiles() -> Vec<NetworkProfile>` — built-in + custom
- `set_active_profile(name: Option<String>) -> Result<(), String>` — None disables
- `get_active_profile() -> Option<NetworkProfile>`

Register a `NetworkConditionsState(Arc<NetworkConditionEngine>)` in `lib.rs` `run()` and add to `generate_handler!`.

### Task 3: Verification

- `cargo build`
- `cargo test` — report counts

---

## Known limitations (yolo accepted)

- Rule pattern matching against live TCP traffic not wired into `pipe_tcp_bidirectional` (needs host extraction from CONNECT/SNI). Rules stored + listed via API; matcher helper exposed for future use.
- CLI subcommands from spec §5 not added — Tauri command surface is the source of truth.
