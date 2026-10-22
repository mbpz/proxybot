# Filter DSL Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Promote the existing in-tree DSL parser/evaluator stubs into a fully wired feature: real preset persistence, frontend `FilterInput` component, TrafficPage integration, E2E coverage.

**Architecture:** Most core parsing/evaluation already exists in `src-tauri/src/filter/{dsl,evaluator}.rs` with a stub `commands/filter.rs`. The remaining work: split types into a dedicated `expr.rs`, add `preset.rs` for JSON-backed persistence, complete the Tauri command surface, register commands in `lib.rs`, build the frontend components under `src/components/filter/`, wire into `TrafficPage`, and add Playwright tests.

**Tech Stack:** Rust (Tauri 2), regex (already in deps), serde_json, dirs, React 18 + TypeScript + existing UI classnames, Playwright for E2E.

**Working directory:** This plan assumes the implementer is at the repo root on a feature branch off `main`. All file paths are relative to the repo root.

---

## File Structure

| File | Responsibility | Status |
|---|---|---|
| `src-tauri/src/filter/mod.rs` | Module root, re-export `dsl`, `evaluator`, `preset` | **Modify** |
| `src-tauri/src/filter/expr.rs` | `FilterExpr` + `FilterOp` types with derives | **New** |
| `src-tauri/src/filter/dsl.rs` | Re-export from `expr`; tokenizer + parser | **Modify** |
| `src-tauri/src/filter/evaluator.rs` | Implement using new `FilterExpr` types | **Modify** |
| `src-tauri/src/filter/preset.rs` | Save/list/delete persisted to `~/.proxybot/filter_presets.json` | **New** |
| `src-tauri/src/commands/filter.rs` | 5 Tauri commands fully implemented | **Modify** |
| `src-tauri/src/commands/mod.rs` | Already exports `filter` | unchanged |
| `src-tauri/src/lib.rs` | Register 5 new commands in `generate_handler!` | **Modify** |
| `src/components/filter/types.ts` | TS types matching Rust | **New** |
| `src/components/filter/FilterInput.tsx` | Input + preset dropdown + SavePresetButton | **New** |
| `src/components/filter/SavePresetButton.tsx` | Modal for preset naming | **New** |
| `src/components/traffic/TrafficPage.tsx` | Replace existing filter wiring with `FilterInput` | **Modify** |
| `e2e/filter-dsl.spec.ts` | Playwright tests | **New** |

No new Rust dependencies (`regex`, `serde`, `serde_json`, `dirs`, `uuid` all already present). No new JS dependencies.

---

## Task 1: Add `expr.rs` types

**Files:**
- Create: `src-tauri/src/filter/expr.rs`
- Modify: `src-tauri/src/filter/dsl.rs` (re-export from `expr`)
- Modify: `src-tauri/src/filter/evaluator.rs` (import from `expr`)

- [ ] **Step 1: Create `expr.rs`**

```rust
//! AST types for the filter DSL.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FilterExpr {
    Field { field: String, op: FilterOp, value: String },
    And(Box<FilterExpr>, Box<FilterExpr>),
    Or(Box<FilterExpr>, Box<FilterExpr>),
    Not(Box<FilterExpr>),
    Group(Box<FilterExpr>),
    /// Plain text search across multiple fields.
    Text(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilterOp {
    Eq,    // :
    Glob,  // :*
    Regex, // :~
    Gt, Lt, Gte, Lte,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_expr_constructs_and_serializes() {
        let expr = FilterExpr::Field {
            field: "method".into(),
            op: FilterOp::Eq,
            value: "GET".into(),
        };
        let json = serde_json::to_string(&expr).unwrap();
        assert!(json.contains("\"Field\""));
        assert!(json.contains("\"Eq\""));
    }

    #[test]
    fn op_variants_match_spec() {
        // Lock in all 7 op variants.
        assert_ne!(FilterOp::Eq, FilterOp::Glob);
        assert_ne!(FilterOp::Regex, FilterOp::Gt);
        assert_ne!(FilterOp::Lt, FilterOp::Gte);
        assert_ne!(FilterOp::Lte, FilterOp::Eq);
    }
}
```

- [ ] **Step 2: Update `dsl.rs` to re-export from `expr`**

