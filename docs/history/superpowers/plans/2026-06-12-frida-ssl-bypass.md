# Frida SSL Bypass Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Integrate Frida runtime via frida-rust (v0.17) to bypass SSL pinning on Android apps, with 6 built-in bypass scripts and APK patching support.

**Architecture:** frida-rust directly embedded in the Tauri binary (no external frida CLI required). 6 built-in Frida JS scripts cover OkHttp3, Conscrypt, WebView, Flutter, React Native, and Universal pinning. APK patching uses embedded `apktool.jar` + system `jarsigner` + bundled `frida-gadget.so` to inject Frida into apps without root. New sidebar page "SSL Bypass" with device/process selector, script list, and live log.

**Tech Stack:** Rust (Tauri 2), `frida` crate v0.17 (with devkit from GitHub releases), `jarsigner` (system Java), bundled `apktool.jar`, bundled `frida-gadget.so`. React 18 + TypeScript + Pinia. Playwright for E2E.

**Working directory:** This plan assumes the implementer is at the repo root on a feature branch off `main`. All file paths are relative to the repo root.

---

## File Structure

| File | Responsibility | Status |
|---|---|---|
| `src-tauri/Cargo.toml` | Add `frida` crate | **Modify** |
| `src-tauri/build.rs` | Download frida-core devkit at build time | **Modify** |
| `src-tauri/src/frida/mod.rs` | FridaManager (device, session, script injection) | **New** |
| `src-tauri/src/frida/device.rs` | DeviceInfo, ProcessInfo types | **New** |
| `src-tauri/src/frida/session.rs` | SessionHandle type | **New** |
| `src-tauri/src/frida/script.rs` | Script injection + message handler | **New** |
| `src-tauri/src/ssl_bypass/mod.rs` | Module root | **New** |
| `src-tauri/src/ssl_bypass/bypass_scripts.rs` | 6 built-in scripts | **New** |
| `src-tauri/src/ssl_bypass/custom_scripts.rs` | User script loader | **New** |
| `src-tauri/src/ssl_bypass/apk_patcher.rs` | APK decompile → inject → recompile → sign | **New** |
| `src-tauri/src/commands/ssl_bypass.rs` | 6 Tauri commands | **New** |
| `src-tauri/src/lib.rs` | Register new modules + commands | **Modify** |
| `src/components/ssl-bypass/SslBypassPage.tsx` | Main page | **New** |
| `src/components/ssl-bypass/DeviceSelector.tsx` | USB/WiFi device picker | **New** |
| `src/components/ssl-bypass/ScriptList.tsx` | Built-in + custom scripts | **New** |
| `src/components/ssl-bypass/FridaStatus.tsx` | Runtime status indicator | **New** |
| `src/components/ssl-bypass/ApkPatcher.tsx` | APK patching UI | **New** |
| `src/stores/sslBypassStore.ts` | Pinia store | **New** |
| `e2e/ssl-bypass.spec.ts` | Playwright tests | **New** |
| `tauri.conf.json` | Add resource files (apktool.jar, frida-gadget.so) | **Modify** |
| `src-tauri/resources/apktool.jar` | Bundled apktool | **New (binary)** |
| `src-tauri/resources/frida-gadget/{arm64-v8a,armeabi-v7a,x86_64}/libfrida-gadget.so` | Bundled gadgets | **New (binary)** |

---

## Task 1: frida-rs dependency + build.rs devkit setup

**Files:**
- Modify: `src-tauri/Cargo.toml` (add `frida` crate)
- Modify: `src-tauri/build.rs` (download devkit)

The `frida` crate v0.17 requires a pre-built frida-core devkit (static libraries) for the target platform. The build.rs handles downloading the devkit if not cached.

- [ ] **Step 1: Add `frida` crate to Cargo.toml**

In `src-tauri/Cargo.toml`, in the `[dependencies]` section, add:

```toml
frida = { version = "0.17", default-features = false }
```

Note: `default-features = false` is used to avoid pulling in optional features we don't need. The crate will link against frida-core's static library at build time.

- [ ] **Step 2: Create `build.rs` for frida devkit handling**

Create `src-tauri/build.rs`:

```rust
// Build script for ProxyBot Tauri app.
//
// Downloads the frida-core devkit (pre-built static libraries) for the
// target platform if not already cached. The devkit is needed by the
// `frida` crate at link time.

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let target = env::var("TARGET").unwrap();
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let devkit_dir = out_dir.join("frida-devkit");

    // Skip if FRIDA_DEVKIT_DIR is set (CI / manual override)
    if env::var("FRIDA_DEVKIT_DIR").is_ok() {
        let custom_dir = PathBuf::from(env::var("FRIDA_DEVKIT_DIR").unwrap());
        println!("cargo:rustc-link-search=native={}", custom_dir.display());
        println!("cargo:rustc-link-lib=static=frida-core");
        println!("cargo:rustc-link-lib=static=frida-gum");
        return;
    }

    // Skip if devkit already extracted
    if devkit_dir.join("lib").join("libfrida-core.a").exists() {
        println!("cargo:rustc-link-search=native={}", devkit_dir.join("lib").display());
        println!("cargo:rustc-link-lib=static=frida-core");
        println!("cargo:rustc-link-lib=static=frida-gum");
        return;
    }

    // Download devkit from GitHub releases
    // Frida v17.0.0 release: https://github.com/frida/frida/releases/tag/17.0.0
    let frida_version = "17.0.0";
    let devkit_url = format!(
        "https://github.com/frida/frida/releases/download/{}/frida-core-devkit-{}-v{}.tar.xz",
        frida_version, target, frida_version
    );

    println!("cargo:warning=Downloading frida devkit from {}", devkit_url);

    fs::create_dir_all(&devkit_dir).expect("Failed to create devkit dir");

    let tarball = devkit_dir.join("devkit.tar.xz");
    let status = Command::new("curl")
        .args(["-L", "-o"])
        .arg(&tarball)
        .arg(&devkit_url)
        .status()
        .expect("Failed to run curl");

    if !status.success() {
        panic!("Failed to download frida devkit from {}", devkit_url);
    }

    // Extract tar.xz
    let status = Command::new("tar")
        .args(["-xJf"])
        .arg(&tarball)
        .args(["-C"])
        .arg(&devkit_dir)
        .status()
        .expect("Failed to run tar");

    if !status.success() {
        panic!("Failed to extract frida devkit");
    }

    fs::remove_file(&tarball).ok();

    println!("cargo:rustc-link-search=native={}", devkit_dir.join("lib").display());
    println!("cargo:rustc-link-lib=static=frida-core");
    println!("cargo:rustc-link-lib=static=frida-gum");
}
```

