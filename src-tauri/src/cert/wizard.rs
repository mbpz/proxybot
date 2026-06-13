//! Android device setup wizard HTML generation.
//!
//! Returns a self-contained HTML page with 4 steps: WiFi proxy, DNS,
//! install CA, verify. Includes an Android 7+ CA-trust warning.
//! Used by the CertServer to serve /android-setup.

/// Build a self-contained Android setup HTML page.
///
/// The page guides the user through 4 steps: configure WiFi proxy,
/// set DNS, install the ProxyBot CA, and verify. All CSS is inline.
/// No external resources are loaded. The CA is downloaded as a separate
/// `/ca.crt` resource rather than embedded in the page.
pub fn build_android_wizard(
    proxy_ip: &str,
    proxy_port: u16,
    dns_port: u16,
) -> String {
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
        let html = build_android_wizard("192.168.1.5", 8088, 5300);
        assert!(html.contains("192.168.1.5"));
        assert!(html.contains("8088"));
    }

    #[test]
    fn test_build_android_wizard_contains_ca_download_link() {
        let html = build_android_wizard("192.168.1.5", 8088, 5300);
        assert!(html.contains(r#"<a class="btn" href="/ca.crt" download>Download ProxyBot CA</a>"#));
    }

    #[test]
    fn test_build_android_wizard_contains_dns_step() {
        let html = build_android_wizard("192.168.1.5", 8088, 5300);
        assert!(html.contains("DNS 1:"));
        assert!(html.contains("1.1.1.1"));
        assert!(html.contains("fallback"));
    }

    #[test]
    fn test_build_android_wizard_self_contained() {
        let html = build_android_wizard("192.168.1.5", 8088, 5300);
        assert!(!html.contains(r#"href="http"#), "should not load external resources");
        assert!(!html.contains(r#"src="http"#), "should not load external images");
        assert!(!html.contains(r#"<link rel="stylesheet""#), "should not have external stylesheet");
    }

    #[test]
    fn test_build_android_wizard_contains_android7_warning() {
        let html = build_android_wizard("192.168.1.5", 8088, 5300);
        assert!(html.contains("Android 7+"));
        assert!(html.contains("network_security_config.xml"));
    }
}