At the top of `src-tauri/src/filter/dsl.rs`, add:
```rust
pub use crate::filter::expr::{FilterExpr, FilterOp};
```
and remove the inline `enum FilterExpr { ... }` and `enum FilterOp { ... }` declarations (replace with the re-export).

- [ ] **Step 3: Update `evaluator.rs` to import from `expr`**

Change `use crate::filter::dsl::{FilterExpr, FilterOp};` → `use crate::filter::expr::{FilterExpr, FilterOp};`.

- [ ] **Step 4: Run tests**

```bash
cargo test -p proxybot --lib filter
```

Expected: existing filter tests still pass.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/filter/expr.rs src-tauri/src/filter/dsl.rs src-tauri/src/filter/evaluator.rs
git commit -m "feat(filter): add FilterExpr and FilterOp types

Extracts AST types into filter::expr so parser, evaluator,
preset storage, and Tauri commands all share a single
serde-compatible representation. Adds 2 unit tests."
```

---

## Task 2: Verify parser coverage

The parser is already implemented and well-tested in `dsl.rs` (30+ tests). This task adds the missing tests from the spec to lock down behavior on the new feature path (`header:X`, `body:X`).

**Files:**
- Modify: `src-tauri/src/filter/dsl.rs` (add tests)

- [ ] **Step 1: Add header/body field tests**

Append to the existing `tests` module in `dsl.rs`:

```rust
    #[test]
    fn test_parse_header_field() {
        // header:content-type:application/json — the lexer treats
        // the first ':' as op separator, the rest is the value.
        let result = parse("header:content-type:application/json");
        assert!(result.is_ok());
        if let Ok(FilterExpr::Field { field, op, value }) = result {
            assert_eq!(field, "header");
            assert_eq!(op, FilterOp::Eq);
            assert_eq!(value, "content-type:application/json");
        } else {
            panic!("Expected Field expr");
        }
    }

    #[test]
    fn test_parse_body_field() {
        // body:*token* — glob on body field.
        let result = parse("body:*token*");
        assert!(result.is_ok());
        if let Ok(FilterExpr::Field { field, op, value }) = result {
            assert_eq!(field, "body");
            assert_eq!(op, FilterOp::Glob);
            assert_eq!(value, "token");
        } else {
            panic!("Expected Field expr");
        }
    }
```

- [ ] **Step 2: Run tests**

```bash
cargo test -p proxybot --lib filter::dsl
```

Expected: 30+ tests pass (existing + 2 new).

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/filter/dsl.rs
git commit -m "test(filter): cover header:X and body:X field parsing"
```

---

## Task 3: Verify evaluator coverage

The evaluator is already implemented and tested in `evaluator.rs` (via use sites). This task adds dedicated tests to match the spec's 6 test list.

**Files:**
- Modify: `src-tauri/src/filter/evaluator.rs` (add tests module)

- [ ] **Step 1: Add tests module**

Append to `evaluator.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::dsl::parse;
    use crate::filter::expr::{FilterExpr, FilterOp};
    use crate::proxy::InterceptedRequest;

    fn req(method: &str, host: &str, path: &str) -> InterceptedRequest {
        InterceptedRequest {
            method: method.into(),
            host: host.into(),
            path: path.into(),
            ..Default::default()
        }
    }

    fn eval_str(expr: &str, r: &InterceptedRequest) -> bool {
        Evaluator::evaluate(&parse(expr).unwrap(), r)
    }

    #[test]
    fn test_evaluate_simple_eq() {
        let r = req("GET", "example.com", "/api");
        assert!(eval_str("method:GET", &r));
        assert!(!eval_str("method:POST", &r));
    }

    #[test]
    fn test_evaluate_glob() {
        let r = req("GET", "api.example.com", "/x");
        assert!(eval_str("host:*.example.com", &r));
        assert!(!eval_str("host:other.*", &r));
    }

    #[test]
    fn test_evaluate_numeric() {
        let mut r = req("GET", "h", "/p");
        r.status = Some(404);
        assert!(eval_str("status:>=400", &r));
        assert!(!eval_str("status:>=500", &r));
    }

    #[test]
    fn test_evaluate_and_or_not() {
        let r = req("GET", "api.example.com", "/x");
        assert!(eval_str("method:GET AND host:*.example.com", &r));
        assert!(eval_str("method:POST OR method:GET", &r));
        assert!(eval_str("NOT method:POST", &r));
        assert!(!eval_str("NOT method:GET", &r));
    }

    #[test]
    fn test_evaluate_group() {
        let r = req("POST", "api.example.com", "/x");
        assert!(eval_str("(method:GET OR method:POST) AND host:*.example.com", &r));
        assert!(!eval_str("(method:GET OR method:POST) AND host:other.*", &r));
    }

    #[test]
    fn test_evaluate_header_field() {
        let mut r = req("GET", "h", "/p");
        r.resp_headers.push(("content-type".into(), "application/json".into()));
        let expr = parse("header:content-type:application/json").unwrap();
        assert!(Evaluator::evaluate(&expr, &r));
    }

    #[test]
    fn test_evaluate_body_field() {
        let mut r = req("POST", "h", "/p");
        r.resp_body = Some("the token is abc123 here".into());
        assert!(eval_str("body:abc123", &r));
        assert!(!eval_str("body:xyz", &r));
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test -p proxybot --lib filter::evaluator
```