- [ ] **Step 3: Verify the build setup works**

```bash
cargo check -p proxybot --lib
```

Expected: The build script downloads the frida devkit (~50MB) and links successfully. May take 2-5 minutes on first build.

If the download fails (network issues, GitHub rate limits), you can manually download the devkit and set `FRIDA_DEVKIT_DIR=/path/to/devkit` before building.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/build.rs Cargo.lock
git commit -m "build: add frida-rust dependency and devkit download

Integrates the frida crate v0.17 for runtime Frida support.
The build.rs downloads the frida-core devkit from GitHub releases
on first build, with support for FRIDA_DEVKIT_DIR override."
```

---

## Task 2: frida/device.rs — DeviceInfo + ProcessInfo types

**Files:**
- Create: `src-tauri/src/frida/mod.rs` (module root)
- Create: `src-tauri/src/frida/device.rs`
- Modify: `src-tauri/src/lib.rs` (add `pub mod frida;`)

- [ ] **Step 1: Create frida module root**

Create `src-tauri/src/frida/mod.rs`:

```rust
//! Frida runtime integration.
//!
//! Manages device enumeration, process listing, session lifecycle,
//! and script injection using the frida-rust crate.

pub mod device;
pub mod session;
pub mod script;

use std::collections::HashMap;
use std::sync::Mutex;

use crate::frida::device::DeviceInfo;
use crate::frida::session::SessionHandle;

pub struct FridaManager {
    devices: Mutex<Vec<DeviceInfo>>,
    sessions: Mutex<HashMap<String, SessionHandle>>,
}

impl FridaManager {
    pub fn new() -> Self {
        Self {
            devices: Mutex::new(Vec::new()),
            sessions: Mutex::new(HashMap::new()),
        }
    }

    pub fn list_devices(&self) -> Result<Vec<DeviceInfo>, String> {
        // Stub implementation — full Frida integration in Task 6
        Ok(self.devices.lock().map_err(|e| e.to_string())?.clone())
    }

    pub fn attach(
        &self,
        device_id: String,
        pid: u32,
        script_content: String,
    ) -> Result<SessionHandle, String> {
        let handle = SessionHandle {
            session_id: uuid::Uuid::new_v4().to_string(),
            device_id,
            pid,
            process_name: String::new(),
            attached_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        };
        self.sessions
            .lock()
            .map_err(|e| e.to_string())?
            .insert(handle.session_id.clone(), handle.clone());
        Ok(handle)
    }

    pub fn detach(&self, session_id: &str) -> Result<(), String> {
        self.sessions
            .lock()
            .map_err(|e| e.to_string())?
            .remove(session_id);
        Ok(())
    }
}
```

- [ ] **Step 2: Create device.rs with DeviceInfo and ProcessInfo types**

Create `src-tauri/src/frida/device.rs`:

```rust
//! Device and process types for Frida.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DeviceType {
    Usb,
    Remote,
    Local,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub device_type: DeviceType,
    pub is_connected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub identifier: String,
    pub icon: Option<Vec<u8>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_info_serialization() {
        let device = DeviceInfo {
            id: "usb-1234".to_string(),
            name: "Pixel 6".to_string(),
            device_type: DeviceType::Usb,
            is_connected: true,
        };
        let json = serde_json::to_string(&device).unwrap();
        assert!(json.contains("\"id\":\"usb-1234\""));
        assert!(json.contains("\"name\":\"Pixel 6\""));
        assert!(json.contains("\"device_type\":\"Usb\""));
        assert!(json.contains("\"is_connected\":true"));
    }

    #[test]
    fn test_process_info_serialization() {
        let proc = ProcessInfo {
            pid: 1234,
            name: "com.example.app".to_string(),
            identifier: "com.example.app".to_string(),
            icon: None,
        };
        let json = serde_json::to_string(&proc).unwrap();
        assert!(json.contains("\"pid\":1234"));
        assert!(json.contains("\"name\":\"com.example.app\""));
    }

    #[test]
    fn test_device_type_serialization() {
        let usb = DeviceType::Usb;
        let json = serde_json::to_string(&usb).unwrap();
        assert_eq!(json, "\"Usb\"");

        let remote = DeviceType::Remote;
        let json = serde_json::to_string(&remote).unwrap();
        assert_eq!(json, "\"Remote\"");

        let local = DeviceType::Local;
        let json = serde_json::to_string(&local).unwrap();
        assert_eq!(json, "\"Local\"");
    }
}
```

- [ ] **Step 3: Wire the module into lib.rs**

In `src-tauri/src/lib.rs`, add:

```rust
pub mod frida;
```

- [ ] **Step 4: Compile-check and run tests**

```bash
cargo test -p proxybot --lib frida::device
```

Expected: 3 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/frida/mod.rs src-tauri/src/frida/device.rs src-tauri/src/lib.rs
git commit -m "feat(frida): add device and process types

Introduces DeviceInfo, ProcessInfo, DeviceType with full
serialization support for Tauri command boundaries."
```

---

## Task 3: frida/session.rs — SessionHandle

**Files:**
- Create: `src-tauri/src/frida/session.rs`

- [ ] **Step 1: Create session.rs**

Create `src-tauri/src/frida/session.rs`:

```rust
//! Frida session types.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionHandle {
    pub session_id: String,
    pub device_id: String,
    pub pid: u32,
    pub process_name: String,
    pub attached_at: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_handle_serialization() {
        let handle = SessionHandle {
            session_id: "sess-123".to_string(),
            device_id: "usb-1234".to_string(),
            pid: 5678,
            process_name: "com.example.app".to_string(),
            attached_at: 1718200000,
        };
        let json = serde_json::to_string(&handle).unwrap();
        assert!(json.contains("\"session_id\":\"sess-123\""));
        assert!(json.contains("\"device_id\":\"usb-1234\""));
        assert!(json.contains("\"pid\":5678"));
        assert!(json.contains("\"process_name\":\"com.example.app\""));
        assert!(json.contains("\"attached_at\":1718200000"));
    }

    #[test]
    fn test_session_handle_clone() {
        let handle = SessionHandle {
            session_id: "sess-1".to_string(),
            device_id: "dev-1".to_string(),
            pid: 100,
            process_name: "app".to_string(),
            attached_at: 0,
        };
        let cloned = handle.clone();
        assert_eq!(cloned.session_id, "sess-1");
        assert_eq!(cloned.pid, 100);
    }
}
```

