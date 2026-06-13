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
        // apktool.jar and frida-gadget are expected to be bundled in
        // Tauri resources. For development (cargo run), we look in
        // src-tauri/resources/. For production builds, they're in
        // the Tauri bundle alongside the binary.
        let resource_dir = std::env::current_exe()
            .map_err(|e| format!("Failed to get exe path: {}", e))?
            .parent()
            .ok_or("Failed to get exe parent")?
            .join("resources");

        // Fallback to source tree during development
        let apktool_path = if resource_dir.join("apktool.jar").exists() {
            resource_dir.join("apktool.jar")
        } else {
            // Dev: look relative to cargo manifest dir
            let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("."));
            manifest_dir.join("resources").join("apktool.jar")
        };

        if !apktool_path.exists() {
            return Err(format!("apktool.jar not found (looked in {})", apktool_path.display()));
        }

        let frida_gadget_path = if resource_dir.join("frida-gadget").exists() {
            resource_dir.join("frida-gadget").join("arm64-v8a").join("libfrida-gadget.so")
        } else {
            let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("."));
            manifest_dir.join("resources").join("frida-gadget").join("arm64-v8a").join("libfrida-gadget.so")
        };

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
        // Will fail because apktool.jar is not bundled yet
        let result = ApkPatcher::new();
        assert!(result.is_err());
    }
}