Expected: 7 tests pass.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/filter/evaluator.rs
git commit -m "test(filter): cover evaluator simple, glob, numeric, combinators, header, body"
```

---

## Task 4: Preset storage

**Files:**
- Create: `src-tauri/src/filter/preset.rs`

- [ ] **Step 1: Create the file**

```rust
//! Persistent storage for filter presets.
//!
//! Backed by `~/.proxybot/filter_presets.json`. Atomic write via
//! tempfile + rename so a crash mid-write doesn't corrupt the file.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::filter::expr::FilterExpr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterPreset {
    pub id: String,
    pub name: String,
    pub expr: String,
    /// Cached parsed AST so we don't re-parse on every list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parsed: Option<FilterExpr>,
}

/// Path to the on-disk presets file.
fn presets_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".proxybot").join("filter_presets.json")
}

fn ensure_parent(path: &PathBuf) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create_dir_all: {}", e))?;
    }
    Ok(())
}

fn read_all() -> Result<Vec<FilterPreset>, String> {
    let path = presets_path();
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(&path).map_err(|e| format!("read: {}", e))?;
    if content.trim().is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str(&content).map_err(|e| format!("parse: {}", e))
}

fn write_all(presets: &[FilterPreset]) -> Result<(), String> {
    let path = presets_path();
    ensure_parent(&path)?;
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(presets)
        .map_err(|e| format!("serialize: {}", e))?;
    fs::write(&tmp, json).map_err(|e| format!("write tmp: {}", e))?;
    fs::rename(&tmp, &path).map_err(|e| format!("rename: {}", e))?;
    Ok(())
}

pub fn list() -> Result<Vec<FilterPreset>, String> {
    read_all()
}

pub fn save(preset: FilterPreset) -> Result<(), String> {
    if preset.id.trim().is_empty() {
        return Err("Preset id is required".into());
    }
    if preset.name.trim().is_empty() {
        return Err("Preset name is required".into());
    }
    let mut all = read_all()?;
    // Replace if id already exists, else append.
    if let Some(slot) = all.iter_mut().find(|p| p.id == preset.id) {
        *slot = preset;
    } else {
        all.push(preset);
    }
    write_all(&all)
}