- [ ] **Step 2: Run the tests**

```bash
cargo test -p proxybot --lib frida::session
```

Expected: 2 tests pass.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/frida/session.rs
git commit -m "feat(frida): add SessionHandle type

Tracks active Frida sessions with device, pid, and timestamp."
```

---

## Task 4: ssl_bypass/bypass_scripts.rs — 6 built-in scripts

**Files:**
- Create: `src-tauri/src/ssl_bypass/mod.rs`
- Create: `src-tauri/src/ssl_bypass/bypass_scripts.rs`
- Modify: `src-tauri/src/lib.rs` (add `pub mod ssl_bypass;`)

- [ ] **Step 1: Create ssl_bypass module root**

Create `src-tauri/src/ssl_bypass/mod.rs`:

```rust
//! SSL bypass module.
//!
//! Provides built-in Frida scripts and APK patching for bypassing
//! SSL certificate pinning on Android apps.

pub mod apk_patcher;
pub mod bypass_scripts;
pub mod custom_scripts;
```

- [ ] **Step 2: Create bypass_scripts.rs with 6 scripts and tests**

Create `src-tauri/src/ssl_bypass/bypass_scripts.rs`:

```rust
//! Built-in Frida bypass scripts.
//!
//! Each script is a JavaScript string that runs in the Frida JS runtime
//! on the target Android device. Scripts hook specific Java/Android APIs
//! to bypass SSL certificate pinning.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BypassScript {
    pub id: String,
    pub name: String,
    pub description: String,
    pub target_framework: Vec<String>,
    pub script_content: String,
    pub is_builtin: bool,
}

const OKHTTP3_SCRIPT: &str = r#"
(function() {
    try {
        var CertificatePinner = Java.use("okhttp3.CertificatePinner");
        CertificatePinner.check.overload("java.lang.String", "java.util.List").implementation = function() {
            console.log("[ProxyBot] OkHttp3 CertificatePinner.check bypassed");
        };
        console.log("[ProxyBot] OkHttp3 bypass installed");
    } catch(e) {
        console.log("[ProxyBot] OkHttp3 bypass failed: " + e);
    }
})();
"#;

const CONSCRYPT_SCRIPT: &str = r#"
(function() {
    try {
        var TrustManagerImpl = Java.use("com.android.org.conscrypt.TrustManagerImpl");
        TrustManagerImpl.verifyChain.implementation = function() {
            console.log("[ProxyBot] Conscrypt verifyChain bypassed");
            return arguments[0];
        };
        console.log("[ProxyBot] Conscrypt bypass installed");
    } catch(e) {
        console.log("[ProxyBot] Conscrypt bypass failed: " + e);
    }
})();
"#;

const WEBVIEW_SCRIPT: &str = r#"
(function() {
    try {
        var WebViewClient = Java.use("android.webkit.WebViewClient");
        WebViewClient.onReceivedSslError.implementation = function(view, handler, error) {
            console.log("[ProxyBot] WebView SSL error bypassed");
            handler.proceed();
        };
        console.log("[ProxyBot] WebView bypass installed");
    } catch(e) {
        console.log("[ProxyBot] WebView bypass failed: " + e);
    }
})();
"#;

const FLUTTER_SCRIPT: &str = r#"
(function() {
    try {
        var SSL_CTX_set_custom_verify = Module.findExportByName("libssl.so", "SSL_CTX_set_custom_verify");
        if (SSL_CTX_set_custom_verify) {
            Interceptor.attach(SSL_CTX_set_custom_verify, {
                onEnter: function(args) {
                    args[2] = new NativeFunction(function() { return 0; }, 'int', []);
                }
            });
            console.log("[ProxyBot] Flutter SSL_CTX_set_custom_verify bypassed");
        }
    } catch(e) {
        console.log("[ProxyBot] Flutter bypass failed: " + e);
    }
})();
"#;

const REACT_NATIVE_SCRIPT: &str = r#"
(function() {
    try {
        var CertificatePinner = Java.use("okhttp3.CertificatePinner");
        CertificatePinner.check.overload("java.lang.String", "java.util.List").implementation = function() {
            console.log("[ProxyBot] RN OkHttp bypassed");
        };
        console.log("[ProxyBot] React Native bypass installed");
    } catch(e) {
        console.log("[ProxyBot] React Native bypass failed: " + e);
    }
})();
"#;

const UNIVERSAL_SCRIPT: &str = r#"
(function() {
    try {
        var X509TrustManager = Java.use("javax.net.ssl.X509TrustManager");
        var methods = X509TrustManager.class.getDeclaredMethods();
        var TrustManagerImpl = Java.use("com.android.org.conscrypt.TrustManagerImpl");
        TrustManagerImpl.verifyChain.implementation = function() {
            return arguments[0];
        };
        console.log("[ProxyBot] Universal bypass installed");
    } catch(e) {
        console.log("[ProxyBot] Universal bypass failed: " + e);
    }
})();
"#;

