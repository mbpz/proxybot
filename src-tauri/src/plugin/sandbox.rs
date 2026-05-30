/// Sandbox runtime for plugin execution.
///
/// Native plugins (.dylib/.so) run in-process with no sandboxing.
/// WASM plugins (.wasm) are not yet supported — they require a wasmtime runtime.
pub struct WasmSandbox;

impl WasmSandbox {
    pub fn new() -> Self { Self }

    /// Execute a .wasm plugin in a sandboxed environment.
    /// WASM runtime (wasmtime) is not yet integrated — returns an error for .wasm files.
    pub fn execute(
        &self,
        plugin_path: &std::path::Path,
        _request: &mut crate::proxy::InterceptedRequest,
    ) -> Result<(), String> {
        let ext = plugin_path.extension().and_then(|e| e.to_str()).unwrap_or("");
        match ext {
            "dylib" | "so" => {
                // Native plugins are loaded via PluginLoader and executed directly
                // through PluginHooks — no sandbox needed.
                Ok(())
            }
            "wasm" => Err(
                "WASM plugin execution requires wasmtime runtime — not yet integrated. \
                 Use native .dylib plugins instead.".into()
            ),
            _ => Err(format!("Unsupported plugin format: {}", ext)),
        }
    }
}
