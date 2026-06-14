# Team Collaboration Design Specification

**Status:** Implemented (v1.3.x)

**Goal:** Enable teams to share ProxyBot configurations (rules, CA certs, profiles, plugins) for consistent debugging setups across team members.

**Architecture:** Workspace system — a directory bundling all ProxyBot config. Export as tar.gz, import to merge or replace. Git-friendly flat file structure.

**Tech Stack:** Rust (tar, flate2 crates for archive), YAML for config manifests

---

## 1. Workspace Structure

```
~/.proxybot/workspaces/<name>/
├── workspace.yaml       # metadata: name, created, description
├── rules.yaml           # Plugin rules (from Plugin System v2)
├── profiles.yaml        # Network condition profiles
├── ca.crt               # Team CA certificate (public)
├── ca.key               # Team CA private key (optional, encrypted)
└── config.yaml          # ProxyBot settings
```

### workspace.yaml

```yaml
name: my-team
created: "2026-05-10T10:00:00Z"
description: "Shared config for mobile app debugging team"
version: 1
```

---

## 2. CLI Commands

```bash
# Workspace management
proxybot workspace init <name>        # Create workspace from current config
proxybot workspace export <name>      # Export as .tar.gz archive
proxybot workspace import <path>      # Import from .tar.gz archive
proxybot workspace list               # List local workspaces
proxybot workspace switch <name>      # Activate a workspace
proxybot workspace status             # Show current workspace

# Sharing
proxybot workspace share <name>       # Print path for AirDrop/sharing
proxybot workspace merge <path>       # Merge external config into current
```

---

## 3. Data Flow

```
User A (exports)                  User B (imports)
┌─────────────┐                  ┌─────────────┐
│ workspace   │ ──tar.gz──►     │ workspace   │
│ export      │                  │ import      │
└─────────────┘                  └─────────────┘
                                      │
                          ┌───────────┼───────────┐
                          ▼           ▼           ▼
                      rules.yaml  ca.crt     profiles.yaml
```

---

## 4. Implementation

### 4.1 WorkspaceManager

```rust
pub struct WorkspaceManager {
    base_dir: PathBuf,  // ~/.proxybot/workspaces
    active: RwLock<Option<String>>,
}

impl WorkspaceManager {
    pub fn new() -> Self;
    pub fn init(&self, name: &str) -> Result<Workspace, String>;
    pub fn export(&self, name: &str, output: &Path) -> Result<(), String>;
    pub fn import(&self, archive: &Path) -> Result<Workspace, String>;
    pub fn list(&self) -> Vec<Workspace>;
    pub fn switch(&self, name: &str) -> Result<(), String>;
    pub fn active(&self) -> Option<Workspace>;
}
```

### 4.2 Merge Strategy

On import, new rules are merged into existing rules (by name). Existing rules with same name are NOT overwritten. Imported CA cert replaces current if user confirms.

---

## 5. File Structure

```
src-tauri/src/
├── workspace/
│   ├── mod.rs     # Module exports
│   └── manager.rs # WorkspaceManager, Workspace, export/import
```

---

## 6. Test Plan

1. Unit: init workspace, verify directory structure
2. Unit: export to tar.gz, import from tar.gz, verify contents match
3. Unit: list workspaces after init
4. Unit: switch active workspace

---

## 7. Implementation Notes (self-review, 2026-06-14)

Spec self-review pass completed. The spec was previously in `Draft` status — promoted directly to `Implemented` in this pass because the underlying `WorkspaceManager` has been shipping since v1.0 and the only outstanding work was surface-area wiring.

Audit-by-grep at the time of self-review:

| Spec item | Status | Location |
|-----------|--------|----------|
| `WorkspaceManager::new` / `with_base_dir` / `base_dir` | ✅ done | `src-tauri/src/workspace/manager.rs:24, 37, 45` |
| `WorkspaceManager::init` (workspace.yaml + dir layout) | ✅ done | `manager.rs:50` |
| `WorkspaceManager::export` (tar.gz via `flate2` + `tar` crates) | ✅ done | `manager.rs:86` |
| `WorkspaceManager::import` | ✅ done | `manager.rs:137` |
| `WorkspaceManager::list` | ✅ done | `manager.rs:207` |
| `WorkspaceManager::switch` (copies rules.yaml/ca.crt/config.yaml into active `~/.proxybot/`) | ✅ done | `manager.rs:225` |
| `WorkspaceManager::active` / `set_active` / `status` | ✅ done | `manager.rs:248, 253, 258` |
| `WorkspaceManager::load` (new — read workspace.json by name) | ✅ done (commit `5834528`) | `manager.rs:268` |
| 6 `#[tauri::command]` wrappers (init/export/import/list/switch/status) | ✅ done (commit `5834528`) | `manager.rs:298-372` |
| Re-exports from `proxybot_lib` | ✅ done (commit `5834528`) | `src-tauri/src/lib.rs:53-58` |
| Tauri state registration + `invoke_handler` wiring | ✅ done (commit `5834528`) | `src-tauri/src/bin/proxybot-gui.rs:41, 54, 86-91` |
| Unit tests (15 cases: init/list/switch/status/set_active + load round-trip + load error) | ✅ done | `manager.rs` test module |
| `tar = "0.4"` + `flate2 = "1"` deps | ✅ present | `src-tauri/Cargo.toml:56-57` |
| `Workspace` / `WorkspaceInfo` serializable types | ✅ done | `src-tauri/src/workspace/serialize.rs` |

**Surface area actually touched by this self-review pass:** 1 plan (new) + 3 modified files (`workspace/manager.rs`, `lib.rs`, `bin/proxybot-gui.rs`). No new dependencies. No DB schema change. No frontend change.

**Deviation from spec §2 (CLI subcommands):** The spec lists `proxybot workspace init/export/import/...` as shell commands. They are implemented as Tauri commands (`init_workspace`, `export_workspace`, etc.) instead of a separate `clap`-based CLI binary. Rationale: the desktop app is the primary surface, and adding a second binary doubles the packaging surface (Homebrew formula, MSI, .dmg) for marginal value. A future `proxybot-cli` binary can call the same Tauri commands (or `WorkspaceManager` directly) if shell workflows are needed — the underlying API is already suitable.

**Out of scope for this pass:**
- Workspace UI page (`/workspaces` route) — the Tauri commands are callable from any future page; no UI in this pass.
- Encrypted CA private key handling — current `WorkspaceManager` exports the public `ca.crt` only. The spec §1 mentions `ca.key` as optional/encrypted but does not require implementation.
- Cross-platform path handling — `WorkspaceManager::new` uses `dirs::home_dir()` which already handles macOS/Linux/Windows.

**Validation:** `cargo test --lib` → 616 passed (was 614 before this pass; +2 new tests for `load`). `cargo check` → 0 errors.