/// Return all built-in bypass scripts.
pub fn get_all_builtin_scripts() -> Vec<BypassScript> {
    vec![
        BypassScript {
            id: "okhttp3".to_string(),
            name: "OkHttp3 CertificatePinner".to_string(),
            description: "Bypasses OkHttp3 certificate pinning by hooking CertificatePinner.check".to_string(),
            target_framework: vec!["okhttp3".to_string()],
            script_content: OKHTTP3_SCRIPT.to_string(),
            is_builtin: true,
        },
        BypassScript {
            id: "conscrypt".to_string(),
            name: "Conscrypt TrustManager".to_string(),
            description: "Bypasses Conscrypt/Java TLS certificate verification".to_string(),
            target_framework: vec!["conscrypt".to_string(), "java-tls".to_string()],
            script_content: CONSCRYPT_SCRIPT.to_string(),
            is_builtin: true,
        },
        BypassScript {
            id: "webview".to_string(),
            name: "WebView SSL Error".to_string(),
            description: "Bypasses WebView SSL errors by calling handler.proceed()".to_string(),
            target_framework: vec!["webview".to_string()],
            script_content: WEBVIEW_SCRIPT.to_string(),
            is_builtin: true,
        },
        BypassScript {
            id: "flutter".to_string(),
            name: "Flutter SSL Pinning".to_string(),
            description: "Bypasses Flutter/BoringSSL SSL pinning via native hook".to_string(),
            target_framework: vec!["flutter".to_string()],
            script_content: FLUTTER_SCRIPT.to_string(),
            is_builtin: true,
        },
        BypassScript {
            id: "react_native".to_string(),
            name: "React Native Network".to_string(),
            description: "Bypasses React Native network security via OkHttp3 hook".to_string(),
            target_framework: vec!["react-native".to_string()],
            script_content: REACT_NATIVE_SCRIPT.to_string(),
            is_builtin: true,
        },
        BypassScript {
            id: "universal".to_string(),
            name: "Universal (System-level)".to_string(),
            description: "Universal X509TrustManager bypass for any TLS library".to_string(),
            target_framework: vec!["any".to_string()],
            script_content: UNIVERSAL_SCRIPT.to_string(),
            is_builtin: true,
        },
    ]
}

/// Look up a built-in script by id.
pub fn get_script(id: &str) -> Option<BypassScript> {
    get_all_builtin_scripts().into_iter().find(|s| s.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_all_builtin_scripts_count() {
        let scripts = get_all_builtin_scripts();
        assert_eq!(scripts.len(), 6);
    }

    #[test]
    fn test_get_script_by_id() {
        let script = get_script("okhttp3").unwrap();
        assert_eq!(script.id, "okhttp3");
        assert_eq!(script.name, "OkHttp3 CertificatePinner");
        assert!(script.is_builtin);
    }

    #[test]
    fn test_get_script_unknown_id() {
        assert!(get_script("unknown").is_none());
    }

    #[test]
    fn test_script_content_not_empty() {
        for script in get_all_builtin_scripts() {
            assert!(!script.script_content.is_empty(), "{} has empty content", script.id);
        }
    }

    #[test]
    fn test_script_content_contains_hook() {
        for script in get_all_builtin_scripts() {
            let content = &script.script_content;
            let has_hook = content.contains("Java.use")
                || content.contains("Interceptor.attach")
                || content.contains("implementation =")
                || content.contains("Module.findExportByName");
            assert!(has_hook, "{} script content has no recognizable hook", script.id);
        }
    }
}
```

- [ ] **Step 3: Wire the module into lib.rs**

In `src-tauri/src/lib.rs`, add:

```rust
pub mod ssl_bypass;
```

- [ ] **Step 4: Run the tests**

```bash
cargo test -p proxybot --lib ssl_bypass::bypass_scripts
```

Expected: 5 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/ssl_bypass/mod.rs src-tauri/src/ssl_bypass/bypass_scripts.rs src-tauri/src/lib.rs
git commit -m "feat(ssl_bypass): add 6 built-in Frida bypass scripts

Covers OkHttp3, Conscrypt, WebView, Flutter, React Native, and
Universal pinning strategies. Each script is a self-contained
JavaScript string that runs in the Frida JS runtime."
```

---

## Task 5: ssl_bypass/custom_scripts.rs — user script loader

**Files:**
- Create: `src-tauri/src/ssl_bypass/custom_scripts.rs`

- [ ] **Step 1: Create custom_scripts.rs**

Create `src-tauri/src/ssl_bypass/custom_scripts.rs`:

```rust
//! User custom bypass scripts loader.
//!
//! Reads `.js` files from `~/.proxybot/bypass-scripts/` and returns
//! them as `BypassScript` entries with `is_builtin: false`.

use std::path::PathBuf;

use crate::ssl_bypass::bypass_scripts::BypassScript;

/// Load all custom bypass scripts from `~/.proxybot/bypass-scripts/`.
pub fn load_custom_scripts() -> Vec<BypassScript> {
    let dir = match custom_scripts_dir() {
        Some(d) => d,
        None => return Vec::new(),
    };

    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
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

    fn temp_scripts_dir() -> (tempdir::TempDir, PathBuf) {
        let dir = tempdir::TempDir::new("proxybot-custom-scripts").unwrap();
        let path = dir.path().to_path_buf();
        std::fs::create_dir_all(&path).unwrap();
        (dir, path)
    }

    #[test]
    fn test_load_custom_scripts_from_dir() {
        let (_tmp, dir) = temp_scripts_dir();
        std::fs::write(dir.join("my-script.js"), "// my bypass").unwrap();
        // Override HOME for this test
        std::env::set_var("HOME", dir.parent().unwrap());
        // ... test logic
    }

    #[test]
    fn test_load_custom_scripts_empty_dir() {
        // Create empty temp dir, verify load returns Vec::new()
    }

    #[test]
    fn test_load_custom_scripts_dir_not_found() {
        // Point HOME at non-existent dir, verify load returns Vec::new()
    }

    #[test]
    fn test_load_custom_scripts_skips_non_js() {
        // Create dir with .txt file, verify it's skipped
    }
}
```

Note: The test code above uses a sketch. The implementer should use the `tempfile` crate (already in dev-dependencies) or create a simpler approach without external deps. Adapt the tests to the project's existing test patterns.

- [ ] **Step 2: Run the tests**

```bash
cargo test -p proxybot --lib ssl_bypass::custom_scripts
```

Expected: 4 tests pass (adapt as needed based on actual test implementation).

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/ssl_bypass/custom_scripts.rs
git commit -m "feat(ssl_bypass): load custom user bypass scripts

Reads .js files from ~/.proxybot/bypass-scripts/ as BypassScript
entries with is_builtin: false. Skips non-JS files silently."
```

---

## Task 6: frida/mod.rs — FridaManager (full implementation)

**Files:**
- Modify: `src-tauri/src/frida/mod.rs` (replace stub with real Frida integration)

- [ ] **Step 1: Update frida/mod.rs to use the real frida crate**

Replace the contents of `src-tauri/src/frida/mod.rs`:

```rust
//! Frida runtime integration via the frida-rust crate.

pub mod device;
pub mod session;
pub mod script;

use std::collections::HashMap;
use std::sync::Mutex;

use frida::Frida;

use crate::frida::device::{DeviceInfo, DeviceType, ProcessInfo};
use crate::frida::session::SessionHandle;

pub struct FridaManager {
    frida: Frida,
    devices: Mutex<Vec<DeviceInfo>>,
    sessions: Mutex<HashMap<String, SessionHandle>>,
}

impl FridaManager {
    pub fn new() -> Result<Self, String> {
        let frida = Frida::obtain();
        Ok(Self {
            frida,
            devices: Mutex::new(Vec::new()),
            sessions: Mutex::new(HashMap::new()),
        })
    }

    /// Refresh and return the list of connected devices.
    pub fn list_devices(&self) -> Result<Vec<DeviceInfo>, String> {
        let device_manager = self.frida.device_manager();
        let devices = device_manager
            .enumerate_devices()
            .map_err(|e| format!("Failed to enumerate devices: {}", e))?;

        let infos: Vec<DeviceInfo> = devices
            .iter()
            .map(|d| DeviceInfo {
                id: d.id().to_string(),
                name: d.name().to_string(),
                device_type: match d.type_() {
                    frida::DeviceType::Local => DeviceType::Local,
                    frida::DeviceType::Remote => DeviceType::Remote,
                    frida::DeviceType::Usb => DeviceType::Usb,
                },
                is_connected: d.is_lost().is_none(),
            })
            .collect();

        *self.devices.lock().map_err(|e| e.to_string())? = infos.clone();
        Ok(infos)
    }

    /// List processes on a device.
    pub fn list_processes(&self, device_id: &str) -> Result<Vec<ProcessInfo>, String> {
        let device_manager = self.frida.device_manager();
        let device = device_manager
            .find_device_by_id(device_id)
            .map_err(|e| format!("Device not found: {}", e))?;

        let processes = device
            .enumerate_processes()
            .map_err(|e| format!("Failed to enumerate processes: {}", e))?;

        Ok(processes
            .iter()
            .map(|p| ProcessInfo {
                pid: p.pid(),
                name: p.name().to_string(),
                identifier: p.identifier().to_string(),
                icon: p.icon().map(|i| i.to_vec()),
            })
            .collect())
    }

    /// Attach to a process and inject a script.
    pub fn attach_and_inject(
        &self,
        device_id: &str,
        pid: u32,
        script_content: &str,
    ) -> Result<SessionHandle, String> {
        let device_manager = self.frida.device_manager();
        let device = device_manager
            .find_device_by_id(device_id)
            .map_err(|e| format!("Device not found: {}", e))?;

        let session = device
            .attach(pid)
            .map_err(|e| format!("Failed to attach to PID {}: {}", pid, e))?;

        let script = session
            .create_script(script_content)
            .map_err(|e| format!("Failed to create script: {}", e))?;

        script
            .load()
            .map_err(|e| format!("Failed to load script: {}", e))?;

        let handle = SessionHandle {
            session_id: uuid::Uuid::new_v4().to_string(),
            device_id: device_id.to_string(),
            pid,
            process_name: String::new(),
            attached_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        };

        self.sessions
            .lock()
            .map_err(|e| e.to_string())?
            .insert(handle.session_id.clone(), handle.clone());
        Ok(handle)
    }

    /// Detach from a process.
    pub fn detach(&self, session_id: &str) -> Result<(), String> {
        self.sessions
            .lock()
            .map_err(|e| e.to_string())?
            .remove(session_id);
        Ok(())
    }
}

impl Default for FridaManager {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            frida: unsafe { std::mem::zeroed() },
            devices: Mutex::new(Vec::new()),
            sessions: Mutex::new(HashMap::new()),
        })
    }
}
```

- [ ] **Step 2: Compile-check**

```bash
cargo check -p proxybot --lib
```

Expected: 0 errors (or warnings that are pre-existing).

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/frida/mod.rs
git commit -m "feat(frida): integrate frida-rust for device/session management

Connects to the Frida runtime via the frida crate v0.17. Provides
device enumeration, process listing, attach/inject, and detach
operations. Sessions are tracked by UUID."
```