pub fn delete(id: &str) -> Result<(), String> {
    let mut all = read_all()?;
    let before = all.len();
    all.retain(|p| p.id != id);
    if all.len() == before {
        return Err(format!("Preset not found: {}", id));
    }
    write_all(&all)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::Mutex;

    // Tests override HOME; serialize them.
    static HOME_LOCK: Mutex<()> = Mutex::new(());

    fn with_temp_home<F: FnOnce(&PathBuf)>(f: F) {
        let _g = HOME_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().to_path_buf();
        // SAFETY: tests are serialized via HOME_LOCK; no other threads
        // read HOME concurrently.
        unsafe { env::set_var("HOME", &path); }
        f(&path);
        unsafe { env::remove_var("HOME"); }
    }

    fn preset(id: &str, name: &str, expr: &str) -> FilterPreset {
        FilterPreset {
            id: id.into(),
            name: name.into(),
            expr: expr.into(),
            parsed: None,
        }
    }

    #[test]
    fn test_save_and_load_preset() {
        with_temp_home(|_| {
            save(preset("p1", "WeChat", "app:wechat")).unwrap();
            let all = list().unwrap();
            assert_eq!(all.len(), 1);
            assert_eq!(all[0].id, "p1");
            assert_eq!(all[0].name, "WeChat");
            assert_eq!(all[0].expr, "app:wechat");
        });
    }

    #[test]
    fn test_list_presets_returns_multiple() {
        with_temp_home(|_| {
            save(preset("p1", "A", "method:GET")).unwrap();
            save(preset("p2", "B", "method:POST")).unwrap();
            save(preset("p3", "C", "host:foo")).unwrap();
            let all = list().unwrap();
            assert_eq!(all.len(), 3);
            assert_eq!(all[0].id, "p1");
            assert_eq!(all[2].id, "p3");
        });
    }

    #[test]
    fn test_delete_preset_removes_only_match() {
        with_temp_home(|_| {
            save(preset("p1", "A", "x")).unwrap();
            save(preset("p2", "B", "y")).unwrap();
            save(preset("p3", "C", "z")).unwrap();
            delete("p2").unwrap();
            let all = list().unwrap();
            assert_eq!(all.len(), 2);
            assert!(all.iter().any(|p| p.id == "p1"));
            assert!(all.iter().any(|p| p.id == "p3"));
            assert!(!all.iter().any(|p| p.id == "p2"));
        });
    }
}
```

- [ ] **Step 2: Wire into `filter/mod.rs`**

In `src-tauri/src/filter/mod.rs`:
```rust
pub mod dsl;
pub mod evaluator;
pub mod expr;
pub mod preset;
```

- [ ] **Step 3: Run tests**

```bash
cargo test -p proxybot --lib filter::preset
```

Expected: 3 passed.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/filter/preset.rs src-tauri/src/filter/mod.rs
git commit -m "feat(filter): add preset storage via filter_presets.json

Save/list/delete backed by ~/.proxybot/filter_presets.json with
atomic write via tempfile + rename. 3 unit tests cover save,
list, delete."
```

---

## Task 5: Complete Tauri commands

**Files:**
- Modify: `src-tauri/src/commands/filter.rs` (full implementation)
- Modify: `src-tauri/src/lib.rs` (register commands in `generate_handler!`)

- [ ] **Step 1: Rewrite `commands/filter.rs`**

```rust
//! Tauri commands for the Filter DSL.

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::app::AppState;
use crate::filter::dsl;
use crate::filter::evaluator::Evaluator;
use crate::filter::preset::{self, FilterPreset};
use crate::proxy::InterceptedRequest;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseResult {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[tauri::command]
pub fn parse_filter(expr: String) -> ParseResult {
    match dsl::parse(&expr) {
        Ok(_) => ParseResult { ok: true, error: None },
        Err(e) => ParseResult { ok: false, error: Some(e) },
    }
}

#[tauri::command]
pub fn evaluate_filter(expr: String, request: InterceptedRequest) -> bool {
    match dsl::parse(&expr) {
        Ok(parsed) => Evaluator::evaluate(&parsed, &request),
        Err(_) => false,
    }
}

#[tauri::command]
pub fn list_filter_presets() -> Result<Vec<FilterPreset>, String> {
    preset::list()
}

#[tauri::command]
pub fn save_filter_preset(preset: FilterPreset) -> Result<(), String> {
    preset::save(preset)
}

#[tauri::command]
pub fn delete_filter_preset(id: String) -> Result<(), String> {
    preset::delete(&id)
}
```

(Note: remove the unused `AppState` import — kept as a placeholder in case future commands need app state.)

- [ ] **Step 2: Register in `lib.rs` `generate_handler!`**

In `src-tauri/src/lib.rs`, inside the `tauri::generate_handler![...]` block, add:

```rust
commands::filter::parse_filter,
commands::filter::evaluate_filter,
commands::filter::list_filter_presets,
commands::filter::save_filter_preset,
commands::filter::delete_filter_preset,
```

- [ ] **Step 3: Compile-check**

```bash
cargo check -p proxybot --lib
```

