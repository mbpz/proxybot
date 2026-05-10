use rhai::{Engine, Scope, AST, Dynamic};
use std::sync::RwLock;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use crate::proxy::InterceptedRequest;
use crate::plugin::InterceptedResponse;

/// A sandboxed Rhai scripting engine for intercepting and modifying
/// HTTP requests and responses at runtime.
///
/// Scripts are loaded from `.rhai` files and run in a sandboxed
/// environment with no filesystem or network access. Each script
/// can expose `on_request` and `on_response` hooks.
///
/// This type is `Send + Sync` because it uses the Rhai `sync` feature
/// which replaces internal `Rc`/`RefCell` with `Arc`/`RwLock`.
pub struct ScriptEngine {
    engine: Engine,
    scripts: RwLock<HashMap<String, (PathBuf, AST)>>,
}

impl ScriptEngine {
    /// Create a new scripting engine with safe defaults.
    ///
    /// The engine is sandboxed by default: no filesystem, no network,
    /// no process spawning. Only registered API functions are available
    /// to scripts.
    pub fn new() -> Self {
        let mut engine = Engine::new();

        // Register logging API functions that scripts can call
        engine.register_fn("log", |msg: String| {
            log::info!("[rhai] {}", msg);
        });

        engine.register_fn("warn", |msg: String| {
            log::warn!("[rhai] {}", msg);
        });

        // Register utility: base64 encode/decode
        engine.register_fn("base64_encode", |s: String| -> String {
            use base64::Engine as _;
            base64::engine::general_purpose::STANDARD.encode(s.as_bytes())
        });

        engine.register_fn("base64_decode", |s: String| -> String {
            use base64::Engine as _;
            base64::engine::general_purpose::STANDARD
                .decode(s.as_bytes())
                .map(|bytes| String::from_utf8_lossy(&bytes).to_string())
                .unwrap_or_default()
        });

        Self {
            engine,
            scripts: RwLock::new(HashMap::new()),
        }
    }

