//! Persistent storage for filter presets.
//!
//! Backed by `~/.proxybot/filter_presets.json`. Atomic write via
//! tempfile + rename so a crash mid-write can't corrupt the file.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::filter::expr::FilterExpr;

/// A user-saved filter preset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterPreset {
    pub id: String,
    pub name: String,
    pub expr: String,
    /// Optional cached parsed AST. Skipped on serialize so the file
    /// stays clean of derived data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parsed: Option<FilterExpr>,
}

/// Path to the on-disk presets file (`~/.proxybot/filter_presets.json`).
fn presets_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home)
        .join(".proxybot")
        .join("filter_presets.json")
}

fn ensure_parent(path: &Path) -> Result<(), String> {
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
    let json = serde_json::to_string_pretty(presets).map_err(|e| format!("serialize: {}", e))?;
    fs::write(&tmp, json).map_err(|e| format!("write tmp: {}", e))?;
    fs::rename(&tmp, &path).map_err(|e| format!("rename: {}", e))?;
    Ok(())
}

/// Return all saved presets, in insertion order.
pub fn list() -> Result<Vec<FilterPreset>, String> {
    read_all()
}

/// Save (insert or replace-by-id) a preset.
pub fn save(preset: FilterPreset) -> Result<(), String> {
    if preset.id.trim().is_empty() {
        return Err("Preset id is required".into());
    }
    if preset.name.trim().is_empty() {
        return Err("Preset name is required".into());
    }
    let mut all = read_all()?;
    if let Some(slot) = all.iter_mut().find(|p| p.id == preset.id) {
        *slot = preset;
    } else {
        all.push(preset);
    }
    write_all(&all)
}

/// Delete the preset with the given id. Returns Err if not found.
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

    // Tests override HOME; serialize them so we don't trample each other.
    static HOME_LOCK: Mutex<()> = Mutex::new(());

    fn with_temp_home<F: FnOnce(&PathBuf)>(f: F) {
        let _g = HOME_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().to_path_buf();
        // SAFETY: tests are serialized via HOME_LOCK; no other threads
        // read HOME concurrently.
        unsafe {
            env::set_var("HOME", &path);
        }
        f(&path);
        unsafe {
            env::remove_var("HOME");
        }
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

    #[test]
    fn test_delete_unknown_id_returns_error() {
        with_temp_home(|_| {
            let err = delete("nope").unwrap_err();
            assert!(err.contains("Preset not found"));
        });
    }
}