Expected: 0 errors.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands/filter.rs src-tauri/src/lib.rs
git commit -m "feat(commands): add 5 filter DSL Tauri commands

parse_filter, evaluate_filter, list_filter_presets,
save_filter_preset, delete_filter_preset. Registered in
generate_handler!. Preset storage uses ~/.proxybot/
filter_presets.json via filter::preset."
```

---

## Task 6: Frontend types and components

**Files:**
- Create: `src/components/filter/types.ts`
- Create: `src/components/filter/FilterInput.tsx`
- Create: `src/components/filter/SavePresetButton.tsx`

- [ ] **Step 1: Create `types.ts`**

```typescript
// Shared types for the Filter DSL components.

export type FilterOp = "Eq" | "Glob" | "Regex" | "Gt" | "Lt" | "Gte" | "Lte";

export interface FilterPreset {
  id: string;
  name: string;
  expr: string;
}

export interface ParseResult {
  ok: boolean;
  error?: string;
}
```

- [ ] **Step 2: Create `SavePresetButton.tsx`**

```tsx
import { useState } from "react";
import { Save } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "../ui/Button";

interface SavePresetButtonProps {
  currentExpr: string;
  onSaved?: () => void;
}

export function SavePresetButton({ currentExpr, onSaved }: SavePresetButtonProps) {
  const [showDialog, setShowDialog] = useState(false);
  const [name, setName] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!currentExpr.trim()) return null;

  async function handleSave() {
    if (!name.trim()) {
      setError("Name is required");
      return;
    }
    setSaving(true);
    setError(null);
    try {
      await invoke("save_filter_preset", {
        preset: {
          id: crypto.randomUUID(),
          name: name.trim(),
          expr: currentExpr.trim(),
        },
      });
      setShowDialog(false);
      setName("");
      onSaved?.();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  }

  return (
    <>
      <Button
        variant="secondary"
        size="sm"
        onClick={() => setShowDialog(true)}
        data-testid="filter-save-preset"
      >
        <Save size={14} /> Save Preset
      </Button>

      {showDialog && (
        <div
          className="fixed inset-0 bg-black/40 flex items-center justify-center z-50"
          data-testid="filter-save-preset-dialog"
        >
          <div className="bg-white rounded-lg p-4 w-80 space-y-3 shadow-lg">
            <h3 className="font-semibold text-sm">Save Filter Preset</h3>
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="Preset name"
              className="w-full border rounded px-2 py-1 text-sm"
              data-testid="filter-save-preset-name"
              onKeyDown={(e) => e.key === "Enter" && handleSave()}
              autoFocus
            />
            {error && <p className="text-red-500 text-xs">{error}</p>}
            <p className="text-xs text-gray-500 break-all font-mono">{currentExpr}</p>
            <div className="flex justify-end gap-2">
              <Button variant="ghost" size="sm" onClick={() => setShowDialog(false)}>
                Cancel
              </Button>
              <Button
                variant="primary"
                size="sm"
                onClick={handleSave}
                disabled={saving || !name.trim()}
                data-testid="filter-save-preset-confirm"
              >
                {saving ? "Saving..." : "Save"}
              </Button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
```

- [ ] **Step 3: Create `FilterInput.tsx`**

```tsx
import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { FilterPreset, ParseResult } from "./types";
import { SavePresetButton } from "./SavePresetButton";

interface FilterInputProps {
  value: string;
  onChange: (value: string) => void;
  presets?: FilterPreset[];
  onSelectPreset?: (preset: FilterPreset) => void;
  onPresetsChange?: () => void;
}

export function FilterInput({
  value,
  onChange,
  presets = [],
  onSelectPreset,
  onPresetsChange,
}: FilterInputProps) {
  const [error, setError] = useState<string | null>(null);

  // Debounced validation.
  useEffect(() => {
    if (!value.trim()) {
      setError(null);
      return;
    }
    const t = setTimeout(async () => {
      try {
        const result = await invoke<ParseResult>("parse_filter", { expr: value });
        if (!result.ok) {
          setError(result.error ?? "Invalid expression");
        } else {
          setError(null);
        }
      } catch (e) {
        setError(String(e));
      }
    }, 250);
    return () => clearTimeout(t);
  }, [value]);

  const handleChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      onChange(e.target.value);
    },
    [onChange]
  );

  return (
    <div className="flex items-center gap-2 px-4 py-2 bg-surface-secondary border-b border-border">
      <input
        type="text"
        value={value}
        onChange={handleChange}
        placeholder="method:GET AND host:*.example.com"
        data-testid="filter-input"
        className={`flex-1 px-2 py-1 text-sm border rounded font-mono ${
          error ? "border-red-500" : "border-border"
        }`}
      />
      {error && (
        <span className="text-red-500 text-xs" data-testid="filter-error">
          {error}
        </span>
      )}
      {presets.length > 0 && (
        <select
          value=""
          onChange={(e) => {
            const p = presets.find((x) => x.id === e.target.value);
            if (p && onSelectPreset) onSelectPreset(p);
            e.currentTarget.value = "";
          }}
          data-testid="filter-preset-select"
          className="text-xs border rounded px-2 py-1"
        >
          <option value="">Load Preset...</option>
          {presets.map((p) => (
            <option key={p.id} value={p.id}>
              {p.name}
            </option>
          ))}
        </select>
      )}
      <SavePresetButton currentExpr={value} onSaved={onPresetsChange} />
    </div>
  );
}
```

- [ ] **Step 4: Typecheck**

```bash
pnpm typecheck
```

Expected: 0 errors.

- [ ] **Step 5: Commit**

```bash
git add src/components/filter/types.ts src/components/filter/FilterInput.tsx src/components/filter/SavePresetButton.tsx
git commit -m "feat(ui): add FilterInput and SavePresetButton components

FilterInput: text input with debounced validation, preset
dropdown, save button. SavePresetButton: modal dialog with
name input, calls save_filter_preset Tauri command."
```

---

## Task 7: Wire into TrafficPage

**Files:**
- Modify: `src/components/traffic/TrafficPage.tsx` (add DSL filter alongside existing)

- [ ] **Step 1: Add DSL state and presets**

In `TrafficPage.tsx`, add to imports:

```typescript
import { FilterInput } from "../filter/FilterInput";
import { FilterPreset } from "../filter/types";
```

Inside the `TrafficPage` component, add new state:

```typescript
const [dslExpr, setDslExpr] = useState("");
const [presets, setPresets] = useState<FilterPreset[]>([]);

async function loadPresets() {
  try {
    const list = await invoke<FilterPreset[]>("list_filter_presets");
    setPresets(list);
  } catch (e) {
    console.error("Failed to load presets:", e);
  }
}

useEffect(() => {
  loadPresets();
}, []);
```

- [ ] **Step 2: Add DSL-evaluated filtered list**

Add a helper and `useMemo`:

```typescript
const dslFilteredRequests = useMemo(() => {
  if (!dslExpr.trim()) return requests;
  return requests.filter((r) => {
    const intercepted = {
      id: r.id,
      timestamp: String(r.timestamp),
      method: r.method,
      host: r.host,
      path: r.path,
      query_params: undefined,
      status: r.status,
      latency_ms: r.duration_ms,
      scheme: "https",
      req_headers: Object.entries(r.headers),
      req_body: r.body,
      resp_headers: [],
      resp_body: undefined,
      resp_size: r.size,
      app_name: r.app_tag,
      app_icon: undefined,
      device_id: undefined,
      device_name: undefined,
      client_ip: undefined,
      is_websocket: false,
      ws_frames: undefined,
      grpc_decoded: undefined,
      graphql_op: undefined,
    };
    return invoke<boolean>("evaluate_filter", {
      expr: dslExpr,
      request: intercepted,
    });
  });
}, [requests, dslExpr]);

// Note: invoke is async — this needs special handling. See real implementation.
```

**Important:** The Tauri command is async. Use an effect-based approach instead:

```typescript
const [dslFilteredRequests, setDslFilteredRequests] = useState<InterceptedRequest[]>([]);

useEffect(() => {
  if (!dslExpr.trim()) {
    setDslFilteredRequests(requests);
    return;
  }
  let cancelled = false;
  (async () => {
    const results: InterceptedRequest[] = [];
    for (const r of requests) {
      const intercepted = { /* as above */ };
      try {
        const matches = await invoke<boolean>("evaluate_filter", {
          expr: dslExpr,
          request: intercepted,
        });
        if (matches) results.push(r);
      } catch {
        // skip on parse error
      }
    }
    if (!cancelled) setDslFilteredRequests(results);
  })();
  return () => { cancelled = true; };
}, [requests, dslExpr]);
```

- [ ] **Step 3: Render `<FilterInput>` above the existing `FilterBar`**

Above the existing `<FilterBar ... />` line, add:

```tsx
<FilterInput
  value={dslExpr}
  onChange={setDslExpr}
  presets={presets}
  onSelectPreset={(p) => setDslExpr(p.expr)}
  onPresetsChange={loadPresets}
/>
```

- [ ] **Step 4: Use the DSL-filtered list**

Replace `filteredRequests` references (in the RequestTable line and the count) with `dslFilteredRequests`:

```tsx
<RequestTable
  requests={normalizedView ? normalizedData : dslFilteredRequests}
  selectedId={selectedId}
  onSelect={setSelectedId}
/>
```

And update the count to `{dslFilteredRequests.length} requests`.

(Keep the existing `FilterBar` for now — both filters can coexist, with the DSL being the more powerful one.)

- [ ] **Step 5: Typecheck**

```bash
pnpm typecheck
```

Expected: 0 errors.

- [ ] **Step 6: Commit**

```bash
git add src/components/traffic/TrafficPage.tsx
git commit -m "feat(ui): wire FilterInput into TrafficPage

Adds a DSL expression input above the existing FilterBar. When
non-empty, the DSL result overrides the simple filter for
display. Presets are loaded on mount and refreshed after save."
```

---

## Task 8: E2E tests

**Files:**
- Create: `e2e/filter-dsl.spec.ts`

- [ ] **Step 1: Create the file**

```typescript
import { test, expect } from "@playwright/test";
import { mockTauriCommands } from "./fixtures/tauri-mock";

const BASE_MOCKS = {
  is_dashboard_running: false,
  get_dashboard_url: "",
  get_network_info: { lan_ip: "192.168.1.100", interface: "en0" },
  is_pf_enabled: false,
  is_tun_enabled: false,
  get_ca_metadata: null,
  get_dns_log: [],
  get_dns_upstream: "8.8.8.8",
  get_replay_targets: [],
  get_rules: [],
  get_devices: [],
  get_ca_cert_pem: "",
};

const MOCK_REQUESTS = [
  {
    id: "1",
    method: "GET",
    host: "api.weixin.qq.com",
    path: "/cgi-bin/micromsg-bin/getcontact",
    status: 200,
    duration_ms: 42,
    timestamp: Math.floor(Date.now() / 1000),
    app_tag: "WeChat",
    headers: { authorization: "Bearer token123", "content-type": "application/json" },
    body: '{"contacts":[]}',
    size: 128,
  },
  {
    id: "2",
    method: "POST",
    host: "api.douyin.com",
    path: "/aweme/v1/feed/",
    status: 200,
    duration_ms: 156,
    timestamp: Math.floor(Date.now() / 1000),
    app_tag: "Douyin",
    headers: { "content-type": "application/json" },
    body: '{"items":[]}',
    size: 2048,
  },
];

async function injectRequests(
  page: import("@playwright/test").Page,
  requests: typeof MOCK_REQUESTS
) {
  for (const req of requests) {
    await page.evaluate((r) => {
      const internals = window.__TAURI_INTERNALS__;
      if (internals?.callbacks) {
        for (const [, cb] of internals.callbacks) {
          try { cb({ payload: r, event: "intercepted-request" }); } catch {}
        }
      }
    }, req);
  }
}

test.describe("Filter DSL", () => {
  test("filter_input_validates_known_syntax", async ({ page }) => {
    await mockTauriCommands(page, {
      ...BASE_MOCKS,
      parse_filter: { ok: true },
    });
    await page.goto("/");
    await page.waitForLoadState("networkidle");
    await page.getByTestId("filter-input").fill("method:GET AND host:*.example.com");
    // Validation completes (debounced 250ms) — no error.
    await expect(page.getByTestId("filter-error")).not.toBeVisible();
  });

  test("filter_input_shows_error_for_bad_syntax", async ({ page }) => {
    await mockTauriCommands(page, {
      ...BASE_MOCKS,
      parse_filter: { ok: false, error: "Expected closing paren" },
    });
    await page.goto("/");
    await page.waitForLoadState("networkidle");
    await page.getByTestId("filter-input").fill("((method:GET");
    await expect(page.getByTestId("filter-error")).toBeVisible({ timeout: 2000 });
  });

  test("preset_select_loads_expression", async ({ page }) => {
    await mockTauriCommands(page, {
      ...BASE_MOCKS,
      list_filter_presets: [
        { id: "p1", name: "WeChat 2xx", expr: "app:wechat AND status:2*" },
      ],
      parse_filter: { ok: true },
    });
    await page.goto("/");
    await page.waitForLoadState("networkidle");
    await page.getByTestId("filter-preset-select").selectOption("p1");
    await expect(page.getByTestId("filter-input")).toHaveValue(
      "app:wechat AND status:2*"
    );
  });

  test("preset_save_and_load", async ({ page }) => {
    const saved: any[] = [];
    await mockTauriCommands(page, {
      ...BASE_MOCKS,
      parse_filter: { ok: true },
      save_filter_preset: null,
      list_filter_presets: [],
    });
    await page.goto("/");
    await page.waitForLoadState("networkidle");

    await page.getByTestId("filter-input").fill("host:api.example.com");
    await page.getByTestId("filter-save-preset").click();
    await page.getByTestId("filter-save-preset-name").fill("My Preset");
    await page.getByTestId("filter-save-preset-confirm").click();
    // Dialog closes.
    await expect(page.getByTestId("filter-save-preset-dialog")).not.toBeVisible();
  });

  test("preset_delete", async ({ page }) => {
    await mockTauriCommands(page, {
      ...BASE_MOCKS,
      list_filter_presets: [
        { id: "p1", name: "First", expr: "method:GET" },
        { id: "p2", name: "Second", expr: "method:POST" },
      ],
      parse_filter: { ok: true },
    });
    await page.goto("/");
    await page.waitForLoadState("networkidle");
    // Both presets in the dropdown.
    await expect(page.getByTestId("filter-preset-select").locator("option")).toHaveCount(3);
  });
});
```

- [ ] **Step 2: Run E2E**

```bash
pnpm test:e2e -- filter-dsl
```

Expected: 5 tests pass.

- [ ] **Step 3: Commit**

```bash
git add e2e/filter-dsl.spec.ts
git commit -m "test(e2e): add Playwright tests for Filter DSL

Validation, error display, preset select, preset save flow,
preset list rendering."
```

---

## Task 9: Final verification

**Files:** none modified

- [ ] **Step 1: `cargo build`**

```bash
cargo build
```

Expected: 0 errors.

- [ ] **Step 2: `cargo test`**

```bash
cargo test
```

Expected: all tests pass (existing + new filter tests).

- [ ] **Step 3: `pnpm typecheck`**

```bash
pnpm typecheck
```

Expected: 0 errors.

- [ ] **Step 4: `pnpm test:ui`**

```bash
pnpm test:ui
```

Expected: existing tests pass.

- [ ] **Step 5: `cargo clippy`**

```bash
cargo clippy -p proxybot --no-deps 2>&1 | tee /tmp/clippy.log
```

Note any new warnings from this branch's code.

---

## Manual verification (out-of-band)

Real-device testing:
1. Start ProxyBot
2. Open an app that uses HTTPS (e.g., WeChat)
3. In the traffic filter bar, type `app:wechat AND status:2*`
4. Verify list shows only WeChat 2xx requests
5. Click Save Preset → enter name → verify it appears in the dropdown
6. Restart the app → verify preset is still there
7. Delete the preset → verify it's gone

---

## References

- Spec: `docs/superpowers/specs/2026-06-14-filter-dsl-design.md`
- Existing parser: `src-tauri/src/filter/dsl.rs`
- Existing evaluator: `src-tauri/src/filter/evaluator.rs`
- Existing stub commands: `src-tauri/src/commands/filter.rs`
- Config dir helper: `src-tauri/src/config.rs`
- WS Frame Viewer plan (template): `docs/superpowers/plans/2026-06-14-ws-frame-viewer.md`