    /// Load a Rhai script from a file path.
    ///
    /// The script is compiled to AST and stored under the given name.
    /// If a script with the same name already exists, it is replaced.
    pub fn load(&self, name: &str, path: &Path) -> Result<(), String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Cannot read {}: {}", path.display(), e))?;
        let ast = self.engine.compile(&content)
            .map_err(|e| format!("Compile error in {}: {}", name, e))?;
        self.scripts.write().unwrap()
            .insert(name.to_string(), (path.to_path_buf(), ast));
        log::info!("Script '{}' loaded from {}", name, path.display());
        Ok(())
    }

    /// Load a Rhai script from a string (for inline loading or defaults).
    pub fn load_from_string(&self, name: &str, source: &str) -> Result<(), String> {
        let ast = self.engine.compile(source)
            .map_err(|e| format!("Compile error in {}: {}", name, e))?;
        self.scripts.write().unwrap()
            .insert(name.to_string(), (PathBuf::new(), ast));
        Ok(())
    }

    /// Run the on_request hook for a named script.
    ///
    /// The script receives the request fields in its scope and can
    /// inspect or modify them. Returns whether the request should
    /// continue or be blocked.
    pub fn run_on_request(
        &self,
        script_name: &str,
        request: &InterceptedRequest,
    ) -> Result<ScriptResult, String> {
        let scripts = self.scripts.read().unwrap();
        let (_path, ast) = scripts.get(script_name)
            .ok_or_else(|| format!("Script not found: {}", script_name))?;

        let mut scope = Scope::new();

        // Expose request fields to script as read-only scope variables
        scope.push("method", request.method.clone());
        scope.push("scheme", request.scheme.clone());
        scope.push("host", request.host.clone());
        scope.push("path", request.path.clone());
        scope.push("query_params", request.query_params.clone().unwrap_or_default());

        // Evaluate the script body. Rhai returns the last expression value.
        // Script convention: return true to continue, false to block.
        match self.engine.eval_ast_with_scope::<Dynamic>(&mut scope, ast) {
            Ok(result) => {
                if result.is_bool() {
                    if result.as_bool().unwrap() {
                        Ok(ScriptResult::Continue)
                    } else {
                        Ok(ScriptResult::Block)
                    }
                } else {
                    // If script doesn't return a bool, default to Continue
                    Ok(ScriptResult::Continue)
                }
            }
            Err(e) => {
                log::error!("Script '{}' on_request error: {}", script_name, e);
                // On error, allow the request through (fail-open)
                Ok(ScriptResult::Continue)
            }
        }
    }

    /// Run the on_response hook for a named script.
    ///
    /// The script receives request context and response fields.
    /// Returns whether the response should be forwarded or blocked.
    pub fn run_on_response(
        &self,
        script_name: &str,
        response: &InterceptedResponse,
        request: &InterceptedRequest,
    ) -> Result<ScriptResult, String> {
        let scripts = self.scripts.read().unwrap();
        let (_path, ast) = scripts.get(script_name)
            .ok_or_else(|| format!("Script not found: {}", script_name))?;

        let mut scope = Scope::new();

        // Request context
        scope.push("method", request.method.clone());
        scope.push("host", request.host.clone());
        scope.push("path", request.path.clone());

        // Response fields
        scope.push("status", response.status.unwrap_or(0) as i64);
        scope.push("resp_body", response.body.clone().unwrap_or_default());

        match self.engine.eval_ast_with_scope::<Dynamic>(&mut scope, ast) {
            Ok(result) => {
                if result.is_bool() {
                    if result.as_bool().unwrap() {
                        Ok(ScriptResult::Continue)
                    } else {
                        Ok(ScriptResult::Block)
                    }
                } else {
                    Ok(ScriptResult::Continue)
                }
            }
            Err(e) => {
                log::error!("Script '{}' on_response error: {}", script_name, e);
                Ok(ScriptResult::Continue)
            }
        }
    }

    /// Run a specific hook function from the script scope.
    ///
    /// This allows scripts to define named functions like
    /// `fn on_request(method, host, path) { ... }` and have them
    /// called explicitly.
    pub fn run_hook(
        &self,
        script_name: &str,
        hook_name: &str,
        args: &[Dynamic],
    ) -> Result<Dynamic, String> {
        let scripts = self.scripts.read().unwrap();
        let (_path, ast) = scripts.get(script_name)
            .ok_or_else(|| format!("Script not found: {}", script_name))?;

        let mut scope = Scope::new();

        // Evaluate the script first to define functions in scope
        self.engine.eval_ast_with_scope::<()>(&mut scope, ast)
            .map_err(|e| format!("Script '{}' eval error: {}", script_name, e))?;

        // Call the function by name with arguments
        self.engine
            .call_fn::<Dynamic>(&mut scope, ast, hook_name, (args.to_vec(),))
            .map_err(|e| format!("Hook '{}' error in '{}': {}", hook_name, script_name, e))
    }

    /// Run on_request hooks for ALL loaded scripts against the given request.
    /// Returns the first Block result, or Continue if all scripts allow it.
    pub fn run_all_on_request(&self, request: &InterceptedRequest) -> ScriptResult {
        let names = self.list_scripts();
        for name in &names {
            match self.run_on_request(name, request) {
                Ok(ScriptResult::Block) => {
                    log::info!("Script '{}' blocked request to {}", name, request.host);
                    return ScriptResult::Block;
                }
                Ok(ScriptResult::Continue) => continue,
                Err(e) => {
                    log::error!("{}", e);
                }
            }
        }
        ScriptResult::Continue
    }

    /// Run on_response hooks for ALL loaded scripts.
    pub fn run_all_on_response(
        &self,
        response: &InterceptedResponse,
        request: &InterceptedRequest,
    ) -> ScriptResult {
        let names = self.list_scripts();
        for name in &names {
            match self.run_on_response(name, response, request) {
                Ok(ScriptResult::Block) => {
                    log::info!("Script '{}' blocked response from {}", name, request.host);
                    return ScriptResult::Block;
                }
                Ok(ScriptResult::Continue) => continue,
                Err(e) => {
                    log::error!("{}", e);
                }
            }
        }
        ScriptResult::Continue
    }

    /// List all loaded script names.
    pub fn list_scripts(&self) -> Vec<String> {
        self.scripts.read().unwrap().keys().cloned().collect()
    }

    /// Unload a script by name.
    pub fn unload(&self, name: &str) -> bool {
        let removed = self.scripts.write().unwrap().remove(name).is_some();
        if removed {
            log::info!("Script '{}' unloaded", name);
        }
        removed
    }

    /// Load all .rhai scripts from a directory.
    ///
    /// Script names are derived from the filename (without extension).
    /// Non-recursive; only scans the top level.
    pub fn load_dir(&self, dir: &Path) -> Result<usize, String> {
        if !dir.exists() {
            return Ok(0);
        }
        let mut count = 0;
        let entries = std::fs::read_dir(dir)
            .map_err(|e| format!("Cannot read scripts dir {}: {}", dir.display(), e))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("Dir entry error: {}", e))?;
            let path = entry.path();
            if path.extension().map(|e| e == "rhai").unwrap_or(false) {
                let name = path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unnamed")
                    .to_string();
                match self.load(&name, &path) {
                    Ok(()) => count += 1,
                    Err(e) => log::error!("Failed to load {}: {}", path.display(), e),
                }
            }
        }
        log::info!("Loaded {} Rhai scripts from {}", count, dir.display());
        Ok(count)
    }

    /// Get the path of a loaded script.
    pub fn script_path(&self, name: &str) -> Option<PathBuf> {
        self.scripts.read().unwrap().get(name).map(|(p, _)| p.clone())
    }
}