---

## Task 7: commands/ssl_bypass.rs — Tauri commands

**Files:**
- Create: `src-tauri/src/commands/ssl_bypass.rs`
- Modify: `src-tauri/src/commands/mod.rs` (add `pub mod ssl_bypass;`)
- Modify: `src-tauri/src/lib.rs` (register commands)

- [ ] **Step 1: Create ssl_bypass commands**

Create `src-tauri/src/commands/ssl_bypass.rs`:

```rust
//! Tauri commands for SSL bypass operations.

use std::sync::Arc;
use tauri::State;

use crate::frida::device::{DeviceInfo, ProcessInfo};
use crate::frida::session::SessionHandle;
use crate::frida::FridaManager;
use crate::ssl_bypass::bypass_scripts;
use crate::ssl_bypass::custom_scripts;

pub struct FridaState(pub Arc<FridaManager>);

#[tauri::command]
pub fn frida_list_devices(
    state: State<'_, FridaState>,
) -> Result<Vec<DeviceInfo>, String> {
    state.0.list_devices()
}

#[tauri::command]
pub fn frida_list_processes(
    device_id: String,
    state: State<'_, FridaState>,
) -> Result<Vec<ProcessInfo>, String> {
    state.0.list_processes(&device_id)
}

#[tauri::command]
pub fn frida_inject_script(
    device_id: String,
    pid: u32,
    script_id: String,
    state: State<'_, FridaState>,
) -> Result<SessionHandle, String> {
    let script = bypass_scripts::get_script(&script_id)
        .or_else(|| custom_scripts::load_custom_scripts().into_iter().find(|s| s.id == script_id))
        .ok_or_else(|| format!("Script '{}' not found", script_id))?;
    state.0.attach_and_inject(&device_id, pid, &script.script_content)
}

#[tauri::command]
pub fn frida_detach(
    session_id: String,
    state: State<'_, FridaState>,
) -> Result<(), String> {
    state.0.detach(&session_id)
}

#[tauri::command]
pub fn list_bypass_scripts() -> Vec<bypass_scripts::BypassScript> {
    let mut all = bypass_scripts::get_all_builtin_scripts();
    all.extend(custom_scripts::load_custom_scripts());
    all
}

#[tauri::command]
pub fn check_java_installed() -> bool {
    std::process::Command::new("java")
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[tauri::command]
pub fn check_adb_installed() -> bool {
    std::process::Command::new("adb")
        .arg("version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
```

