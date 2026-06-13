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

    /// Inject the Frida Gadget .so into the decompiled APK's lib/ directory.
    ///
    /// For arm64-v8a (the most common); other architectures can be added
    /// later by extending this method.
    pub fn inject_frida_gadget(&self, work_dir: &PathBuf) -> Result<(), String> {
        let lib_dir = work_dir.join("lib").join("arm64-v8a");
        std::fs::create_dir_all(&lib_dir)
            .map_err(|e| format!("Failed to create lib dir: {}", e))?;

        let gadget_dest = lib_dir.join("libfrida-gadget.so");
        std::fs::copy(&self.frida_gadget_path, &gadget_dest).map_err(|e| {
            format!(
                "Failed to copy frida-gadget from {} to {}: {}. \
                 The gadget .so may not be bundled — see docs on APK patching prerequisites.",
                self.frida_gadget_path.display(),
                gadget_dest.display(),
                e
            )
        })?;

        Ok(())
    }

    /// Embed the bypass script into the decompiled APK's assets/ directory.
    pub fn embed_script(&self, work_dir: &PathBuf, script_content: &str) -> Result<(), String> {
        let assets_dir = work_dir.join("assets");
        std::fs::create_dir_all(&assets_dir)
            .map_err(|e| format!("Failed to create assets dir: {}", e))?;

        let script_path = assets_dir.join("frida-bypass.js");
        std::fs::write(&script_path, script_content)
            .map_err(|e| format!("Failed to write bypass script: {}", e))?;

        Ok(())
    }

    /// Modify AndroidManifest.xml to add INTERNET permission and Frida
    /// GadgetLoader content provider.
    ///
    /// The manifest is read as plain text (apktool's binary form is not
    /// used here because the format is well-defined for these specific
    /// insertions).
    pub fn modify_manifest(&self, work_dir: &PathBuf) -> Result<(), String> {
        let manifest_path = work_dir.join("AndroidManifest.xml");
        let mut content = std::fs::read_to_string(&manifest_path)
            .map_err(|e| format!("Failed to read AndroidManifest.xml: {}", e))?;

        // Add INTERNET permission if not present
        if !content.contains("android.permission.INTERNET") {
            // Insert before <application> tag (or after <manifest> if no
            // <application> at start)
            if let Some(pos) = content.find("<application") {
                content.insert_str(
                    pos,
                    "<uses-permission android:name=\"android.permission.INTERNET\"/>\n",
                );
            } else if let Some(pos) = content.find("<manifest") {
                if let Some(close_pos) = content[pos..].find('>') {
                    let insert_pos = pos + close_pos + 1;
                    content.insert_str(
                        insert_pos,
                        "\n<uses-permission android:name=\"android.permission.INTERNET\"/>",
                    );
                }
            }
        }

        // Add Frida GadgetLoader content provider if not present
        if !content.contains("GadgetLoader") {
            let provider = "\n<provider \
                android:name=\"com.frida.gadget.GadgetLoader\" \
                android:authorities=\"${applicationId}.gadget.provider\" \
                android:exported=\"false\" \
                android:directBootAware=\"true\" />";

            if let Some(pos) = content.find("</application>") {
                content.insert_str(pos, provider);
            }
        }

        std::fs::write(&manifest_path, content)
            .map_err(|e| format!("Failed to write AndroidManifest.xml: {}", e))?;

        Ok(())
    }

    /// Full APK patching pipeline:
    /// 1. Decompile with apktool
    /// 2. Inject frida-gadget.so
    /// 3. Embed bypass script
    /// 4. Modify AndroidManifest.xml
    /// 5. Recompile
    /// 6. Sign
    ///
    /// Returns the path to the patched-signed APK.
    pub fn patch_apk(&self, apk_path: &str, script_content: &str) -> Result<String, String> {
        let apk = PathBuf::from(apk_path);
        if !apk.exists() {
            return Err(format!("APK not found: {}", apk_path));
        }

        let work_dir = self.temp_dir.join("work");
        if work_dir.exists() {
            std::fs::remove_dir_all(&work_dir)
                .map_err(|e| format!("Failed to clean work dir: {}", e))?;
        }

        // Step 1: Decompile
        self.decompile(&apk, &work_dir)?;

        // Step 2-4: Modify
        self.inject_frida_gadget(&work_dir)?;
        self.embed_script(&work_dir, script_content)?;
        self.modify_manifest(&work_dir)?;

        // Step 5: Recompile
        let recompiled_apk = self.temp_dir.join("patched.apk");
        self.recompile(&work_dir, &recompiled_apk)?;

        // Step 6: Sign
        let signed_apk = self.sign(&recompiled_apk)?;

        Ok(signed_apk.to_string_lossy().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build an ApkPatcher with arbitrary paths for testing methods that
    /// don't shell out.
    fn test_patcher(work_dir: &std::path::Path) -> ApkPatcher {
        ApkPatcher {
            apktool_path: PathBuf::from("/nonexistent/apktool.jar"),
            frida_gadget_path: PathBuf::from("/nonexistent/libfrida-gadget.so"),
            temp_dir: work_dir.to_path_buf(),
        }
    }

    #[test]
    fn test_decompile_apk_invalid_path() {
        let tmp = std::env::temp_dir().join("test-apk-patcher-decompile");
        let patcher = test_patcher(&tmp);
        let result = patcher.decompile(
            &PathBuf::from("/nonexistent/app.apk"),
            &std::env::temp_dir().join("output"),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_new_succeeds_with_resources_present() {
        // apktool.jar and libfrida-gadget.so are bundled in src-tauri/resources/
        // (dev mode path via CARGO_MANIFEST_DIR). ApkPatcher::new() should
        // locate both resources and return Ok.
        let result = ApkPatcher::new();
        assert!(
            result.is_ok(),
            "ApkPatcher::new() should succeed when bundled resources are present, got: {:?}",
            result.as_ref().err()
        );

        let patcher = result.unwrap();
        assert!(
            !patcher.temp_dir.as_os_str().is_empty(),
            "temp_dir should be a valid PathBuf"
        );
        assert!(
            patcher.temp_dir.exists(),
            "temp_dir should exist on disk: {}",
            patcher.temp_dir.display()
        );
    }

    #[test]
    fn test_inject_frida_gadget_creates_lib_dir() {
        let tmp = std::env::temp_dir().join("test-apk-patcher-inject");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        // Create a dummy gadget .so to copy from
        let gadget_src = tmp.join("gadget-src.so");
        std::fs::write(&gadget_src, b"fake-gadget-bytes").unwrap();

        let work_dir = tmp.join("work");
        std::fs::create_dir_all(&work_dir).unwrap();

        let mut patcher = test_patcher(&tmp);
        patcher.frida_gadget_path = gadget_src.clone();

        patcher.inject_frida_gadget(&work_dir).unwrap();

        let dest = work_dir.join("lib").join("arm64-v8a").join("libfrida-gadget.so");
        assert!(dest.exists(), "gadget .so should be at {}", dest.display());
        let copied = std::fs::read(&dest).unwrap();
        assert_eq!(copied, b"fake-gadget-bytes");
    }

    #[test]
    fn test_inject_frida_gadget_missing_source() {
        let tmp = std::env::temp_dir().join("test-apk-patcher-inject-missing");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let work_dir = tmp.join("work");
        std::fs::create_dir_all(&work_dir).unwrap();

        let patcher = test_patcher(&tmp);
        let result = patcher.inject_frida_gadget(&work_dir);
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("frida-gadget") || msg.contains("libfrida-gadget.so"),
            "error should mention frida-gadget, got: {}",
            msg
        );
    }

    #[test]
    fn test_embed_script_writes_to_assets() {
        let tmp = std::env::temp_dir().join("test-apk-patcher-embed");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let work_dir = tmp.join("work");
        std::fs::create_dir_all(&work_dir).unwrap();

        let patcher = test_patcher(&tmp);
        let script = "Java.use('okhttp3.CertificatePinner').check.implementation = function(){};";
        patcher.embed_script(&work_dir, script).unwrap();

        let dest = work_dir.join("assets").join("frida-bypass.js");
        assert!(dest.exists(), "bypass script should be at {}", dest.display());
        let written = std::fs::read_to_string(&dest).unwrap();
        assert_eq!(written, script);
    }

    #[test]
    fn test_modify_manifest_adds_internet_permission() {
        let tmp = std::env::temp_dir().join("test-apk-patcher-manifest-perm");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let work_dir = tmp.join("work");
        std::fs::create_dir_all(&work_dir).unwrap();

        let manifest = work_dir.join("AndroidManifest.xml");
        std::fs::write(
            &manifest,
            r#"<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="com.example.app">
  <application android:label="Example">
    <activity android:name=".Main"/>
  </application>
</manifest>"#,
        )
        .unwrap();

        let patcher = test_patcher(&tmp);
        patcher.modify_manifest(&work_dir).unwrap();

        let updated = std::fs::read_to_string(&manifest).unwrap();
        assert!(
            updated.contains("android.permission.INTERNET"),
            "manifest should now contain INTERNET permission: {}",
            updated
        );
        // INTERNET must come before <application>
        let perm_pos = updated.find("android.permission.INTERNET").unwrap();
        let app_pos = updated.find("<application").unwrap();
        assert!(
            perm_pos < app_pos,
            "INTERNET permission must appear before <application>"
        );
    }

    #[test]
    fn test_modify_manifest_adds_gadget_loader() {
        let tmp = std::env::temp_dir().join("test-apk-patcher-manifest-loader");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let work_dir = tmp.join("work");
        std::fs::create_dir_all(&work_dir).unwrap();

        let manifest = work_dir.join("AndroidManifest.xml");
        std::fs::write(
            &manifest,
            r#"<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="com.example.app">
  <uses-permission android:name="android.permission.INTERNET"/>
  <application android:label="Example">
    <activity android:name=".Main"/>
  </application>
</manifest>"#,
        )
        .unwrap();

        let patcher = test_patcher(&tmp);
        patcher.modify_manifest(&work_dir).unwrap();

        let updated = std::fs::read_to_string(&manifest).unwrap();
        assert!(
            updated.contains("GadgetLoader"),
            "manifest should now contain GadgetLoader provider: {}",
            updated
        );
        assert!(
            updated.contains("com.frida.gadget.GadgetLoader"),
            "manifest should contain GadgetLoader fully-qualified class name"
        );
        // GadgetLoader must appear before </application>
        let loader_pos = updated.find("GadgetLoader").unwrap();
        let close_app = updated.find("</application>").unwrap();
        assert!(
            loader_pos < close_app,
            "GadgetLoader must appear before </application>"
        );
    }

    #[test]
    fn test_modify_manifest_idempotent() {
        let tmp = std::env::temp_dir().join("test-apk-patcher-manifest-idem");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let work_dir = tmp.join("work");
        std::fs::create_dir_all(&work_dir).unwrap();

        let manifest = work_dir.join("AndroidManifest.xml");
        std::fs::write(
            &manifest,
            r#"<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="com.example.app">
  <application android:label="Example">
    <activity android:name=".Main"/>
  </application>
</manifest>"#,
        )
        .unwrap();

        let patcher = test_patcher(&tmp);
        // Run twice — second call must not duplicate entries
        patcher.modify_manifest(&work_dir).unwrap();
        let after_first = std::fs::read_to_string(&manifest).unwrap();
        patcher.modify_manifest(&work_dir).unwrap();
        let after_second = std::fs::read_to_string(&manifest).unwrap();

        assert_eq!(
            after_first, after_second,
            "modify_manifest must be idempotent; first:\n{}\nsecond:\n{}",
            after_first, after_second
        );

        // Count INTERNET occurrences — should be exactly 1
        let internet_count = after_second.matches("android.permission.INTERNET").count();
        assert_eq!(internet_count, 1, "INTERNET should appear exactly once");

        // Count GadgetLoader occurrences — should be exactly 1
        let loader_count = after_second.matches("GadgetLoader").count();
        assert_eq!(loader_count, 1, "GadgetLoader should appear exactly once");
    }

    #[test]
    fn test_modify_manifest_missing_manifest_file() {
        let tmp = std::env::temp_dir().join("test-apk-patcher-manifest-missing");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let work_dir = tmp.join("work");
        std::fs::create_dir_all(&work_dir).unwrap();

        let patcher = test_patcher(&tmp);
        let result = patcher.modify_manifest(&work_dir);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("AndroidManifest.xml"));
    }

    #[test]
    fn test_patch_apk_nonexistent_source() {
        let tmp = std::env::temp_dir().join("test-apk-patcher-patch-apk");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let patcher = test_patcher(&tmp);
        let result = patcher.patch_apk("/nonexistent/app.apk", "script");
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("APK not found"),
            "expected 'APK not found' error, got: {}",
            msg
        );
    }
}
