//! User custom bypass scripts loader.
//!
//! Reads `.js` files from `~/.proxybot/bypass-scripts/` and returns
//! them as `BypassScript` entries with `is_builtin: false`.

use std::path::PathBuf;

use crate::ssl_bypass::bypass_scripts::BypassScript;

/// Load all custom bypass scripts from `~/.proxybot/bypass-scripts/`.
/// Returns an empty Vec if the directory doesn't exist or has no .js files.
pub fn load_custom_scripts() -> Vec<BypassScript> {
    let Some(dir) = custom_scripts_dir() else {
        return Vec::new();
    };

    let Ok(entries) = std::fs::read_dir(&dir) else {
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

fn custom_scripts_dir() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    Some(PathBuf::from(home).join(".proxybot").join("bypass-scripts"))
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
    use std::sync::Mutex;

    // Mutex to serialize tests that mutate the HOME env var
    static HOME_LOCK: Mutex<()> = Mutex::new(());

    /// Create a unique temp directory and set HOME to its parent.
    /// Returns (TempDir, bypass-scripts subdir path).
    fn setup_temp_home() -> (tempfile::TempDir, PathBuf) {
        let tmp = tempfile::TempDir::new().unwrap();
        let scripts_dir = tmp.path().join(".proxybot").join("bypass-scripts");
        std::fs::create_dir_all(&scripts_dir).unwrap();
        std::env::set_var("HOME", tmp.path());
        (tmp, scripts_dir)
    }

    #[test]
    fn test_load_custom_scripts_from_dir() {
        let _guard = HOME_LOCK.lock().unwrap();
        let (_tmp, scripts_dir) = setup_temp_home();
        std::fs::write(scripts_dir.join("my-script.js"), "// my bypass script").unwrap();
        std::fs::write(scripts_dir.join("another.js"), "// another").unwrap();

        let scripts = load_custom_scripts();
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
        let _guard = HOME_LOCK.lock().unwrap();
        let (_tmp, _scripts_dir) = setup_temp_home();
        let scripts = load_custom_scripts();
        assert_eq!(scripts.len(), 0);
    }

    #[test]
    fn test_load_custom_scripts_dir_not_found() {
        let _guard = HOME_LOCK.lock().unwrap();
        // Point HOME at a non-existent path
        std::env::set_var("HOME", "/tmp/__nonexistent_home_for_proxybot_test__");
        let scripts = load_custom_scripts();
        assert_eq!(scripts.len(), 0);
    }

    #[test]
    fn test_load_custom_scripts_skips_non_js() {
        let _guard = HOME_LOCK.lock().unwrap();
        let (_tmp, scripts_dir) = setup_temp_home();
        std::fs::write(scripts_dir.join("real.js"), "// real").unwrap();
        std::fs::write(scripts_dir.join("readme.txt"), "not a script").unwrap();
        std::fs::write(scripts_dir.join("data.json"), "{}").unwrap();

        let scripts = load_custom_scripts();
        assert_eq!(scripts.len(), 1);
        assert_eq!(scripts[0].id, "real");
    }
}