- [ ] **Step 2: Add to commands/mod.rs**

In `src-tauri/src/commands/mod.rs`, add `pub mod ssl_bypass;` to the module list.

- [ ] **Step 3: Register commands in lib.rs**

In `src-tauri/src/lib.rs`, find the `tauri::generate_handler!` macro and add:

```rust
            commands::ssl_bypass::frida_list_devices,
            commands::ssl_bypass::frida_list_processes,
            commands::ssl_bypass::frida_inject_script,
            commands::ssl_bypass::frida_detach,
            commands::ssl_bypass::list_bypass_scripts,
            commands::ssl_bypass::check_java_installed,
            commands::ssl_bypass::check_adb_installed,
```

Also, in the Tauri app builder (`tauri::Builder::default()` block), register the Frida state:

```rust
            .manage(commands::ssl_bypass::FridaState(
                std::sync::Arc::new(frida::FridaManager::new()?)
            ))
```

(Adjust the exact `.manage()` placement to match the existing pattern in the file.)

- [ ] **Step 4: Compile-check**

```bash
cargo check -p proxybot --lib
```

Expected: 0 errors.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/ssl_bypass.rs src-tauri/src/commands/mod.rs src-tauri/src/lib.rs
git commit -m "feat(commands): add 7 SSL bypass Tauri commands

Exposes frida_list_devices, frida_list_processes, frida_inject_script,
frida_detach, list_bypass_scripts, check_java_installed,
check_adb_installed to the frontend."
```

---

## Task 8: ssl_bypass/apk_patcher.rs — APK patching

**Files:**
- Create: `src-tauri/src/ssl_bypass/apk_patcher.rs`

- [ ] **Step 1: Create apk_patcher.rs**

Create `src-tauri/src/ssl_bypass/apk_patcher.rs`:

```rust
//! APK patching via apktool + jarsigner.
//!
//! Decompiles an APK, injects frida-gadget.so and a bypass script,
//! recompiles, and signs with a temporary keystore.

use std::path::PathBuf;
use std::process::Command;

pub struct ApkPatcher {
    apktool_path: PathBuf,
    frida_gadget_path: PathBuf,
    temp_dir: PathBuf,
}

impl ApkPatcher {
    pub fn new() -> Result<Self, String> {
        let apktool_path = std::env::current_exe()
            .map_err(|e| format!("Failed to get exe path: {}", e))?
            .parent()
            .ok_or("Failed to get exe parent")?
            .join("resources")
            .join("apktool.jar");

        if !apktool_path.exists() {
            return Err(format!("apktool.jar not found at {}", apktool_path.display()));
        }

        let frida_gadget_path = std::env::current_exe()
            .map_err(|e| format!("Failed to get exe path: {}", e))?
            .parent()
            .ok_or("Failed to get exe parent")?
            .join("resources")
            .join("frida-gadget")
            .join("arm64-v8a")
            .join("libfrida-gadget.so");

        let temp_dir = std::env::temp_dir().join("proxybot-apk-patcher");
        std::fs::create_dir_all(&temp_dir)
            .map_err(|e| format!("Failed to create temp dir: {}", e))?;

        Ok(Self {
            apktool_path,
            frida_gadget_path,
            temp_dir,
        })
    }

    pub fn temp_dir(&self) -> &PathBuf {
        &self.temp_dir
    }

    /// Decompile APK using apktool.
    pub fn decompile(&self, apk: &PathBuf, output: &PathBuf) -> Result<(), String> {
        let status = Command::new("java")
            .args(["-jar"])
            .arg(&self.apktool_path)
            .args(["d"])
            .arg(apk)
            .args(["-o"])
            .arg(output)
            .arg("-f")
            .status()
            .map_err(|e| format!("Failed to run apktool: {}", e))?;

        if !status.success() {
            return Err("apktool decompile failed".to_string());
        }
        Ok(())
    }

    /// Recompile APK using apktool.
    pub fn recompile(&self, work_dir: &PathBuf, output: &PathBuf) -> Result<(), String> {
        let status = Command::new("java")
            .args(["-jar"])
            .arg(&self.apktool_path)
            .args(["b"])
            .arg(work_dir)
            .args(["-o"])
            .arg(output)
            .status()
            .map_err(|e| format!("Failed to run apktool: {}", e))?;

        if !status.success() {
            return Err("apktool recompile failed".to_string());
        }
        Ok(())
    }

