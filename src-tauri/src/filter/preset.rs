//! Persistent storage for filter presets.
//!
//! Backed by `~/.proxybot/filter_presets.json`. Atomic write via
//! tempfile + rename so a crash mid-write can't corrupt the file.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

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

fn ensure_parent(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create_dir_all: {}", e))?;
    }
    Ok(())
}

fn read_all(path: &Path) -> Result<Vec<FilterPreset>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(path).map_err(|e| format!("read: {}", e))?;
    if content.trim().is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str(&content).map_err(|e| format!("parse: {}", e))
}

fn write_all(path: &Path, presets: &[FilterPreset]) -> Result<(), String> {
    ensure_parent(path)?;
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(presets).map_err(|e| format!("serialize: {}", e))?;
    fs::write(&tmp, json).map_err(|e| format!("write tmp: {}", e))?;
    fs::rename(&tmp, path).map_err(|e| format!("rename: {}", e))?;
    Ok(())
}

/// Return all saved presets, in insertion order.
pub fn list(path: &Path) -> Result<Vec<FilterPreset>, String> {
    read_all(path)
}

/// Save (insert or replace-by-id) a preset.
pub fn save(path: &Path, preset: FilterPreset) -> Result<(), String> {
    if preset.id.trim().is_empty() {
        return Err("Preset id is required".into());
    }
    if preset.name.trim().is_empty() {
        return Err("Preset name is required".into());
    }
    let mut all = read_all(path)?;
    if let Some(slot) = all.iter_mut().find(|p| p.id == preset.id) {
        *slot = preset;
    } else {
        all.push(preset);
    }
    write_all(path, &all)
}

/// Delete the preset with the given id. Returns Err if not found.
pub fn delete(path: &Path, id: &str) -> Result<(), String> {
    let mut all = read_all(path)?;
    let before = all.len();
    all.retain(|p| p.id != id);
    if all.len() == before {
        return Err(format!("Preset not found: {}", id));
    }
    write_all(path, &all)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn with_temp_path<F: FnOnce(&Path)>(f: F) {
        let dir = tempfile::tempdir().unwrap();
        f(&dir.path().join("filter_presets.json"));
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
        with_temp_path(|path| {
            save(path, preset("p1", "WeChat", "app:wechat")).unwrap();
            let all = list(path).unwrap();
            assert_eq!(all.len(), 1);
            assert_eq!(all[0].id, "p1");
            assert_eq!(all[0].name, "WeChat");
            assert_eq!(all[0].expr, "app:wechat");
        });
    }

    #[test]
    fn test_list_presets_returns_multiple() {
        with_temp_path(|path| {
            save(path, preset("p1", "A", "method:GET")).unwrap();
            save(path, preset("p2", "B", "method:POST")).unwrap();
            save(path, preset("p3", "C", "host:foo")).unwrap();
            let all = list(path).unwrap();
            assert_eq!(all.len(), 3);
            assert_eq!(all[0].id, "p1");
            assert_eq!(all[2].id, "p3");
        });
    }

    #[test]
    fn test_delete_preset_removes_only_match() {
        with_temp_path(|path| {
            save(path, preset("p1", "A", "x")).unwrap();
            save(path, preset("p2", "B", "y")).unwrap();
            save(path, preset("p3", "C", "z")).unwrap();
            delete(path, "p2").unwrap();
            let all = list(path).unwrap();
            assert_eq!(all.len(), 2);
            assert!(all.iter().any(|p| p.id == "p1"));
            assert!(all.iter().any(|p| p.id == "p3"));
            assert!(!all.iter().any(|p| p.id == "p2"));
        });
    }

    #[test]
    fn test_delete_unknown_id_returns_error() {
        with_temp_path(|path| {
            let err = delete(path, "nope").unwrap_err();
            assert!(err.contains("Preset not found"));
        });
    }
}
