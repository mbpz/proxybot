# Team Collaboration Design Specification

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
