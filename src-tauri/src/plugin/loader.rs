use super::registry::PluginRegistry;
use std::path::Path;

pub struct PluginLoader;

impl PluginLoader {
    /// Load all plugins from a directory
    /// Supports .wasm and .so files
    pub fn load_dir(dir: &Path, registry: &PluginRegistry) -> Result<usize, String> {
        let mut count = 0;
        if !dir.exists() {
            return Ok(0);
        }
        for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "wasm" || e == "so") {
                match Self::load_plugin(&path, registry) {
                    Ok(_) => count += 1,
                    Err(e) => eprintln!("Failed to load plugin {:?}: {}", path, e),
                }
            }
        }
        Ok(count)
    }

    fn load_plugin(_path: &Path, _registry: &PluginRegistry) -> Result<(), String> {
        // TODO: WASM runtime (wasmtime) or native .so (libloading)
        // For now, this is a stub that just logs
        Err("Plugin loading not yet implemented - use native plugins only".into())
    }
}
