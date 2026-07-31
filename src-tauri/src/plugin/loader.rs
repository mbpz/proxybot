use super::plugin_trait::Plugin;
use super::registry::PluginRegistry;
use std::path::Path;

pub struct PluginLoader;

/// Symbol name that native plugins must export to return a Plugin instance.
const PLUGIN_CREATE_SYM: &[u8] = b"_plugin_create\0";

impl PluginLoader {
    /// Load all plugins from a directory.
    /// Supports `.dylib` (macOS), `.so` (Linux), and `.wasm` files.
    pub fn load_dir(dir: &Path, registry: &PluginRegistry) -> Result<usize, String> {
        let mut count = 0;
        if !dir.exists() {
            return Ok(0);
        }
        for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            match ext {
                "dylib" | "so" => match Self::load_native_plugin(&path, registry) {
                    Ok(_) => count += 1,
                    Err(e) => log::warn!("Failed to load plugin {:?}: {}", path, e),
                },
                "wasm" => {
                    log::info!(
                        "WASM plugin {:?} skipped — WASM runtime not yet available",
                        path
                    );
                }
                _ => {}
            }
        }
        Ok(count)
    }

    /// Load a native dynamic library plugin (.dylib / .so).
    fn load_native_plugin(path: &Path, registry: &PluginRegistry) -> Result<(), String> {
        unsafe {
            let lib = libloading::Library::new(path)
                .map_err(|e| format!("Failed to load library {:?}: {}", path, e))?;

            // Leak the library so it stays loaded for the lifetime of the process
            // (plugins must outlive the proxy session)
            let lib = Box::leak(Box::new(lib));

            let creator: libloading::Symbol<unsafe extern "C" fn() -> *mut dyn Plugin> = lib
                .get(PLUGIN_CREATE_SYM)
                .map_err(|e| format!("Plugin {:?} missing _plugin_create symbol: {}", path, e))?;

            let plugin_ptr = creator();
            if plugin_ptr.is_null() {
                return Err(format!("Plugin {:?} _plugin_create returned null", path));
            }

            let plugin: Box<dyn Plugin> = Box::from_raw(plugin_ptr);
            log::info!("Loaded plugin: {} from {:?}", plugin.name(), path);
            registry.register(plugin);

            Ok(())
        }
    }
}
