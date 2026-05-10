use qrcode::QrCode;
use qrcode::render::svg;

pub struct QrGenerator;

impl QrGenerator {
    /// Generate QR code SVG for CA certificate download
    pub fn generate_ca_qr(ca_pem: &str, download_url: &str) -> Result<String, String> {
        // Combine CA PEM data URL + download URL
        let data = format!("{}|{}", ca_pem, download_url);
        let code = QrCode::new(data.as_bytes()).map_err(|e| e.to_string())?;
        Ok(code.render::<svg::Color>()
            .max_dimensions(300, 300)
            .build())
    }

    /// Generate download page HTML with embedded QR code
    pub fn generate_download_page(ca_pem: &str, download_url: &str) -> String {
        let qr_svg = Self::generate_ca_qr(ca_pem, download_url).unwrap_or_default();
        format!(r#"<!DOCTYPE html>
<html><body>
<h1>ProxyBot CA Certificate</h1>
<p>Scan to install CA on your mobile device:</p>
{}
<p>Or <a href="{}">click here to download</a></p>
</body></html>"#, qr_svg, download_url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_ca_qr() {
        let ca_pem = "-----BEGIN CERTIFICATE-----\ntest\n-----END CERTIFICATE-----";
        let download_url = "http://localhost:8089/ca/download";
        let result = QrGenerator::generate_ca_qr(ca_pem, download_url);
        assert!(result.is_ok());
        let svg = result.unwrap();
        assert!(svg.contains("<svg"));
    }

    #[test]
    fn test_generate_download_page() {
        let ca_pem = "-----BEGIN CERTIFICATE-----\ntest\n-----END CERTIFICATE-----";
        let download_url = "http://localhost:8089/ca/download";
        let page = QrGenerator::generate_download_page(ca_pem, download_url);
        assert!(page.contains("<!DOCTYPE html>"));
        assert!(page.contains("ProxyBot CA Certificate"));
        assert!(page.contains("<svg"));
        assert!(page.contains(download_url));
    }
}