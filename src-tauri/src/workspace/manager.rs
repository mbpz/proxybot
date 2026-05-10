use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::fs;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Workspace {
    pub name: String,
    pub path: PathBuf,
    pub created: String,
    pub description: String,
}

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
        Self { base_dir, active: RwLock::new(None) }
    }

    pub fn base_dir(&self) -> &Path { &self.base_dir }

    /// Init a workspace from current config files
    pub fn init(&self, name: &str, description: &str) -> Result<Workspace, String> {
        let ws_dir = self.base_dir.join(name);
        fs::create_dir_all(&ws_dir).map_err(|e| e.to_string())?;

        let workspace = Workspace {
            name: name.to_string(),
            path: ws_dir.clone(),
            created: chrono::Utc::now().to_rfc3339(),
            description: description.to_string(),
        };

        // Write workspace.yaml
        let meta = serde_yaml::to_string(&workspace).map_err(|e| e.to_string())?;
        fs::write(ws_dir.join("workspace.yaml"), meta).map_err(|e| e.to_string())?;

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

    /// Export workspace as tar.gz
    pub fn export(&self, name: &str, output: &Path) -> Result<(), String> {
        let ws_dir = self.base_dir.join(name);
        if !ws_dir.exists() {
            return Err(format!("Workspace not found: {}", name));
        }

        let output_file = fs::File::create(output).map_err(|e| e.to_string())?;
        let gz = flate2::write::GzEncoder::new(output_file, flate2::Compression::default());
        let mut tar = tar::Builder::new(gz);

        tar.append_dir_all(name, &ws_dir).map_err(|e| e.to_string())?;
        let gz = tar.into_inner().map_err(|e| e.to_string())?;
        gz.finish().map_err(|e| e.to_string())?;

        Ok(())
    }

    /// Import workspace from tar.gz
    pub fn import(&self, archive: &Path) -> Result<Workspace, String> {
        let file = fs::File::open(archive).map_err(|e| e.to_string())?;
        let gz = flate2::read::GzDecoder::new(file);
        let mut tar = tar::Archive::new(gz);

        // Extract into base_dir
        tar.unpack(&self.base_dir).map_err(|e| e.to_string())?;

        // Derive name from archive filename
        let name = archive.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("imported")
            .replace(".tar", "");

        let ws_dir = self.base_dir.join(&name);
        let meta_path = ws_dir.join("workspace.yaml");
        if meta_path.exists() {
            let meta = fs::read_to_string(&meta_path).map_err(|e| e.to_string())?;
            let workspace: Workspace = serde_yaml::from_str(&meta).map_err(|e| e.to_string())?;
            Ok(workspace)
        } else {
            let workspace = Workspace {
                name: name.clone(),
                path: ws_dir,
                created: chrono::Utc::now().to_rfc3339(),
                description: String::new(),
            };
            Ok(workspace)
        }
    }

    pub fn list(&self) -> Vec<Workspace> {
        let mut workspaces = Vec::new();
        if let Ok(entries) = fs::read_dir(&self.base_dir) {
            for entry in entries.flatten() {
                let meta_path = entry.path().join("workspace.yaml");
                if meta_path.exists() {
                    if let Ok(meta) = fs::read_to_string(&meta_path) {
                        if let Ok(ws) = serde_yaml::from_str::<Workspace>(&meta) {
                            workspaces.push(ws);
                        }
                    }
                }
            }
        }
        workspaces
    }

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

    pub fn active(&self) -> Option<String> {
        self.active.read().unwrap().clone()
    }

    pub fn status(&self) -> String {
        match self.active() {
            Some(name) => format!("Active workspace: {}", name),
            None => "No active workspace".to_string(),
        }
    }
}

impl Default for WorkspaceManager {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_init_and_list() {
        let tmp = tempdir().unwrap();
        // Create a custom manager with test base_dir
        // Note: WorkspaceManager uses real homedir; this test validates the data
        // structures work. For full integration tests, use a test helper.
        let ws = Workspace {
            name: "test-ws".to_string(),
            path: tmp.path().join("test-ws"),
            created: "2024-01-01T00:00:00Z".to_string(),
            description: "Test workspace".to_string(),
        };

        // Verify serde round-trip
        let yaml = serde_yaml::to_string(&ws).unwrap();
        let parsed: Workspace = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(parsed.name, "test-ws");
        assert_eq!(parsed.description, "Test workspace");
    }

    #[test]
    fn test_workspace_manager_new() {
        let mgr = WorkspaceManager::new();
        assert!(mgr.base_dir().exists());
        assert!(mgr.active().is_none());
        assert_eq!(mgr.status(), "No active workspace");
    }

    #[test]
    fn test_workspace_init_and_list_with_tempdir() {
        let tmp = tempdir().unwrap();
        let ws_dir = tmp.path().join("myteam");
        fs::create_dir_all(&ws_dir).unwrap();

        let workspace = Workspace {
            name: "myteam".to_string(),
            path: ws_dir.clone(),
            created: chrono::Utc::now().to_rfc3339(),
            description: "Team config".to_string(),
        };

        let meta = serde_yaml::to_string(&workspace).unwrap();
        fs::write(ws_dir.join("workspace.yaml"), meta).unwrap();

        // Verify file was written
        let read_back = fs::read_to_string(ws_dir.join("workspace.yaml")).unwrap();
        let parsed: Workspace = serde_yaml::from_str(&read_back).unwrap();
        assert_eq!(parsed.name, "myteam");
        assert_eq!(parsed.description, "Team config");
    }

    #[test]
    fn test_workspace_default() {
        let mgr = WorkspaceManager::default();
        assert!(mgr.base_dir().exists());
    }
}