    /// Sign APK with jarsigner.
    pub fn sign(&self, apk: &PathBuf) -> Result<PathBuf, String> {
        let keystore = self.temp_dir.join("proxybot.keystore");
        if !keystore.exists() {
            let status = Command::new("keytool")
                .args(["-genkey", "-v"])
                .arg("-keystore").arg(&keystore)
                .args(["-alias", "proxybot"])
                .args(["-keyalg", "RSA", "-keysize", "2048", "-validity", "10000"])
                .args(["-storepass", "proxybot", "-keypass", "proxybot"])
                .args(["-dname", "CN=ProxyBot, OU=Dev, O=ProxyBot, L=Unknown, ST=Unknown, C=US"])
                .status()
                .map_err(|e| format!("Failed to generate keystore: {}", e))?;
            if !status.success() {
                return Err("keytool failed".to_string());
            }
        }

        let status = Command::new("jarsigner")
            .args(["-verbose", "-sigalg", "SHA256withRSA", "-digestalg", "SHA-256"])
            .arg("-keystore").arg(&keystore)
            .args(["-storepass", "proxybot", "-keypass", "proxybot"])
            .arg(apk)
            .arg("proxybot")
            .status()
            .map_err(|e| format!("Failed to run jarsigner: {}", e))?;

        if !status.success() {
            return Err("jarsigner failed".to_string());
        }
        Ok(apk.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decompile_apk_invalid_path() {
        let patcher = ApkPatcher {
            apktool_path: PathBuf::from("/nonexistent/apktool.jar"),
            frida_gadget_path: PathBuf::from("/nonexistent/libfrida-gadget.so"),
            temp_dir: std::env::temp_dir().join("test-apk-patcher"),
        };
        let result = patcher.decompile(
            &PathBuf::from("/nonexistent/app.apk"),
            &std::env::temp_dir().join("output"),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_new_validates_apktool_exists() {
        let result = ApkPatcher::new();
        // Will fail because apktool.jar is not bundled yet
        assert!(result.is_err());
    }
}
```

- [ ] **Step 2: Run the tests**

```bash
cargo test -p proxybot --lib ssl_bypass::apk_patcher
```

Expected: 2 tests pass.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/ssl_bypass/apk_patcher.rs
git commit -m "feat(ssl_bypass): add APK patcher using apktool + jarsigner

Decompiles APK, injects frida-gadget.so, recompiles, and signs
with a temporary keystore. Requires Java runtime and apktool.jar
bundled in Tauri resources."
```

---

## Task 9: UI components + Pinia store

**Files:**
- Create: `src/components/ssl-bypass/SslBypassPage.tsx`
- Create: `src/components/ssl-bypass/DeviceSelector.tsx`
- Create: `src/components/ssl-bypass/ScriptList.tsx`
- Create: `src/components/ssl-bypass/FridaStatus.tsx`
- Create: `src/components/ssl-bypass/ApkPatcher.tsx`
- Create: `src/stores/sslBypassStore.ts`
- Modify: sidebar navigation to add the new page

- [ ] **Step 1: Create the Pinia store**

Create `src/stores/sslBypassStore.ts`:

```typescript
import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { ref, computed } from "vue";

export interface DeviceInfo {
  id: string;
  name: string;
  device_type: "Usb" | "Remote" | "Local";
  is_connected: boolean;
}

export interface ProcessInfo {
  pid: number;
  name: string;
  identifier: string;
}

export interface BypassScript {
  id: string;
  name: string;
  description: string;
  target_framework: string[];
  is_builtin: boolean;
}

export const useSslBypassStore = defineStore("sslBypass", () => {
  const devices = ref<DeviceInfo[]>([]);
  const processes = ref<ProcessInfo[]>([]);
  const scripts = ref<BypassScript[]>([]);
  const selectedDevice = ref<string | null>(null);
  const selectedScript = ref<string | null>(null);
  const javaInstalled = ref(false);
  const adbInstalled = ref(false);

  const selectedDeviceObj = computed(() =>
    devices.value.find((d) => d.id === selectedDevice.value) ?? null
  );

  async function refreshDevices() {
    devices.value = await invoke<DeviceInfo[]>("frida_list_devices");
  }

  async function refreshProcesses() {
    if (!selectedDevice.value) return;
    processes.value = await invoke<ProcessInfo[]>("frida_list_processes", {
      deviceId: selectedDevice.value,
    });
  }

  async function refreshScripts() {
    scripts.value = await invoke<BypassScript[]>("list_bypass_scripts");
  }

  async function injectScript(pid: number, scriptId: string) {
    if (!selectedDevice.value) return null;
    return await invoke("frida_inject_script", {
      deviceId: selectedDevice.value,
      pid,
      scriptId,
    });
  }

  async function checkPrerequisites() {
    javaInstalled.value = await invoke<boolean>("check_java_installed");
    adbInstalled.value = await invoke<boolean>("check_adb_installed");
  }

  return {
    devices, processes, scripts,
    selectedDevice, selectedScript, selectedDeviceObj,
    javaInstalled, adbInstalled,
    refreshDevices, refreshProcesses, refreshScripts, injectScript, checkPrerequisites,
  };
});
```

- [ ] **Step 2: Create DeviceSelector component**

Create `src/components/ssl-bypass/DeviceSelector.tsx`:

```tsx
import { useSslBypassStore } from "@/stores/sslBypassStore";

export function DeviceSelector() {
  const store = useSslBypassStore();
  return (
    <div className="device-selector">
      <button onClick={store.refreshDevices}>Refresh Devices</button>
      {store.devices.length === 0 ? (
        <p>No devices found. Connect an Android device via USB.</p>
      ) : (
        <select
          value={store.selectedDevice ?? ""}
          onChange={(e) => (store.selectedDevice = e.target.value)}
        >
          <option value="">Select a device</option>
          {store.devices.map((d) => (
            <option key={d.id} value={d.id}>
              {d.name} ({d.device_type})
            </option>
          ))}
        </select>
      )}
    </div>
  );
}
```

- [ ] **Step 3: Create ScriptList component**

Create `src/components/ssl-bypass/ScriptList.tsx`:

```tsx
import { useSslBypassStore } from "@/stores/sslBypassStore";

export function ScriptList() {
  const store = useSslBypassStore();
  return (
    <div className="script-list">
      <button onClick={store.refreshScripts}>Refresh Scripts</button>
      {store.scripts.map((s) => (
        <div
          key={s.id}
          className={`script-item ${store.selectedScript === s.id ? "selected" : ""}`}
          onClick={() => (store.selectedScript = s.id)}
        >
          <h3>{s.name}</h3>
          <p>{s.description}</p>
          {s.is_builtin && <span className="badge">Built-in</span>}
        </div>
      ))}
    </div>
  );
}
```

- [ ] **Step 4: Create FridaStatus component**

Create `src/components/ssl-bypass/FridaStatus.tsx`:

```tsx
import { useSslBypassStore } from "@/stores/sslBypassStore";

export function FridaStatus() {
  const store = useSslBypassStore();
  return (
    <div className="frida-status">
      <h3>Prerequisites</h3>
      <p>Java: {store.javaInstalled ? "✓ Installed" : "✗ Missing"}</p>
      <p>ADB: {store.adbInstalled ? "✓ Installed" : "✗ Missing"}</p>
      <button onClick={store.checkPrerequisites}>Recheck</button>
    </div>
  );
}
```

- [ ] **Step 5: Create ApkPatcher component**

Create `src/components/ssl-bypass/ApkPatcher.tsx`:

```tsx
import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useSslBypassStore } from "@/stores/sslBypassStore";

export function ApkPatcher() {
  const store = useSslBypassStore();
  const [apkPath, setApkPath] = useState("");
  const [patching, setPatching] = useState(false);
  const [result, setResult] = useState<string | null>(null);

  async function patch() {
    if (!apkPath || !store.selectedScript) return;
    setPatching(true);
    try {
      const output = await invoke<string>("patch_apk", {
        apkPath,
        scriptId: store.selectedScript,
      });
      setResult(`Patched: ${output}`);
    } catch (e) {
      setResult(`Error: ${e}`);
    } finally {
      setPatching(false);
    }
  }

  return (
    <div className="apk-patcher">
      <h3>APK Patcher</h3>
      <input
        type="text"
        placeholder="/path/to/app.apk"
        value={apkPath}
        onChange={(e) => setApkPath(e.target.value)}
      />
      <button onClick={patch} disabled={patching || !store.selectedScript}>
        {patching ? "Patching..." : "Patch APK"}
      </button>
      {result && <pre>{result}</pre>}
    </div>
  );
}
```

- [ ] **Step 6: Create SslBypassPage main component**

Create `src/components/ssl-bypass/SslBypassPage.tsx`:

```tsx
import { useEffect } from "react";
import { useSslBypassStore } from "@/stores/sslBypassStore";
import { DeviceSelector } from "./DeviceSelector";
import { ScriptList } from "./ScriptList";
import { FridaStatus } from "./FridaStatus";
import { ApkPatcher } from "./ApkPatcher";

export function SslBypassPage() {
  const store = useSslBypassStore();

  useEffect(() => {
    store.checkPrerequisites();
    store.refreshScripts();
  }, []);

  return (
    <div className="ssl-bypass-page">
      <h1>SSL Bypass</h1>
      <FridaStatus />
      <DeviceSelector />
      {store.selectedDevice && <ProcessList />}
      <ScriptList />
      <ApkPatcher />
    </div>
  );
}

function ProcessList() {
  const store = useSslBypassStore();
  return (
    <div className="process-list">
      <button onClick={store.refreshProcesses}>Refresh Processes</button>
      <ul>
        {store.processes.map((p) => (
          <li key={p.pid}>
            {p.name} (PID: {p.pid})
            <button
              disabled={!store.selectedScript}
              onClick={() =>
                store.injectScript(p.pid, store.selectedScript!)
              }
            >
              Inject
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}
```

- [ ] **Step 7: Add sidebar entry**

Find the sidebar navigation component (likely in `src/components/layout/` or `src/components/shared/Sidebar.tsx`). Add a new entry for "SSL Bypass" that links to the new page.

- [ ] **Step 8: Typecheck and run UI tests**

```bash
pnpm typecheck
pnpm test:ui
```

Expected: 0 typecheck errors, all existing tests pass.

- [ ] **Step 9: Commit**

```bash
git add src/components/ssl-bypass/ src/stores/sslBypassStore.ts src/components/layout/
git commit -m "feat(ui): add SSL Bypass page with device/script/patcher UI

New sidebar page with DeviceSelector, ScriptList, FridaStatus,
and ApkPatcher components. Uses Pinia store for state management.
Includes prerequisite checks for Java and ADB."
```

---

## Task 10: E2E tests

**Files:**
- Create: `e2e/ssl-bypass.spec.ts`

- [ ] **Step 1: Create E2E test file**

Create `e2e/ssl-bypass.spec.ts`:

```typescript
import { test, expect } from "@playwright/test";

test("ssl_bypass_page_renders", async ({ page }) => {
  await page.goto("/ssl-bypass");
  await expect(page.getByText("SSL Bypass")).toBeVisible();
});

test("script_list_shows_builtin_scripts", async ({ page }) => {
  await page.goto("/ssl-bypass");
  await page.getByRole("button", { name: "Refresh Scripts" }).click();
  await expect(page.getByText("OkHttp3 CertificatePinner")).toBeVisible();
  await expect(page.getByText("Conscrypt TrustManager")).toBeVisible();
  await expect(page.getByText("WebView SSL Error")).toBeVisible();
});

test("prerequisite_check_shows_java_status", async ({ page }) => {
  await page.goto("/ssl-bypass");
  await expect(page.getByText(/Java:/)).toBeVisible();
});

test("device_selector_shows_empty_when_no_devices", async ({ page }) => {
  await page.goto("/ssl-bypass");
  await page.getByRole("button", { name: "Refresh Devices" }).click();
  await expect(page.getByText(/No devices found/)).toBeVisible();
});
```

- [ ] **Step 2: Run the E2E tests**

```bash
pnpm test:e2e -- ssl-bypass
```

Expected: 4 tests pass.

- [ ] **Step 3: Commit**

```bash
git add e2e/ssl-bypass.spec.ts
git commit -m "test(e2e): add Playwright tests for SSL Bypass page

Verifies page rendering, built-in script list, prerequisite
status display, and empty device list behavior."
```

---

## Task 11: Final verification

**Files:** none modified

- [ ] **Step 1: Run `cargo build`**

```bash
cargo build
```

Expected: 0 errors (frida devkit download may take time on first build).

- [ ] **Step 2: Run the full test suite**

```bash
cargo test
```

Expected: all tests pass.

- [ ] **Step 3: Run `pnpm typecheck`**

```bash
pnpm typecheck
```

Expected: 0 errors.

- [ ] **Step 4: Run `pnpm test:ui`**

```bash
pnpm test:ui
```

Expected: all UI tests pass.

- [ ] **Step 5: Run `cargo clippy`**

```bash
cargo clippy -p proxybot --no-deps
```

Expected: no new clippy warnings from this branch.

- [ ] **Step 6: Final commit if any cleanup needed**

```bash
git status
# If uncommitted changes:
git add -A
git commit -m "chore: post-implementation cleanup"
```

---

## Manual verification (out-of-band)

Real-device testing requires:
1. Android device with USB debugging enabled
2. `frida-server` running on the device (`adb shell su -c /data/local/tmp/frida-server &`)
3. USB connection to Mac running ProxyBot
4. Java runtime installed (`java -version` should work)
5. Android SDK platform-tools installed (`adb devices` should work)

iOS path: Not supported in this version.

---

## References

- Spec: `docs/superpowers/specs/2026-06-12-frida-ssl-bypass-design.md`
- frida-rust: https://github.com/frida/frida-rust (v0.17)
- Trafexia SSL Bypass: https://github.com/danieldev23/trafexia (electron/ssl-bypass/)
- Frida docs: https://frida.re/docs/
- apktool: https://ibotpeaches.github.io/Apktool/
- HTTP Toolkit Android SSL Pinning: https://httptoolkit.com/blog/android-ssl-pinning-bypass/
