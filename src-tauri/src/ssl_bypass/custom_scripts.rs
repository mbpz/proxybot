//! User custom bypass scripts loader.
//!
//! Reads `.js` files from `~/.proxybot/bypass-scripts/` and returns
//! them as `BypassScript` entries with `is_builtin: false`.

use std::path::{Path, PathBuf};

use crate::ssl_bypass::bypass_scripts::BypassScript;

/// Load all custom bypass scripts from `~/.proxybot/bypass-scripts/`.
/// Returns an empty Vec if the directory doesn't exist or has no .js files.
pub fn load_custom_scripts(dir: &Path) -> Vec<BypassScript> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut scripts = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("js") {
            continue;
        }
        if let Some(script) = load_one(&path) {
            scripts.push(script);
        }
    }
    scripts
}

fn load_one(path: &PathBuf) -> Option<BypassScript> {
    let content = std::fs::read_to_string(path).ok()?;
    let id = path.file_stem()?.to_string_lossy().to_string();
    Some(BypassScript {
        id: id.clone(),
        name: format!("Custom: {}", id),
        description: format!("User script from {}", path.display()),
        target_framework: vec!["custom".to_string()],
        script_content: content,
        is_builtin: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn setup_temp_dir() -> (tempfile::TempDir, PathBuf) {
        let tmp = tempfile::TempDir::new().unwrap();
        let scripts_dir = tmp.path().join("bypass-scripts");
        std::fs::create_dir_all(&scripts_dir).unwrap();
        (tmp, scripts_dir)
    }

    #[test]
    fn test_load_custom_scripts_from_dir() {
        let (_tmp, scripts_dir) = setup_temp_dir();
        std::fs::write(scripts_dir.join("my-script.js"), "// my bypass script").unwrap();
        std::fs::write(scripts_dir.join("another.js"), "// another").unwrap();

        let scripts = load_custom_scripts(&scripts_dir);
        assert_eq!(scripts.len(), 2);
        let ids: Vec<&str> = scripts.iter().map(|s| s.id.as_str()).collect();
        assert!(ids.contains(&"my-script"));
        assert!(ids.contains(&"another"));
        for s in &scripts {
            assert!(!s.is_builtin);
        }
    }

    #[test]
    fn test_load_custom_scripts_empty_dir() {
        let (_tmp, scripts_dir) = setup_temp_dir();
        let scripts = load_custom_scripts(&scripts_dir);
        assert_eq!(scripts.len(), 0);
    }

    #[test]
    fn test_load_custom_scripts_dir_not_found() {
        let scripts = load_custom_scripts(Path::new("/tmp/__nonexistent_proxybot_scripts__"));
        assert_eq!(scripts.len(), 0);
    }

    #[test]
    fn test_load_custom_scripts_skips_non_js() {
        let (_tmp, scripts_dir) = setup_temp_dir();
        std::fs::write(scripts_dir.join("real.js"), "// real").unwrap();
        std::fs::write(scripts_dir.join("readme.txt"), "not a script").unwrap();
        std::fs::write(scripts_dir.join("data.json"), "{}").unwrap();

        let scripts = load_custom_scripts(&scripts_dir);
        assert_eq!(scripts.len(), 1);
        assert_eq!(scripts[0].id, "real");
    }
}
