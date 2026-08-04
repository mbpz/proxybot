//! Device Onboarding Module for explicit-proxy mobile setup.

use proxybot_core::desktop_contract::{DesktopContractType, WireType};
use qrcode::render::svg;
use qrcode::QrCode;
use std::sync::Arc;
use tauri::State;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DevicePlatform {
    Ios,
    Android,
}

impl WireType for DevicePlatform {
    fn type_script_type() -> String {
        "DevicePlatform".to_owned()
    }
}

impl DesktopContractType for DevicePlatform {
    const NAME: &'static str = "DevicePlatform";

    fn type_script_declaration() -> String {
        "export type DevicePlatform = \"ios\" | \"android\";\n".to_owned()
    }
}

use crate::cert_server::CertServerState;
use crate::proxy::ProxyState;

proxybot_core::desktop_contract_type! {
    /// Prepared, self-contained inputs for one mobile Device Onboarding flow.
    #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    pub struct DeviceOnboarding {
        pub platform: DevicePlatform,
        pub interface: String,
        pub lan_ip: String,
        pub proxy_port: u16,
        pub server_url: String,
        pub setup_url: String,
        pub ca_url: String,
        pub qr_svg: String,
    }
}

/// Discover the active LAN Interface, publish it for other desktop Adapters,
/// start the certificate distribution server if needed, and return all inputs
/// required by the explicit-proxy setup UI.
#[tauri::command]
pub fn prepare_device_onboarding(
    platform: DevicePlatform,
    proxy_state: State<'_, Arc<ProxyState>>,
    cert_server: State<'_, Arc<CertServerState>>,
    config: State<'_, Arc<proxybot_core::AppConfig>>,
) -> Result<DeviceOnboarding, String> {
    let network = crate::network::get_network_info()?;
    *proxy_state
        .interface
        .lock()
        .map_err(|error| format!("Network state unavailable: {error}"))? =
        Some(network.interface.clone());
    *proxy_state
        .local_ip
        .lock()
        .map_err(|error| format!("Network state unavailable: {error}"))? =
        Some(network.lan_ip.clone());

    let server_url = format!("http://{}:{}", network.lan_ip, config.cert_server_port);
    cert_server.ensure_started(
        &format!("{}:{}", network.lan_ip, config.cert_server_port),
        server_url.clone(),
        &config.ca_cert_path,
        network.lan_ip.clone(),
        config.proxy_port,
        config.dns_port,
    )?;

    build_device_onboarding(
        platform,
        &network.interface,
        &network.lan_ip,
        config.proxy_port,
        &server_url,
    )
}

#[tauri::command]
pub fn stop_device_onboarding(cert_server: State<'_, Arc<CertServerState>>) -> Result<(), String> {
    cert_server.stop()
}

fn build_device_onboarding(
    platform: DevicePlatform,
    interface: &str,
    lan_ip: &str,
    proxy_port: u16,
    server_url: &str,
) -> Result<DeviceOnboarding, String> {
    let path = setup_path(platform);
    let setup_url = format!("{server_url}/{path}");
    let ca_url = format!("{server_url}/ca.crt");
    let code = QrCode::new(setup_url.as_bytes())
        .map_err(|error| format!("Could not create setup QR code: {error}"))?;
    let qr_svg = code.render::<svg::Color>().max_dimensions(300, 300).build();

    Ok(DeviceOnboarding {
        platform,
        interface: interface.to_owned(),
        lan_ip: lan_ip.to_owned(),
        proxy_port,
        server_url: server_url.to_owned(),
        setup_url,
        ca_url,
        qr_svg,
    })
}

fn setup_path(platform: DevicePlatform) -> &'static str {
    match platform {
        // Core onboarding uses a manual explicit proxy. The iOS QR downloads
        // only the CA; the legacy managed Wi-Fi/DNS profile remains a Lab.
        DevicePlatform::Ios => "ca.crt",
        DevicePlatform::Android => "android-setup",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ios_onboarding_uses_explicit_proxy_and_ca_download() {
        let setup = build_device_onboarding(
            DevicePlatform::Ios,
            "en0",
            "192.168.1.5",
            8088,
            "http://192.168.1.5:19876",
        )
        .unwrap();

        assert_eq!(setup.lan_ip, "192.168.1.5");
        assert_eq!(setup.proxy_port, 8088);
        assert_eq!(setup.setup_url, "http://192.168.1.5:19876/ca.crt");
        assert!(!setup.setup_url.contains("mobileconfig"));
        assert!(setup.qr_svg.contains("<svg"));
    }

    #[test]
    fn android_onboarding_uses_the_guided_setup_page() {
        let setup = build_device_onboarding(
            DevicePlatform::Android,
            "en0",
            "192.168.1.5",
            8088,
            "http://192.168.1.5:19876",
        )
        .unwrap();

        assert_eq!(setup.setup_url, "http://192.168.1.5:19876/android-setup");
        assert_eq!(setup.ca_url, "http://192.168.1.5:19876/ca.crt");
    }

    #[test]
    fn unsupported_platform_is_rejected_before_a_url_is_created() {
        assert!(serde_json::from_str::<DevicePlatform>("\"windows\"").is_err());
    }
}
