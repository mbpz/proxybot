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
#[tauri::command]
pub fn generate_device_qr(
    platform: String,
    state: State<'_, Arc<ProxyState>>,
) -> Result<String, String> {
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
        let url = build_qr_url("ios", "192.168.1.5", 19876).unwrap();
        assert!(url.starts_with("http://"));
    }

    #[test]
    fn test_generate_device_qr_returns_svg_for_known_platforms() {
        for platform in ["ios", "android"] {
            let url = build_qr_url(platform, "192.168.1.5", 19876).unwrap();
            let code = QrCode::new(url.as_bytes()).unwrap();
            let svg = code.render::<svg::Color>().max_dimensions(300, 300).build();
            // qrcode 0.14 SVG may start with <?xml or <svg
            assert!(svg.contains("<svg"), "platform {} produced non-SVG output: {}", platform, &svg[..svg.len().min(100)]);
            assert!(svg.contains("</svg>"));
        }
    }
}
