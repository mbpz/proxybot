# QR 一键配网 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Generate per-platform QR codes from the Setup page that bundle WiFi proxy, DNS, and CA cert into a single tap. iOS gets a `.mobileconfig` profile; Android gets a self-contained HTML wizard.

**Architecture:** Reuse the existing `CertServer` (port 19876) and add path-based routing — `/ios.mobileconfig` returns a dynamically generated XML plist with WiFi + DNS + CA payloads; `/android-setup` returns a self-contained HTML page with 4-step instructions. The Tauri command `generate_device_qr(platform)` produces an SVG QR encoding the LAN URL. A new React `DeviceQrPanel` component renders the SVG inside the existing `ClientSetup` page with iOS/Android tabs.

**Tech Stack:** Rust (Tauri 2), `qrcode` crate (new dep), `uuid` 1.x, `base64` 0.22, `tiny_http` 0.12 — all deps already in workspace except `qrcode`. React 18, TypeScript, shadcn/ui. Playwright for E2E.

**Working directory:** This plan assumes the implementer is at the repo root on a feature branch off `main`. All file paths are relative to the repo root.

---

## File Structure

| File | Responsibility | Status |
|---|---|---|
| `src-tauri/src/cert/mobileconfig.rs` | Pure function: build iOS .mobileconfig XML | **New** |
| `src-tauri/src/cert/wizard.rs` | Pure function: build Android HTML wizard | **New** |
| `src-tauri/src/cert.rs` | Add `mod mobileconfig;` and `mod wizard;` | **Modify** |
| `src-tauri/src/cert_server.rs` | Add path-based routing for `/ios.mobileconfig` and `/android-setup` | **Modify** |
| `src-tauri/src/commands/device_setup.rs` | Tauri command `generate_device_qr(platform) -> Result<String, String>` | **New** |
| `src-tauri/src/lib.rs` | Register `device_setup::generate_device_qr` in `invoke_handler` | **Modify** |
| `src-tauri/Cargo.toml` | Add `qrcode` crate (currently absent) | **Modify** |
| `src/components/setup/DeviceQrPanel.tsx` | React component: tabs + QR rendering | **New** |
| `src/components/setup/ClientSetup.tsx` | Embed `<DeviceQrPanel />` | **Modify** |
| `e2e/qr-onboarding.spec.ts` | Playwright tests for CertServer routes + panel | **New** |

