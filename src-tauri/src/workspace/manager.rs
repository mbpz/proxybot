//! Workspace management for saving and loading capture sessions.
//!
//! A workspace is exported as a `.proxybot` file, which is a gzip-compressed
//! tar archive containing workspace.json, requests.db, and rules.yaml.

use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use tar::{Archive, Builder, Header};

use crate::workspace::serialize::{Device, Rule, RuleAction, Workspace, WorkspaceInfo};

/// Workspace manager handles saving/loading workspace sessions.
pub struct WorkspaceManager {
    base_dir: PathBuf,
    active: RwLock<Option<String>>,
}

impl WorkspaceManager {
    pub fn new() -> Self {
        let base_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".proxybot")
            .join("workspaces");
        fs::create_dir_all(&base_dir).ok();
        Self {
            base_dir,
            active: RwLock::new(None),
        }
    }

    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// Init a workspace from current config files
    pub fn init(&self, name: &str, _description: &str) -> Result<Workspace, String> {
        let ws_dir = self.base_dir.join(name);
        fs::create_dir_all(&ws_dir).map_err(|e| e.to_string())?;

        let workspace = Workspace::new(
            name.to_string(),
            ws_dir.join("requests.db"),
            vec![],
            vec![],
        );

        // Write workspace.json
        let json = serde_json::to_string_pretty(&workspace).map_err(|e| e.to_string())?;
        fs::write(ws_dir.join("workspace.json"), json).map_err(|e| e.to_string())?;

        // Copy current config files if they exist
        let proxybot_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".proxybot");

        for file in &["rules.yaml", "ca.crt", "config.yaml"] {
            let src = proxybot_dir.join(file);
            if src.exists() {
                fs::copy(&src, ws_dir.join(file)).ok();
            }
        }

        Ok(workspace)
    }

    /// Export a workspace to a .proxybot file.
    ///
    /// The .proxybot format is a gzip-compressed tar archive containing:
    /// - workspace.json — metadata
    /// - requests.db — SQLite database (copy)
    /// - rules.yaml — exported rules
    pub fn export(&self, workspace: &Workspace, path: &Path) -> Result<(), String> {
        let file = File::create(path).map_err(|e| e.to_string())?;
        let enc = GzEncoder::new(file, Compression::default());
        let mut tar = Builder::new(enc);

        // Add workspace.json
        let json = serde_json::to_string_pretty(workspace).map_err(|e| e.to_string())?;
        let mut header = Header::new_ustar();
        header.set_size(json.len() as u64);
        header.set_mode(0o644);
        tar.append_data(&mut header, "workspace.json", json.as_bytes())
            .map_err(|e| e.to_string())?;

        // Add requests.db if it exists and is a relative path
        if workspace.db_path.exists() {
            if workspace.db_path.is_relative() {
                tar.append_path(&workspace.db_path)
                    .map_err(|e| e.to_string())?;
            } else {
                // For absolute paths, open the file and append its contents
                if let Ok(file) = File::open(&workspace.db_path) {
                    let mut header = Header::new_ustar();
                    if let Ok(metadata) = file.metadata() {
                        header.set_size(metadata.len());
                        header.set_mode(0o644);
                        header.set_path("requests.db").map_err(|e| e.to_string())?;
                        tar.append_data(&mut header, "requests.db", &file)
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
        }

        // Add rules.yaml
        let rules_yaml = serde_yaml::to_string(&workspace.rules).map_err(|e| e.to_string())?;
        let mut header = Header::new_ustar();
        header.set_size(rules_yaml.len() as u64);
        header.set_mode(0o644);
        tar.append_data(&mut header, "rules.yaml", rules_yaml.as_bytes())
            .map_err(|e| e.to_string())?;

        // Finish the archive
        let enc = tar.into_inner().map_err(|e| e.to_string())?;
        enc.finish().map_err(|e| e.to_string())?;

        Ok(())
    }

    /// Import a workspace from a .proxybot file.
    ///
    /// Returns the Workspace.
    pub fn import(&self, path: &Path) -> Result<Workspace, String> {
        let file = File::open(path).map_err(|e| e.to_string())?;
        let dec = GzDecoder::new(file);
        let mut archive = Archive::new(dec);

        // Create temp directory for extraction using timestamp-based name
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_dir = self.base_dir.join(".tmp").join(format!("import-{}", timestamp));
        fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;

        // Extract all files
        archive.unpack(&temp_dir).map_err(|e| e.to_string())?;

        // Read workspace.json
        let meta_path = temp_dir.join("workspace.json");
        let json = fs::read_to_string(&meta_path).map_err(|e| e.to_string())?;
        let mut workspace: Workspace = serde_json::from_str(&json).map_err(|e| e.to_string())?;

        // Copy requests.db to the appropriate location if it exists
        let db_src = temp_dir.join("requests.db");
        if db_src.exists() {
            // Create a new temp db path for this workspace
            let ws_db_dir = self.base_dir.join(&workspace.name);
            fs::create_dir_all(&ws_db_dir).map_err(|e| e.to_string())?;
            let ws_db_path = ws_db_dir.join("requests.db");
            fs::copy(&db_src, &ws_db_path).map_err(|e| e.to_string())?;
            workspace.db_path = ws_db_path;
        }

        // Clean up temp directory
        fs::remove_dir_all(&temp_dir).ok();

        Ok(workspace)
    }

    /// Get info about a .proxybot file without fully importing it.
    pub fn info(&self, path: &Path) -> Result<WorkspaceInfo, String> {
        let file = File::open(path).map_err(|e| e.to_string())?;
        let dec = GzDecoder::new(file);
        let mut archive = Archive::new(dec);

        // Read workspace.json from the archive
        let mut workspace_json = String::new();
        for entry in archive.entries().map_err(|e| e.to_string())? {
            let mut entry = entry.map_err(|e| e.to_string())?;
            let entry_path = entry.path().map_err(|e| e.to_string())?;
            if entry_path.file_name().map(|s| s == "workspace.json").unwrap_or(false) {
                entry.read_to_string(&mut workspace_json).map_err(|e| e.to_string())?;
                break;
            }
        }

        let workspace: Workspace = serde_json::from_str(&workspace_json)
            .map_err(|e| e.to_string())?;

        let size_bytes = fs::metadata(path).map(|m| m.len()).unwrap_or(0);

        Ok(WorkspaceInfo {
            name: workspace.name,
            created_at: workspace.created_at,
            size_bytes,
            rule_count: workspace.rules.len(),
            device_count: workspace.devices.len(),
        })
    }

    /// List all workspaces in the base directory.
    pub fn list(&self) -> Vec<Workspace> {
        let mut workspaces = Vec::new();
        if let Ok(entries) = fs::read_dir(&self.base_dir) {
            for entry in entries.flatten() {
                let meta_path = entry.path().join("workspace.json");
                if meta_path.exists() {
                    if let Ok(json) = fs::read_to_string(&meta_path) {
                        if let Ok(ws) = serde_json::from_str::<Workspace>(&json) {
                            workspaces.push(ws);
                        }
                    }
                }
            }
        }
        workspaces
    }

    /// Switch to a workspace by name.
    pub fn switch(&self, name: &str) -> Result<(), String> {
        let ws_dir = self.base_dir.join(name);
        if !ws_dir.exists() {
            return Err(format!("Workspace not found: {}", name));
        }
        *self.active.write().unwrap() = Some(name.to_string());

        // Copy workspace config into active .proxybot config
        let proxybot_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".proxybot");

        for file in &["rules.yaml", "ca.crt", "config.yaml"] {
            let src = ws_dir.join(file);
            if src.exists() {
                fs::copy(&src, proxybot_dir.join(file)).ok();
            }
        }

        Ok(())
    }

    /// Get the active workspace name.
    pub fn active(&self) -> Option<String> {
        self.active.read().unwrap().clone()
    }

    /// Set the active workspace.
    pub fn set_active(&self, name: Option<String>) {
        *self.active.write().unwrap() = name;
    }

    /// Get status string.
    pub fn status(&self) -> String {
        match self.active() {
            Some(name) => format!("Active workspace: {}", name),
            None => "No active workspace".to_string(),
        }
    }
}

impl Default for WorkspaceManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::{tempdir, TempDir};

    fn create_test_workspace(tmp: &TempDir) -> Workspace {
        let db_dir = tmp.path().join("db");
        fs::create_dir_all(&db_dir).unwrap();
        let db_path = db_dir.join("requests.db");

        // Create a dummy database file
        fs::write(&db_path, "dummy db content").unwrap();

        let workspace = Workspace::new(
            "test-workspace".to_string(),
            db_path,
            vec![
                Rule::new(
                    "rule-1".to_string(),
                    RuleAction::Direct,
                    "*.example.com".to_string(),
                    true,
                ),
                Rule::new(
                    "rule-2".to_string(),
                    RuleAction::Proxy,
                    "api.example.com".to_string(),
                    false,
                ),
            ],
            vec![
                Device {
                    id: 1,
                    name: "iPhone".to_string(),
                    mac_address: "AA:BB:CC:DD:EE:FF".to_string(),
                },
            ],
        );

        workspace
    }

    #[test]
    fn test_export_and_import_roundtrip() {
        let tmp = tempdir().unwrap();
        let workspace = create_test_workspace(&tmp);

        let export_path = tmp.path().join("test.proxybot");

        let manager = WorkspaceManager::new();

        // Export
        manager.export(&workspace, &export_path).unwrap();
        assert!(export_path.exists());

        // Import
        let imported = manager.import(&export_path).unwrap();

        assert_eq!(imported.name, workspace.name);
        assert_eq!(imported.rules.len(), 2);
        assert_eq!(imported.devices.len(), 1);
        assert_eq!(imported.rules[0].id, "rule-1");
        assert_eq!(imported.rules[0].pattern, "*.example.com");
    }

    #[test]
    fn test_workspace_info() {
        let tmp = tempdir().unwrap();
        let workspace = create_test_workspace(&tmp);

        let export_path = tmp.path().join("info-test.proxybot");

        let manager = WorkspaceManager::new();
        manager.export(&workspace, &export_path).unwrap();

        let info = manager.info(&export_path).unwrap();

        assert_eq!(info.name, "test-workspace");
        assert_eq!(info.rule_count, 2);
        assert_eq!(info.device_count, 1);
    }

    #[test]
    fn test_manager_default() {
        let manager = WorkspaceManager::default();
        assert!(manager.base_dir().exists());
        assert!(manager.active().is_none());
    }

    #[test]
    fn test_init_and_list() {
        let tmp = tempdir().unwrap();
        // Create a custom manager with test base_dir
        // Note: WorkspaceManager uses real homedir; this test validates the data
        // structures work. For full integration tests, use a test helper.
        let ws = Workspace::new(
            "test-ws".to_string(),
            tmp.path().join("test-ws").join("requests.db"),
            vec![],
            vec![],
        );

        // Verify serde round-trip
        let json = serde_json::to_string(&ws).unwrap();
        let parsed: Workspace = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "test-ws");
    }

    #[test]
    fn test_workspace_default() {
        let mgr = WorkspaceManager::default();
        assert!(mgr.base_dir().exists());
    }
}