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
pub fn build_ios_profile(ca_pem: &str, proxy_ip: &str, proxy_port: u16, dns_port: u16) -> String {
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
        <key>DNSProtocol</key><string>UDP</string>
        <key>ProhibitDOH</key><true/>
        <key>ServerName</key><string>{proxy_ip}</string>
        <key>ServerPort</key><integer>{dns_port}</integer>
        <key>SupplementalMatchDomains</key>
        <array/>
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
        assert!(xml.contains("<key>DNSProtocol</key><string>UDP</string>"));
    }

    #[test]
    fn test_build_ios_profile_contains_ca_payload() {
        let xml = build_ios_profile(SAMPLE_CA, "192.168.1.5", 8088, 5300);
        assert!(
            xml.contains("<key>PayloadCertificateFileName</key><string>proxybot-ca.cer</string>")
        );
        // base64 of SAMPLE_CA
        let expected_b64 = base64::engine::general_purpose::STANDARD.encode(SAMPLE_CA.as_bytes());
        assert!(xml.contains(&format!("<data>{}</data>", expected_b64)));
    }

    #[test]
    fn test_build_ios_profile_payload_count() {
        let xml = build_ios_profile(SAMPLE_CA, "192.168.1.5", 8088, 5300);
        assert_eq!(
            xml.matches("<string>com.apple.wifi.managed</string>")
                .count(),
            1
        );
        assert_eq!(
            xml.matches("<string>com.apple.dnsSettings.managed</string>")
                .count(),
            1
        );
        assert_eq!(
            xml.matches("<string>com.apple.security.root</string>")
                .count(),
            1
        );
    }

    #[test]
    fn test_build_ios_profile_uuids_are_unique() {
        let xml = build_ios_profile(SAMPLE_CA, "192.168.1.5", 8088, 5300);

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
        let opens = xml.matches("<dict>").count();
        let closes = xml.matches("</dict>").count();
        assert_eq!(opens, closes, "unbalanced <dict> tags");
        let array_opens = xml.matches("<array>").count();
        let array_closes = xml.matches("</array>").count();
        assert_eq!(array_opens, array_closes, "unbalanced <array> tags");
    }
}