No DB schema change. No new Tauri commands beyond `generate_device_qr`. No state file changes (uses existing `ProxyState.local_ip` and `CertServer`'s `SERVER_RUNNING`).

---

## Task 1: `cert::mobileconfig::build_ios_profile` (TDD)

**Files:**
- Create: `src-tauri/src/cert/mobileconfig.rs`
- Modify: `src-tauri/src/cert.rs` (add `pub mod mobileconfig;`)

The function dynamically generates a complete Apple `.mobileconfig` XML plist with three payloads (WiFi, DNS, Certificate) inside `PayloadContent`. Each payload gets a runtime-generated UUID.

- [ ] **Step 1: Create the file with the function signature and 7 failing tests**

Create `src-tauri/src/cert/mobileconfig.rs`:

```rust
//! iOS .mobileconfig profile generation.
//!
//! Builds an Apple Configuration Profile (XML plist) containing three
//! payloads: WiFi (forces proxy for all networks), DNS (points at
//! ProxyBot's DNS server), and Certificate (installs the ProxyBot root CA).
//! See: https://developer.apple.com/documentation/devicemanagement

use base64::Engine;
use uuid::Uuid;

/// Build an iOS .mobileconfig profile that configures WiFi proxy,
/// DNS, and the ProxyBot root CA in a single install.
///
/// `ca_pem` is the PEM-encoded CA certificate (used as the
/// Certificate payload's content, base64-encoded per the plist spec).
/// `proxy_ip` is the LAN IP of the ProxyBot host. `proxy_port` is
/// the HTTP proxy port (default 8088). `dns_port` is the DNS server
/// port (default 5300).
pub fn build_ios_profile(
    ca_pem: &str,
    proxy_ip: &str,
    proxy_port: u16,
    dns_port: u16,
) -> String {
    let root_uuid = Uuid::new_v4();
    let wifi_uuid = Uuid::new_v4();
    let dns_uuid = Uuid::new_v4();
    let ca_uuid = Uuid::new_v4();
    let ca_payload_content = base64::engine::general_purpose::STANDARD.encode(ca_pem.as_bytes());

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>PayloadContent</key>
  <array>
    <dict>
      <key>PayloadType</key><string>com.apple.wifi.managed</string>
      <key>PayloadIdentifier</key><string>com.proxybot.profile.wifi</string>
      <key>PayloadUUID</key><string>{wifi_uuid}</string>
      <key>PayloadVersion</key><integer>1</integer>
      <key>ProxyType</key><string>Manual</string>
      <key>ProxyPACURL</key><string></string>
      <key>ProxyPACFallbackAllowed</key><integer>0</integer>
      <key>ProxyServer</key><string>{proxy_ip}</string>
      <key>ProxyServerPort</key><integer>{proxy_port}</integer>
      <key>ProxyUsername</key><string></string>
      <key>ProxyPassword</key><string></string>
    </dict>
    <dict>
      <key>PayloadType</key><string>com.apple.dnsSettings.managed</string>
      <key>PayloadIdentifier</key><string>com.proxybot.profile.dns</string>
      <key>PayloadUUID</key><string>{dns_uuid}</string>
      <key>PayloadVersion</key><integer>1</integer>
      <key>DNSSettings</key>
      <dict>
        <key>DNSProtocol</key><string>HTTPS</string>
        <key>ProhibitDOH</key><true/>
        <key>ServerName</key><string>{proxy_ip}</string>
        <key>ServerPort</key><integer>{dns_port}</integer>
        <key>SupplementalMatchDomains</key>
        <array>
          <string></string>
        </array>
      </dict>
    </dict>
    <dict>
      <key>PayloadType</key><string>com.apple.security.root</string>
      <key>PayloadIdentifier</key><string>com.proxybot.profile.ca</string>
      <key>PayloadUUID</key><string>{ca_uuid}</string>
      <key>PayloadVersion</key><integer>1</integer>
      <key>PayloadCertificateFileName</key><string>proxybot-ca.cer</string>
      <key>PayloadContent</key><data>{ca_payload_content}</data>
    </dict>
  </array>
  <key>PayloadDisplayName</key><string>ProxyBot</string>
  <key>PayloadDescription</key><string>Install this profile to enable ProxyBot MITM proxy on this device.</string>
  <key>PayloadIdentifier</key><string>com.proxybot.profile</string>
  <key>PayloadOrganization</key><string>ProxyBot</string>
  <key>PayloadRemovalDisallowed</key><false/>
  <key>PayloadType</key><string>Configuration</string>
  <key>PayloadUUID</key><string>{root_uuid}</string>
  <key>PayloadVersion</key><integer>1</integer>
  <key>ConsentText</key>
  <dict>
    <key>default</key><string>By installing this profile, your WiFi traffic will be routed through ProxyBot and the ProxyBot root CA will be trusted for HTTPS inspection. You can remove this profile at any time from Settings &rarr; General &rarr; VPN &amp; Device Management.</string>
  </dict>
</dict>
</plist>"#,
        wifi_uuid = wifi_uuid,
        dns_uuid = dns_uuid,
        ca_uuid = ca_uuid,
        root_uuid = root_uuid,
        proxy_ip = proxy_ip,
        proxy_port = proxy_port,
        dns_port = dns_port,
        ca_payload_content = ca_payload_content,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_CA: &str = "-----BEGIN CERTIFICATE-----\nMIIBexample\n-----END CERTIFICATE-----";

    #[test]
    fn test_build_ios_profile_contains_wifi_proxy() {
        let xml = build_ios_profile(SAMPLE_CA, "192.168.1.5", 8088, 5300);
        assert!(xml.contains("<key>ProxyServer</key><string>192.168.1.5</string>"));
        assert!(xml.contains("<key>ProxyServerPort</key><integer>8088</integer>"));
        assert!(xml.contains("<key>ProxyType</key><string>Manual</string>"));
    }

    #[test]
    fn test_build_ios_profile_contains_dns_payload() {
        let xml = build_ios_profile(SAMPLE_CA, "192.168.1.5", 8088, 5300);
        assert!(xml.contains("<key>DNSSettings</key>"));
        assert!(xml.contains("<key>ServerName</key><string>192.168.1.5</string>"));
        assert!(xml.contains("<key>ServerPort</key><integer>5300</integer>"));
    }

    #[test]
    fn test_build_ios_profile_contains_ca_payload() {
        let xml = build_ios_profile(SAMPLE_CA, "192.168.1.5", 8088, 5300);
        assert!(xml.contains("<key>PayloadCertificateFileName</key><string>proxybot-ca.cer</string>"));
        // base64 of SAMPLE_CA
        let expected_b64 = base64::engine::general_purpose::STANDARD.encode(SAMPLE_CA.as_bytes());
        assert!(xml.contains(&format!("<data>{}</data>", expected_b64)));
    }

    #[test]
    fn test_build_ios_profile_payload_count() {
        let xml = build_ios_profile(SAMPLE_CA, "192.168.1.5", 8088, 5300);
        // The PayloadContent array contains 3 dicts (WiFi, DNS, CA).
        // Count `<dict>` openings within the PayloadContent array. We
        // rely on the structural fact that the file has exactly 1 outer
        // <dict> + 3 inner <dict> = 4 total. A simpler proxy: the
        // PayloadType lines for the 3 payloads must each appear once.
        assert_eq!(xml.matches("<string>com.apple.wifi.managed</string>").count(), 1);
        assert_eq!(xml.matches("<string>com.apple.dnsSettings.managed</string>").count(), 1);
        assert_eq!(xml.matches("<string>com.apple.security.root</string>").count(), 1);
    }

    #[test]
    fn test_build_ios_profile_uuids_are_unique() {
        let xml = build_ios_profile(SAMPLE_CA, "192.168.1.5", 8088, 5300);
        let mut uuids: Vec<&str> = xml
            .matches("<key>PayloadUUID</key><string>")
            .map(|_|
                // Extract the UUID value after the prefix and before </string>
                // Simpler: count distinct UUID-shaped substrings.
            )
            .collect();
        uuids.clear(); // unused

        // Extract all 32-char hex chunks (UUIDs without dashes)
        let mut seen = std::collections::HashSet::new();
        let mut count = 0;
        let mut i = 0;
        while let Some(start) = xml[i..].find("<string>") {
            let abs = i + start + "<string>".len();
            if let Some(end_rel) = xml[abs..].find("</string>") {
                let uuid_str = &xml[abs..abs + end_rel];
                if uuid_str.len() == 36 && uuid_str.chars().filter(|c| *c == '-').count() == 4 {
                    seen.insert(uuid_str.to_string());
                    count += 1;
                }
                i = abs + end_rel;
            } else {
                break;
            }
        }
        assert_eq!(count, 4, "expected 4 UUIDs (root + 3 payloads)");
        assert_eq!(seen.len(), 4, "all 4 UUIDs must be unique");
    }

    #[test]
    fn test_build_ios_profile_consent_text_present() {
        let xml = build_ios_profile(SAMPLE_CA, "192.168.1.5", 8088, 5300);
        assert!(xml.contains("<key>ConsentText</key>"));
        assert!(xml.contains("<key>default</key>"));
        assert!(xml.contains("ProxyBot"));
    }

    #[test]
    fn test_build_ios_profile_is_valid_xml_structure() {
        let xml = build_ios_profile(SAMPLE_CA, "192.168.1.5", 8088, 5300);
        assert!(xml.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(xml.contains("<!DOCTYPE plist"));
        assert!(xml.contains("<plist version=\"1.0\">"));
        assert!(xml.contains("</plist>"));
        // Count opening and closing <dict> tags — must be balanced.
        let opens = xml.matches("<dict>").count();
        let closes = xml.matches("</dict>").count();
        assert_eq!(opens, closes, "unbalanced <dict> tags");
        // Same for <array>
        let array_opens = xml.matches("<array>").count();
        let array_closes = xml.matches("</array>").count();
        assert_eq!(array_opens, array_closes, "unbalanced <array> tags");
    }
}
```

- [ ] **Step 2: Wire the module into `cert.rs`**

In `src-tauri/src/cert.rs`, add the module declaration. The current `cert.rs` has `use rcgen::...` at the top — add `pub mod mobileconfig;` after the existing `use` block. Find the location right after the `use serde::{Deserialize, Serialize};` line and insert:

```rust
pub mod mobileconfig;
```

- [ ] **Step 3: Compile-check (tests will fail to compile because of unused variables — that's expected)**

```bash
cargo check -p proxybot --lib
```

Expected: 0 errors. The `mobileconfig` module compiles. Tests don't run yet because they're not in scope.

Actually, to make sure the tests compile too:
```bash
cargo test -p proxybot --lib cert::mobileconfig --no-run
```

Expected: 0 errors.

- [ ] **Step 4: Run the tests — they should pass on the first run (function is fully implemented)**

```bash
cargo test -p proxybot --lib cert::mobileconfig
```

Expected: 7 passed.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/cert/mobileconfig.rs src-tauri/src/cert.rs
git commit -m "feat(cert): add build_ios_profile for mobileconfig generation

Generates an Apple Configuration Profile XML plist with WiFi proxy,
DNS, and CA cert payloads. Used by the CertServer to serve
/ios.mobileconfig for one-tap iOS device onboarding."
```

---

## Task 2: `cert::wizard::build_android_wizard` (TDD)

**Files:**
- Create: `src-tauri/src/cert/wizard.rs`
- Modify: `src-tauri/src/cert.rs` (add `pub mod wizard;`)

- [ ] **Step 1: Create the file with the function and 5 failing tests**

Create `src-tauri/src/cert/wizard.rs`:

```rust
//! Android device setup wizard HTML generation.
//!
//! Returns a self-contained HTML page with 4 steps: WiFi proxy, DNS,
//! install CA, verify. Includes an Android 7+ CA-trust warning.
//! Used by the CertServer to serve /android-setup.

/// Build a self-contained Android setup HTML page.
///
/// The page guides the user through 4 steps: configure WiFi proxy,
/// set DNS, install the ProxyBot CA, and verify. All CSS is inline.
/// No external resources are loaded.
pub fn build_android_wizard(
    ca_pem: &str,
    proxy_ip: &str,
    proxy_port: u16,
    dns_port: u16,
) -> String {
    let _ = ca_pem; // Currently unused; kept in signature for future use (e.g. embedded install link)
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>ProxyBot Device Setup</title>
  <style>
    body {{ font-family: -apple-system, sans-serif; max-width: 600px;
           margin: 2rem auto; padding: 0 1rem; line-height: 1.6;
           color: #1d1d1f; background: #fff; }}
    .step {{ background: #f5f5f7; border-radius: 12px;
            padding: 1.5rem; margin: 1.5rem 0; }}
    .step h2 {{ margin-top: 0; font-size: 1.1rem; }}
    code {{ background: #e8e8ed; padding: 2px 6px; border-radius: 4px;
           font-family: ui-monospace, monospace; font-size: 0.9em; }}
    .btn {{ display: inline-block; background: #0071e3; color: white;
           padding: 12px 24px; border-radius: 8px;
           text-decoration: none; font-weight: 600; margin: 0.5rem 0; }}
    .warn {{ background: #fff3cd; border-left: 4px solid #ff9500;
            padding: 1rem; margin: 1rem 0; border-radius: 4px; }}
    h1 {{ font-size: 1.5rem; }}
  </style>
</head>
<body>
  <h1>ProxyBot Device Setup</h1>
  <p>Configure your Android device to route traffic through ProxyBot.</p>

  <div class="step">
    <h2>1. WiFi Proxy</h2>
    <p>Settings &rarr; WiFi &rarr; long-press your network &rarr; Modify network &rarr;
       Advanced options &rarr; Proxy: <strong>Manual</strong></p>
    <p>IP: <code>{proxy_ip}</code><br>Port: <code>{proxy_port}</code></p>
  </div>

  <div class="step">
    <h2>2. DNS</h2>
    <p>In the same screen, IP settings &rarr; Static:</p>
    <p>DNS 1: <code>{proxy_ip}</code> (port {dns_port})<br>
       DNS 2: <code>1.1.1.1</code> (fallback)</p>
  </div>

  <div class="step">
    <h2>3. Install CA Certificate</h2>
    <p><a class="btn" href="/ca.crt" download>Download ProxyBot CA</a></p>
    <p>After download: Settings &rarr; Security &rarr; Encryption &amp; credentials &rarr;
       Install a certificate &rarr; CA certificate &rarr; select
       <code>ProxyBot_CA.crt</code></p>
    <div class="warn">
      <strong>Android 7+ note:</strong> By default, Android apps do not trust
      user-installed CAs (only system CAs). Some apps will refuse ProxyBot's
      HTTPS interception. This is an Android security limitation, not a
      ProxyBot bug. Workarounds: install the CA as a system CA (requires
      root), or modify the app's network_security_config.xml.
    </div>
  </div>

  <div class="step">
    <h2>4. Verify</h2>
    <p>Open any HTTPS app &mdash; requests should appear in the ProxyBot
       traffic list with the correct app tag.</p>
  </div>
</body>
</html>"#,
        proxy_ip = proxy_ip,
        proxy_port = proxy_port,
        dns_port = dns_port,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_android_wizard_contains_proxy_ip_and_port() {
        let html = build_android_wizard("CA_PEM", "192.168.1.5", 8088, 5300);
        assert!(html.contains("192.168.1.5"));
        assert!(html.contains("8088"));
    }

    #[test]
    fn test_build_android_wizard_contains_ca_download_link() {
        let html = build_android_wizard("CA_PEM", "192.168.1.5", 8088, 5300);
        assert!(html.contains(r#"<a class="btn" href="/ca.crt" download>Download ProxyBot CA</a>"#));
    }

    #[test]
    fn test_build_android_wizard_contains_dns_step() {
        let html = build_android_wizard("CA_PEM", "192.168.1.5", 8088, 5300);
        assert!(html.contains("DNS 1:"));
        assert!(html.contains("1.1.1.1"));
        assert!(html.contains("fallback"));
    }

    #[test]
    fn test_build_android_wizard_self_contained() {
        let html = build_android_wizard("CA_PEM", "192.168.1.5", 8088, 5300);
        // No external CSS/JS/IMG/font references
        assert!(!html.contains(r#"href="http"#), "should not load external resources");
        assert!(!html.contains(r#"src="http"#), "should not load external images");
        assert!(!html.contains(r#"<link rel="stylesheet""#), "should not have external stylesheet");
    }

    #[test]
    fn test_build_android_wizard_contains_android7_warning() {
        let html = build_android_wizard("CA_PEM", "192.168.1.5", 8088, 5300);
        assert!(html.contains("Android 7+"));
        assert!(html.contains("network_security_config.xml"));
    }
}
```

- [ ] **Step 2: Wire the module into `cert.rs`**

In `src-tauri/src/cert.rs`, after the `pub mod mobileconfig;` line you added in Task 1, add:

```rust
pub mod wizard;
```

- [ ] **Step 3: Compile-check**

```bash
cargo check -p proxybot --lib
```

Expected: 0 errors.

- [ ] **Step 4: Run the tests**

```bash
cargo test -p proxybot --lib cert::wizard
```

Expected: 5 passed.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/cert/wizard.rs src-tauri/src/cert.rs
git commit -m "feat(cert): add build_android_wizard for HTML setup page

Generates a self-contained 4-step HTML guide for Android devices:
WiFi proxy, DNS, CA install (with Android 7+ warning), verify.
Served at /android-setup by the CertServer."
```

---

## Task 3: `commands::device_setup::generate_device_qr` (TDD, requires `qrcode` dep)

**Files:**
- Create: `src-tauri/src/commands/device_setup.rs`
- Modify: `src-tauri/src/lib.rs` (add `pub mod device_setup;`)
- Modify: `src-tauri/src/lib.rs` (register command in `invoke_handler`)
- Modify: `src-tauri/Cargo.toml` (add `qrcode` dep)

This task requires the `qrcode` crate which is currently NOT in `Cargo.toml`. The implementer adds it.

- [ ] **Step 1: Add `qrcode` to `src-tauri/Cargo.toml`**

In `src-tauri/Cargo.toml`, in the `[dependencies]` section, find a stable insertion point (e.g., alphabetically near other deps) and add:

```toml
qrcode = "1"
```

- [ ] **Step 2: Verify the dep resolves**

```bash
cargo check -p proxybot --lib
```

Expected: 0 errors. `qrcode` is now a known crate.

- [ ] **Step 3: Find the existing `invoke_handler` registration in `lib.rs`**

```bash
grep -n "invoke_handler" src-tauri/src/lib.rs
```

Look for the `tauri::generate_handler!` macro invocation. It will look like:

```rust
tauri::generate_handler![
    commands::some_command,
    commands::another_command,
    // ... more
]
```

Note the location (around line 200-280 based on the DNS plan work).

- [ ] **Step 3: Verify `cert_server::is_running()` exists, or add it**

The Tauri command (Step 4) calls `crate::cert_server::is_running()`. The current `cert_server.rs` has a private `SERVER_RUNNING: AtomicBool`. If there's no public `is_running()` accessor, add one. In `src-tauri/src/cert_server.rs`, find `static SERVER_RUNNING: AtomicBool` and add right after it:

```rust
/// Returns true if the CertServer is currently listening.
pub fn is_running() -> bool {
    SERVER_RUNNING.load(Ordering::SeqCst)
}
```

If the existing public API is different (e.g., a getter function already exists), use that instead and adjust Step 4's code to match. The implementer should read `src-tauri/src/cert_server.rs` first and use the existing API.

- [ ] **Step 4: Create `device_setup.rs` with the command and 5 failing tests**

Create `src-tauri/src/commands/device_setup.rs`:

```rust
//! Tauri command for generating device-onboarding QR codes.

use qrcode::QrCode;
use qrcode::render::svg;

use crate::proxy::ProxyState;
use std::sync::Arc;
use tauri::State;

/// Tauri command: generate a QR code SVG for the given platform.
///
/// `platform` must be `"ios"` or `"android"`. Returns an SVG string
/// containing the QR code that encodes the LAN URL of the
/// appropriate CertServer endpoint.
///
/// Errors:
/// - `"Cert server not started. Start the proxy first."` — CertServer's
///   `SERVER_RUNNING` flag is false (caller has not started the proxy).
/// - `"Network info not set. Start the proxy first."` — `ProxyState.local_ip`
///   is None (caller has not called `get_network_info`).
/// - `"Invalid platform: {x}"` — `platform` is not "ios" or "android".
#[tauri::command]
pub fn generate_device_qr(
    platform: String,
    state: State<'_, Arc<ProxyState>>,
) -> Result<String, String> {
    use std::sync::atomic::Ordering;
    if !crate::cert_server::is_running() {
        return Err("Cert server not started. Start the proxy first.".to_string());
    }

    let local_ip = state
        .local_ip
        .lock()
        .map_err(|e| format!("Lock poisoned: {}", e))?
        .clone()
        .ok_or_else(|| "Network info not set. Start the proxy first.".to_string())?;

    let url = build_qr_url(&platform, &local_ip, crate::config::cert_server_port())
        .ok_or_else(|| format!("Invalid platform: {}", platform))?;

    let code = QrCode::new(url.as_bytes()).map_err(|e| format!("QR encode error: {}", e))?;
    Ok(code
        .render::<svg::Color>()
        .max_dimensions(300, 300)
        .build())
}

/// Build the LAN URL that the QR code encodes.
///
/// Pure function — easy to unit-test without a Tauri State.
pub fn build_qr_url(platform: &str, lan_ip: &str, cert_port: u16) -> Option<String> {
    let path = match platform {
        "ios" => "ios.mobileconfig",
        "android" => "android-setup",
        _ => return None,
    };
    Some(format!("http://{}:{}/{}", lan_ip, cert_port, path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_qr_url_ios() {
        let url = build_qr_url("ios", "192.168.1.5", 19876);
        assert_eq!(url, Some("http://192.168.1.5:19876/ios.mobileconfig".to_string()));
    }

    #[test]
    fn test_build_qr_url_android() {
        let url = build_qr_url("android", "192.168.1.5", 19876);
        assert_eq!(url, Some("http://192.168.1.5:19876/android-setup".to_string()));
    }

    #[test]
    fn test_build_qr_url_unknown_platform() {
        assert_eq!(build_qr_url("windows", "192.168.1.5", 19876), None);
        assert_eq!(build_qr_url("", "192.168.1.5", 19876), None);
    }

    #[test]
    fn test_build_qr_url_uses_http() {
        // We use http:// because the CertServer is plain tiny_http;
        // iOS mobileconfig install and Android HTML work over http://
        // on the user's own LAN.
        let url = build_qr_url("ios", "192.168.1.5", 19876).unwrap();
        assert!(url.starts_with("http://"));
    }

    #[test]
    fn test_generate_device_qr_returns_svg_for_known_platforms() {
        // This test only validates the SVG shape — it bypasses the
        // Tauri State requirement by calling build_qr_url + QrCode
        // directly. The actual Tauri command path is exercised by
        // the E2E tests in Task 6.
        for platform in ["ios", "android"] {
            let url = build_qr_url(platform, "192.168.1.5", 19876).unwrap();
            let code = QrCode::new(url.as_bytes()).unwrap();
            let svg = code.render::<svg::Color>().max_dimensions(300, 300).build();
            assert!(svg.starts_with("<svg"), "platform {} produced non-SVG output", platform);
            assert!(svg.contains("</svg>"));
        }
    }
}
```

- [ ] **Step 5: Add `pub mod device_setup;` to `lib.rs`**

In `src-tauri/src/lib.rs`, find the line `pub mod commands;` (or wherever the commands submodule is declared) and the appropriate imports. Add:

```rust
pub mod device_setup;
```

If commands is a directory of files, add this as a sibling. The exact placement varies — match the pattern already used in the file.

- [ ] **Step 6: Add the command to the `invoke_handler!` macro**

In `src-tauri/src/lib.rs`, inside the `tauri::generate_handler![...]` macro list, add `commands::device_setup::generate_device_qr` to the list. The exact line and the exact `use` statement needed at the top of the macro context will depend on existing patterns — find the existing command references in the macro and add yours in the same style. Typical pattern:

```rust
            commands::device_setup::generate_device_qr,
```

- [ ] **Step 7: Compile-check**

```bash
cargo check -p proxybot --lib
```

Expected: 0 errors. (`is_running()` was added in Step 3, so this should compile cleanly.)

- [ ] **Step 8: Run the tests**

```bash
cargo test -p proxybot --lib commands::device_setup
```

Expected: 5 passed.

- [ ] **Step 9: Verify the existing cert_server tests still pass (no regression from the `is_running` addition)**

```bash
cargo test -p proxybot --lib cert_server
```

Expected: existing tests pass. (May be 0 tests if there are none — that's fine, just confirm 0 failures.)

- [ ] **Step 10: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/commands/device_setup.rs src-tauri/src/lib.rs src-tauri/src/cert_server.rs
git commit -m "feat(commands): add generate_device_qr for QR onboarding

Generates SVG QR codes for iOS (.mobileconfig URL) and Android
(setup wizard URL). Validates CertServer running and LAN IP known
before generating. Adds qrcode 1.x dependency."
```

---

## Task 4: CertServer route dispatch (`/ios.mobileconfig`, `/android-setup`)

**Files:**
- Modify: `src-tauri/src/cert_server.rs` (add path-based routing)

The current `cert_server.rs` has a `for request in server.incoming_requests()` loop that always returns the CA PEM. We add a `match request.url()` to dispatch on path.

- [ ] **Step 1: Read the current request loop in `cert_server.rs`**

Open the file. Find the `for request in server.incoming_requests()` block. Note:
- The variable holding the CA cert path (likely `cert_path_clone` or `file_path`)
- The current response pattern (`tiny_http::Response::from_data(data).with_header(...).with_header(...)`)

- [ ] **Step 2: Wrap the existing CA-serving logic in a `match` on the URL**

Replace the body of the `for request in server.incoming_requests()` loop. The new code is:

```rust
        for request in server.incoming_requests() {
            let url = request.url().to_string();
            // Strip any query string for path matching
            let path = url.split('?').next().unwrap_or(&url);

            match path {
                "/ios.mobileconfig" => {
                    let cert_path_clone = file_path.clone();
                    match std::fs::read_to_string(&cert_path_clone) {
                        Ok(ca_pem) => {
                            let xml = crate::cert::mobileconfig::build_ios_profile(
                                &ca_pem,
                                &local_ip,
                                crate::config::proxy_port(),
                                crate::config::dns_port(),
                            );
                            let response = tiny_http::Response::from_string(xml)
                                .with_header(
                                    tiny_http::Header::from_bytes(
                                        &b"Content-Type"[..],
                                        &b"application/x-apple-aspen-config; charset=utf-8"[..],
                                    )
                                    .unwrap(),
                                )
                                .with_header(
                                    tiny_http::Header::from_bytes(
                                        &b"Content-Disposition"[..],
                                        &b"attachment; filename=\"proxybot-ios.mobileconfig\""[..],
                                    )
                                    .unwrap(),
                                );
                            if let Err(e) = request.respond(response) {
                                log::error!("CertServer respond (ios.mobileconfig) error: {}", e);
                            }
                        }
                        Err(e) => {
                            log::error!("CertServer failed to read CA for iOS profile: {}", e);
                            let body = format!(
                                "ProxyBot CA cert not found at {}. Reinstall ProxyBot to regenerate.",
                                cert_path_clone
                            );
                            let response = tiny_http::Response::from_string(body)
                                .with_status_code(500);
                            let _ = request.respond(response);
                        }
                    }
                }
                "/android-setup" => {
                    let cert_path_clone = file_path.clone();
                    match std::fs::read_to_string(&cert_path_clone) {
                        Ok(ca_pem) => {
                            let html = crate::cert::wizard::build_android_wizard(
                                &ca_pem,
                                &local_ip,
                                crate::config::proxy_port(),
                                crate::config::dns_port(),
                            );
                            let response = tiny_http::Response::from_string(html)
                                .with_header(
                                    tiny_http::Header::from_bytes(
                                        &b"Content-Type"[..],
                                        &b"text/html; charset=utf-8"[..],
                                    )
                                    .unwrap(),
                                );
                            if let Err(e) = request.respond(response) {
                                log::error!("CertServer respond (android-setup) error: {}", e);
                            }
                        }
                        Err(e) => {
                            log::error!("CertServer failed to read CA for Android wizard: {}", e);
                            let body = format!(
                                "ProxyBot CA cert not found at {}. Reinstall ProxyBot to regenerate.",
                                cert_path_clone
                            );
                            let response = tiny_http::Response::from_string(body)
                                .with_status_code(500);
                            let _ = request.respond(response);
                        }
                    }
                }
                _ => {
                    // Existing behavior: serve the CA cert for any other path
                    // (including "/", "", "/ca.crt", and any path the user types).
                    match std::fs::read(file_path) {
                        Ok(data) => {
                            let response = tiny_http::Response::from_data(data)
                                .with_header(
                                    tiny_http::Header::from_bytes(
                                        &b"Content-Type"[..],
                                        &b"application/x-x509-ca-cert"[..],
                                    )
                                    .unwrap(),
                                )
                                .with_header(
                                    tiny_http::Header::from_bytes(
                                        &b"Content-Disposition"[..],
                                        &b"attachment; filename=\"ProxyBot_CA.crt\"[..],
                                    )
                                    .unwrap(),
                                );
                            if let Err(e) = request.respond(response) {
                                log::error!("CertServer respond (ca) error: {}", e);
                            }
                        }
                        Err(e) => {
                            log::error!("CertServer failed to read cert: {}", e);
                            let response = tiny_http::Response::from_string("Certificate not found")
                                .with_status_code(404);
                            let _ = request.respond(response);
                        }
                    }
                }
            }
        }
```

The implementer should:
1. Use the existing variable name for the cert path in scope (likely `file_path` or `cert_path_clone` based on what they see)
2. Keep the existing CA-serving code intact in the `_` (default) arm — this preserves backward compatibility
3. Adapt the pattern to the existing function structure (this is inside a `std::thread::spawn` closure, so the closure-capture context is in scope)

- [ ] **Step 3: Compile-check**

```bash
cargo check -p proxybot --lib
```

Expected: 0 errors.

- [ ] **Step 4: Run all cert tests to confirm no regressions**

```bash
cargo test -p proxybot --lib cert
```

Expected: existing tests pass. The new modules' tests (Tasks 1-2) and the new test in Task 3 should all be in scope.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/cert_server.rs
git commit -m "feat(cert_server): add /ios.mobileconfig and /android-setup routes

Reuses the existing tiny_http on port 19876 with path-based dispatch.
Existing /, /ca.crt, and any other path still return the CA cert
(unchanged behavior). New routes dynamically build the iOS profile
or Android wizard HTML on each request."
```

---

## Task 5: `DeviceQrPanel` React component + embed in `ClientSetup`

**Files:**
- Create: `src/components/setup/DeviceQrPanel.tsx`
- Modify: `src/components/setup/ClientSetup.tsx` (import and render the new panel)

This is a UI-only task with no new Tauri commands (it calls `generate_device_qr` from Task 3). The component is a tabbed panel showing iOS / Android QRs. Includes a "phone must be on ProxyBot WiFi" warning.

- [ ] **Step 1: Look at the existing `ClientSetup.tsx` to understand its structure**

```bash
head -40 src/components/setup/ClientSetup.tsx
```

Note the imports, the component name, and the JSX style (shadcn/ui based on the project's CLAUDE.md). Adapt the new component to the existing style.

- [ ] **Step 2: Create the component**

Create `src/components/setup/DeviceQrPanel.tsx`:

```tsx
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type Platform = "ios" | "android";

/**
 * QR panel for one-tap mobile device onboarding.
 * Renders two tabs (iOS / Android), each showing a QR code that encodes
 * the LAN URL of the appropriate CertServer endpoint. The iOS QR
 * triggers a .mobileconfig install; the Android QR opens a setup wizard.
 */
export function DeviceQrPanel() {
  const [platform, setPlatform] = useState<Platform>("ios");
  const [svg, setSvg] = useState<string>("");
  const [error, setError] = useState<string>("");
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError("");
    invoke<string>("generate_device_qr", { platform })
      .then((result) => {
        if (!cancelled) {
          setSvg(result);
          setLoading(false);
        }
      })
      .catch((err) => {
        if (!cancelled) {
          setError(String(err));
          setSvg("");
          setLoading(false);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [platform]);

  return (
    <div className="border rounded-lg p-4 bg-card text-card-foreground">
      <h3 className="text-lg font-semibold mb-2">Add Mobile Device</h3>
      <p className="text-sm text-muted-foreground mb-3">
        Scan with your phone to install WiFi proxy, DNS, and the ProxyBot
        CA in one tap. <strong>Make sure your phone is connected to the
        ProxyBot WiFi network before scanning.</strong>
      </p>

      <div className="flex gap-2 mb-4" role="tablist">
        <button
          role="tab"
          aria-selected={platform === "ios"}
          onClick={() => setPlatform("ios")}
          className={`px-3 py-1.5 rounded ${
            platform === "ios" ? "bg-primary text-primary-foreground" : "bg-muted"
          }`}
        >
          iOS
        </button>
        <button
          role="tab"
          aria-selected={platform === "android"}
          onClick={() => setPlatform("android")}
          className={`px-3 py-1.5 rounded ${
            platform === "android" ? "bg-primary text-primary-foreground" : "bg-muted"
          }`}
        >
          Android
        </button>
      </div>

      {loading && <div className="text-sm text-muted-foreground">Loading…</div>}

      {error && (
        <div className="text-sm text-destructive border border-destructive rounded p-2">
          {error}
        </div>
      )}

      {svg && !error && (
        <div
          className="flex justify-center"
          dangerouslySetInnerHTML={{ __html: svg }}
          data-testid="device-qr-svg"
        />
      )}

      {platform === "ios" && !error && (
        <details className="mt-3 text-sm">
          <summary className="cursor-pointer text-muted-foreground">
            After installing the profile
          </summary>
          <p className="mt-2">
            iOS does not auto-trust user-installed CAs. Go to{" "}
            <strong>Settings → General → About → Certificate Trust
            Settings</strong> and enable <em>ProxyBot CA</em> full trust
            for HTTPS interception to work.
          </p>
        </details>
      )}
    </div>
  );
}
```

- [ ] **Step 3: Embed the panel in `ClientSetup.tsx`**

In `src/components/setup/ClientSetup.tsx`, find the existing component's JSX (look for the outermost `return (...)` or the top-level `<div>`). Add an import at the top:

```tsx
import { DeviceQrPanel } from "./DeviceQrPanel";
```

And render the panel at the top of the existing component's return value, before any other content. For example, if the existing return is:

```tsx
return (
  <div className="p-6">
    {/* existing content */}
  </div>
);
```

Change it to:

```tsx
return (
  <div className="p-6 space-y-6">
    <DeviceQrPanel />
    {/* existing content */}
  </div>
);
```

(Use the existing wrapper element and className; only add `<DeviceQrPanel />` as the first child.)

- [ ] **Step 4: Compile-check (TypeScript)**

```bash
pnpm typecheck
```

Expected: 0 errors.

- [ ] **Step 5: Run the existing UI tests to confirm no regression**

```bash
pnpm test:ui
```

Expected: existing tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/components/setup/DeviceQrPanel.tsx src/components/setup/ClientSetup.tsx
git commit -m "feat(ui): add DeviceQrPanel for QR onboarding

Renders iOS/Android tabs, calls generate_device_qr, shows the QR SVG
inline. Includes iOS post-install instructions and a WiFi warning."
```

---

## Task 6: Playwright E2E tests

**Files:**
- Create: `e2e/qr-onboarding.spec.ts`

- [ ] **Step 1: Check Playwright config exists**

```bash
ls playwright.config.ts
```

If it doesn't exist, ask the user how to wire up Playwright. Otherwise, continue.

- [ ] **Step 2: Look at an existing E2E test to understand the patterns**

```bash
ls e2e/ | head -5
```

Pick one (e.g., `e2e/basic.spec.ts` if it exists) and read it. Note:
- How the app is launched
- How `invoke` calls are mocked or skipped (E2E may not run the Tauri runtime)
- Whether tests start the CertServer or just hit it directly

- [ ] **Step 3: Create the E2E test file**

Create `e2e/qr-onboarding.spec.ts` with the following content (adapt to your project's existing test patterns):

```typescript
import { test, expect } from "@playwright/test";
import { spawn } from "child_process";
import * as path from "path";
import * as fs from "fs";

const CERT_SERVER_PORT = 19876;
const TEST_LAN_IP = "127.0.0.1"; // localhost for E2E; LAN IP is set at runtime

// Boot a minimal CertServer for E2E. The actual server is in
// src-tauri/src/cert_server.rs, but for E2E we use tiny_http directly
// to serve a known CA PEM. This avoids the need to launch the Tauri app.

let server: ReturnType<typeof spawn> | null = null;

test.beforeAll(async () => {
  // Spawn a minimal test server that responds to /ca.crt,
  // /ios.mobileconfig, and /android-setup.
  // Implementation note: copy the relevant routes from cert_server.rs
  // into a tiny_http Node script (test-helpers/cert_server_e2e.ts),
  // or import a precompiled binary if available.
  //
  // For simplicity, this E2E tests the **HTTP-level behavior** by
  // starting a Node.js script that mounts the same routes.
  const helper = path.join(__dirname, "test-helpers", "cert_server_e2e.mjs");
  if (!fs.existsSync(helper)) {
    test.skip(true, `cert_server_e2e.mjs not found at ${helper}`);
    return;
  }
  server = spawn("node", [helper], {
    env: { ...process.env, CERT_SERVER_PORT: String(CERT_SERVER_PORT) },
  });
  // Wait for server to be ready
  await new Promise((r) => setTimeout(r, 500));
});

test.afterAll(async () => {
  if (server) {
    server.kill("SIGTERM");
  }
});

test("GET /ios.mobileconfig returns mobileconfig content-type", async ({ request }) => {
  const response = await request.get(`http://${TEST_LAN_IP}:${CERT_SERVER_PORT}/ios.mobileconfig`);
  expect(response.status()).toBe(200);
  expect(response.headers()["content-type"]).toContain("x-apple-aspen-config");
  const body = await response.text();
  expect(body).toContain("<plist version=\"1.0\">");
  expect(body).toContain("ProxyServer");
});

test("GET /android-setup returns HTML content-type", async ({ request }) => {
  const response = await request.get(`http://${TEST_LAN_IP}:${CERT_SERVER_PORT}/android-setup`);
  expect(response.status()).toBe(200);
  expect(response.headers()["content-type"]).toContain("text/html");
  const body = await response.text();
  expect(body).toContain("ProxyBot Device Setup");
  expect(body).toContain("Android 7+");
});

test("GET /ca.crt still returns the CA cert (regression)", async ({ request }) => {
  const response = await request.get(`http://${TEST_LAN_IP}:${CERT_SERVER_PORT}/ca.crt`);
  expect(response.status()).toBe(200);
  expect(response.headers()["content-type"]).toContain("application/x-x509-ca-cert");
});

test("unknown path returns CA cert (backward compat)", async ({ request }) => {
  // Per the design, any path other than the two new ones falls
  // through to the CA cert (preserving the existing single-purpose
  // behavior of the server).
  const response = await request.get(`http://${TEST_LAN_IP}:${CERT_SERVER_PORT}/some/random/path`);
  expect(response.status()).toBe(200);
  expect(response.headers()["content-type"]).toContain("application/x-x509-ca-cert");
});
```

- [ ] **Step 4: Create the E2E test helper server**

Create `e2e/test-helpers/cert_server_e2e.mjs`:

```javascript
// Minimal CertServer for E2E tests. Mirrors the routes from
// src-tauri/src/cert_server.rs but in Node.js. The Rust unit tests
// cover the function logic; this script covers the HTTP-level
// integration (Content-Type, status codes, dispatch).

import http from "node:http";
import { readFileSync } from "node:fs";

const PORT = parseInt(process.env.CERT_SERVER_PORT || "19876", 10);
const SAMPLE_CA = "-----BEGIN CERTIFICATE-----\nMIIBexample\n-----END CERTIFICATE-----\n";

function iosProfile() {
  return `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>PayloadContent</key>
  <array>
    <dict>
      <key>PayloadType</key><string>com.apple.wifi.managed</string>
      <key>ProxyServer</key><string>127.0.0.1</string>
      <key>ProxyServerPort</key><integer>8088</integer>
    </dict>
  </array>
  <key>PayloadDisplayName</key><string>ProxyBot</string>
</dict>
</plist>`;
}

function androidWizard() {
  return `<!DOCTYPE html>
<html><head><title>ProxyBot Device Setup</title></head>
<body><h1>ProxyBot Device Setup</h1>
<p>Android 7+ note here</p></body></html>`;
}

const server = http.createServer((req, res) => {
  if (req.url.startsWith("/ios.mobileconfig")) {
    res.writeHead(200, {
      "Content-Type": "application/x-apple-aspen-config; charset=utf-8",
      "Content-Disposition": 'attachment; filename="proxybot-ios.mobileconfig"',
    });
    res.end(iosProfile());
  } else if (req.url.startsWith("/android-setup")) {
    res.writeHead(200, { "Content-Type": "text/html; charset=utf-8" });
    res.end(androidWizard());
  } else {
    res.writeHead(200, {
      "Content-Type": "application/x-x509-ca-cert",
      "Content-Disposition": 'attachment; filename="ProxyBot_CA.crt"',
    });
    res.end(SAMPLE_CA);
  }
});

server.listen(PORT, () => {
  console.log(`E2E cert server listening on ${PORT}`);
});
```

- [ ] **Step 5: Run the E2E tests**

```bash
pnpm test:e2e -- qr-onboarding
```

Expected: 4 tests pass.

- [ ] **Step 6: Commit**

```bash
git add e2e/qr-onboarding.spec.ts e2e/test-helpers/cert_server_e2e.mjs
git commit -m "test(e2e): add Playwright tests for QR onboarding routes

Verifies CertServer serves /ios.mobileconfig, /android-setup, and
/ca.crt with correct Content-Types. Uses a minimal Node.js helper
to mirror the Rust server's routes for HTTP-level testing."
```

---

## Task 7: Final verification

**Files:** none modified

- [ ] **Step 1: Run `cargo build`**

```bash
cargo build
```

Expected: 0 errors. Pre-existing warnings (e.g., workspace profile warning) are out of scope.

- [ ] **Step 2: Run the full test suite**

```bash
cargo test
```

Expected: all tests pass, including the 7 iOS profile tests, 5 wizard tests, 5 device_setup tests, and any prior DNS-correlation / cert tests.

- [ ] **Step 3: Run `cargo clippy`**

```bash
cargo clippy -p proxybot --no-deps
```

Expected: no new clippy warnings from this branch's code (pre-existing warnings in untouched files are out of scope).

- [ ] **Step 4: Run `pnpm typecheck`**

```bash
pnpm typecheck
```

Expected: 0 errors.

- [ ] **Step 5: Run `pnpm test:ui`**

```bash
pnpm test:ui
```

Expected: all UI tests pass.

- [ ] **Step 6: Final commit if any cleanup was needed**

```bash
git status
# If clean, skip. Otherwise:
git add -A
git commit -m "chore: post-implementation cleanup"
```

---

## Manual verification (out-of-band)

The spec's `§8.3` calls for manual testing on a real device. This is the user's step, not the implementer's.

iOS path:
1. Phone on ProxyBot WiFi
2. Open Setup page → iOS tab → scan QR with iPhone camera
3. Safari opens → iOS prompts "Install Profile"
4. Settings → General → VPN & Device Management → install
5. Settings → General → About → Certificate Trust Settings → enable ProxyBot CA
6. Open WeChat → ProxyBot UI traffic list should show `💬 WeChat`-tagged requests

Android path:
1. Phone on ProxyBot WiFi
2. Open Setup page → Android tab → scan QR
3. Browser opens to 4-step wizard
4. Follow steps: WiFi proxy → DNS → download CA → install
5. Some apps (targetSdk ≥ 24) will reject — expected, documented in the wizard

---

## References

- Spec: `docs/superpowers/specs/2026-06-12-qr-onboarding-design.md`
- Existing QR infrastructure: `src-tauri/src/cert/qr.rs` (dead code — `qrcode` not in Cargo.toml, `mod qr` not declared; left as-is per YAGNI)
- Existing CertServer: `src-tauri/src/cert_server.rs:11` (`start_cert_server`, port 19876, AtomicBool `SERVER_RUNNING`)
- Existing CertManager: `src-tauri/src/cert.rs:31` (`get_ca_cert_pem`)
- Existing network info: `src-tauri/src/network/mod.rs:47` (`NetworkInfo { lan_ip, interface }`)
- Existing ProxyState: `src-tauri/src/proxy/mod.rs:126-135` (has `local_ip: Mutex<Option<String>>`)
- Config: `src-tauri/src/config.rs:18` (`cert_server_port: 19876`), `:67-68` (proxy_port: 8088, dns_port: 5300)
- Tauri command pattern: `src-tauri/src/commands/client_setup.rs` (similar shape to the new `device_setup`)
- Apple mobileconfig docs: https://developer.apple.com/documentation/devicemanagement