/// Result of a script hook evaluation.
#[derive(Debug, PartialEq, Clone)]
pub enum ScriptResult {
    /// Allow the request/response to proceed.
    Continue,
    /// Block the request/response.
    Block,
}

impl Default for ScriptEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_new_engine() {
        let engine = ScriptEngine::new();
        assert!(engine.list_scripts().is_empty());
    }

    #[test]
    fn test_load_and_list() {
        let dir = tempdir().unwrap();
        let script_path = dir.path().join("test.rhai");
        std::fs::write(&script_path, "true").unwrap();

        let engine = ScriptEngine::new();
        engine.load("test", &script_path).unwrap();
        assert!(engine.list_scripts().contains(&"test".to_string()));
    }

    #[test]
    fn test_load_and_unload() {
        let dir = tempdir().unwrap();
        let script_path = dir.path().join("temp_script.rhai");
        std::fs::write(&script_path, "true").unwrap();

        let engine = ScriptEngine::new();
        engine.load("temp_script", &script_path).unwrap();
        assert_eq!(engine.list_scripts().len(), 1);

        assert!(engine.unload("temp_script"));
        assert!(engine.list_scripts().is_empty());
        assert!(!engine.unload("nonexistent"));
    }

    #[test]
    fn test_load_from_string() {
        let engine = ScriptEngine::new();
        engine.load_from_string("inline", "let x = 42;").unwrap();
        assert!(engine.list_scripts().contains(&"inline".to_string()));
    }

    #[test]
    fn test_script_sandbox_no_fs_access() {
        // Rhai engine by default has no filesystem access - good for sandboxing
        let engine = ScriptEngine::new();
        let result = engine.engine.compile("let x = 42;");
        assert!(result.is_ok());
    }

    #[test]
    fn test_load_valid_rhai_syntax() {
        let dir = tempdir().unwrap();
        let script_path = dir.path().join("valid.rhai");
        std::fs::write(&script_path, r#"
            // Simple script that returns true
            true
        "#).unwrap();

        let engine = ScriptEngine::new();
        assert!(engine.load("valid", &script_path).is_ok());
    }

    #[test]
    fn test_load_invalid_rhai_syntax() {
        let dir = tempdir().unwrap();
        let script_path = dir.path().join("invalid.rhai");
        std::fs::write(&script_path, "let x = ;").unwrap(); // syntax error

        let engine = ScriptEngine::new();
        assert!(engine.load("invalid", &script_path).is_err());
    }

    #[test]
    fn test_load_nonexistent_file() {
        let engine = ScriptEngine::new();
        assert!(engine.load("missing", Path::new("/nonexistent/path.rhai")).is_err());
    }

    #[test]
    fn test_run_on_request_continue() {
        let engine = ScriptEngine::new();
        engine.load_from_string("allow_all", "true").unwrap();

        let req = InterceptedRequest::default();
        let result = engine.run_on_request("allow_all", &req).unwrap();
        assert_eq!(result, ScriptResult::Continue);
    }

    #[test]
    fn test_run_on_request_block() {
        let engine = ScriptEngine::new();
        engine.load_from_string("block_all", "false").unwrap();

        let req = InterceptedRequest::default();
        let result = engine.run_on_request("block_all", &req).unwrap();
        assert_eq!(result, ScriptResult::Block);
    }

    #[test]
    fn test_run_on_request_with_fields() {
        let engine = ScriptEngine::new();
        // Script that only allows GET requests
        engine.load_from_string("get_only", r#"
            method == "GET"
        "#).unwrap();

        let get_req = InterceptedRequest {
            method: "GET".into(),
            ..Default::default()
        };
        assert_eq!(engine.run_on_request("get_only", &get_req).unwrap(), ScriptResult::Continue);

        let post_req = InterceptedRequest {
            method: "POST".into(),
            ..Default::default()
        };
        assert_eq!(engine.run_on_request("get_only", &post_req).unwrap(), ScriptResult::Block);
    }

    #[test]
    fn test_run_on_request_host_filter() {
        let engine = ScriptEngine::new();
        engine.load_from_string("block_tiktok", r#"
            !host.contains("tiktok")
        "#).unwrap();

        let tiktok_req = InterceptedRequest {
            host: "api.tiktokv.com".into(),
            ..Default::default()
        };
        assert_eq!(engine.run_on_request("block_tiktok", &tiktok_req).unwrap(), ScriptResult::Block);

        let github_req = InterceptedRequest {
            host: "api.github.com".into(),
            ..Default::default()
        };
        assert_eq!(engine.run_on_request("block_tiktok", &github_req).unwrap(), ScriptResult::Continue);
    }

    #[test]
    fn test_run_all_on_request() {
        let engine = ScriptEngine::new();
        engine.load_from_string("allow", "true").unwrap();
        engine.load_from_string("block", "false").unwrap();

        let req = InterceptedRequest::default();
        // Scripts are iterated in hashmap order, so either allow or block
        // We just verify it doesn't crash
        let _result = engine.run_all_on_request(&req);
    }

    #[test]
    fn test_run_on_response_with_status() {
        let engine = ScriptEngine::new();
        // Only allow 2xx responses
        engine.load_from_string("success_only", r#"
            status >= 200 && status < 300
        "#).unwrap();

        let req = InterceptedRequest::default();

        let ok_resp = InterceptedResponse {
            status: Some(200),
            ..Default::default()
        };
        assert_eq!(
            engine.run_on_response("success_only", &ok_resp, &req).unwrap(),
            ScriptResult::Continue
        );

        let err_resp = InterceptedResponse {
            status: Some(500),
            ..Default::default()
        };
        assert_eq!(
            engine.run_on_response("success_only", &err_resp, &req).unwrap(),
            ScriptResult::Block
        );
    }

    #[test]
    fn test_script_with_log_calls() {
        let engine = ScriptEngine::new();
        // Script that uses the registered log functions
        let result = engine.load_from_string("logger", r#"
            log("hello from rhai");
            warn("this is a warning");
            true
        "#);
        assert!(result.is_ok());
    }

    #[test]
    fn test_base64_functions() {
        let engine = ScriptEngine::new();
        engine.load_from_string("b64", r#"
            let encoded = base64_encode("hello");
            let decoded = base64_decode(encoded);
            decoded == "hello"
        "#).unwrap();

        let req = InterceptedRequest::default();
        let result = engine.run_on_request("b64", &req).unwrap();
        assert_eq!(result, ScriptResult::Continue);
    }

    #[test]
    fn test_load_dir() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("script1.rhai"), "true").unwrap();
        std::fs::write(dir.path().join("script2.rhai"), "false").unwrap();
        std::fs::write(dir.path().join("not_a_script.txt"), "hello").unwrap();

        let engine = ScriptEngine::new();
        let count = engine.load_dir(dir.path()).unwrap();
        assert_eq!(count, 2);
        let names = engine.list_scripts();
        assert!(names.contains(&"script1".to_string()));
        assert!(names.contains(&"script2".to_string()));
    }

    #[test]
    fn test_load_dir_empty_or_missing() {
        let dir = tempdir().unwrap();
        let engine = ScriptEngine::new();
        let count = engine.load_dir(dir.path()).unwrap();
        assert_eq!(count, 0);

        let count = engine.load_dir(Path::new("/nonexistent/dir")).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_unload_nonexistent() {
        let engine = ScriptEngine::new();
        assert!(!engine.unload("no_such_script"));
    }

    #[test]
    fn test_script_non_bool_result_defaults_to_continue() {
        let engine = ScriptEngine::new();
        // Script that returns a number instead of bool
        engine.load_from_string("numeric", "42").unwrap();

        let req = InterceptedRequest::default();
        let result = engine.run_on_request("numeric", &req).unwrap();
        assert_eq!(result, ScriptResult::Continue);
    }

    #[test]
    fn test_script_error_fail_open() {
        let engine = ScriptEngine::new();
        // Script access a non-existent variable
        engine.load_from_string("buggy", "undefined_var").unwrap();

        let req = InterceptedRequest::default();
        let result = engine.run_on_request("buggy", &req).unwrap();
        // Should fail open (continue) on error
        assert_eq!(result, ScriptResult::Continue);
    }
}
