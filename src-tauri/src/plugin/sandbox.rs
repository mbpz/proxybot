//! Sandbox runtime for plugin execution.
//!
//! ## Current state (WIP — `wip/wasm-sandbox` branch)
//!
//! - Native plugins (`.dylib` / `.so`) run in-process via `libloading`. No
//!   sandboxing is applied because the host already trusts the plugin's
//!   filesystem origin (operator-installed).
//! - WASM plugins (`.wasm`) are NOT yet executable. The host function surface
//!   is not designed, the wasmtime dependency is not pulled in, and the
//!   `WasmState` / `add_host_functions` stubs in `wasm_host.rs` are placeholders.
//!
//! ## Planned implementation
//!
//! When the plugin v2 spec lands and a real WASM host-function surface is
//! designed, this module should:
//! 1. Add `wasmtime = "45"` to `src-tauri/Cargo.toml`.
//! 2. Replace `WasmSandbox` with a struct holding `wasmtime::Engine` configured
//!    for `16 MB` memory reservation (per the design sketch stashed in
//!    this branch's history).
//! 3. Implement `execute_wasm` to: read the `.wasm` bytes, compile via
//!    `Module::from_binary`, instantiate with a `WasmState { request: Mutex<InterceptedRequest> }`
//!    Store, register host functions via `add_host_functions(&mut linker)`,
//!    call the `transform` export if present, then extract the modified
//!    request via `state.request.into_inner().unwrap()`.
//! 4. Add an integration test that loads a fixture `.wasm` module and
//!    verifies a header rewrite round-trips through the sandbox.
//!
//! The frida-sys auto-download devkit blocks `cargo check` on machines that
//! have never built the frida crate before, which is why this work was
//! deferred to a branch rather than landed on main.

use crate::proxy::InterceptedRequest;

pub struct WasmSandbox;

impl WasmSandbox {
    pub fn new() -> Self {
        Self
    }

    /// Execute a plugin against a request.
    ///
    /// - `.dylib` / `.so`: dispatched through `PluginLoader` + `PluginHooks`
    ///   by the caller. This method returns `Ok(())` for native plugins
    ///   because the actual invocation happens elsewhere in the pipeline.
    /// - `.wasm`: not yet supported. Returns an explicit error so the
    ///   operator gets a clear "use a native plugin" message instead of
    ///   a generic load failure.
    pub fn execute(
        &self,
        plugin_path: &std::path::Path,
        _request: &mut InterceptedRequest,
    ) -> Result<(), String> {
        let ext = plugin_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        match ext {
            "dylib" | "so" => {
                // Native plugins are loaded via PluginLoader and executed
                // directly through PluginHooks — no sandbox needed.
                Ok(())
            }
            "wasm" => Err(
                "WASM plugin execution requires wasmtime runtime — not yet integrated. \
                 Use native .dylib plugins instead."
                    .into(),
            ),
            _ => Err(format!("Unsupported plugin format: {}", ext)),
        }
    }
}

impl Default for WasmSandbox {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sandbox_accepts_native_formats() {
        let sandbox = WasmSandbox::new();
        let mut req = InterceptedRequest::default();
        assert!(sandbox.execute(std::path::Path::new("plugin.dylib"), &mut req).is_ok());
        assert!(sandbox.execute(std::path::Path::new("plugin.so"), &mut req).is_ok());
    }

    #[test]
    fn test_sandbox_rejects_wasm_until_wasmtime_integrated() {
        let sandbox = WasmSandbox::new();
        let mut req = InterceptedRequest::default();
        let result = sandbox.execute(std::path::Path::new("plugin.wasm"), &mut req);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("wasmtime"));
    }

    #[test]
    fn test_sandbox_rejects_unknown_format() {
        let sandbox = WasmSandbox::new();
        let mut req = InterceptedRequest::default();
        let result = sandbox.execute(std::path::Path::new("plugin.exe"), &mut req);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unsupported"));
    }
}
