# QR Code CA Distribution Design Specification

**Date:** 2026-05-10
**Feature:** QR Code CA Distribution
**Status:** Design

---

## 1. Overview

This document specifies the design for distributing the ProxyBot Root CA certificate to mobile devices via QR code. Users can scan the QR code with their phone camera to download and install the CA certificate, enabling MITM inspection of HTTPS traffic on mobile devices.

## 2. Problem Statement

Currently, CA distribution requires:
1. Finding the CA file on the filesystem
2. Transferring it to the mobile device (email, AirDrop, etc.)
3. Manual import in iOS/Android settings

This is friction-heavy and discourages adoption. A QR code approach allows:
1. Display QR code in ProxyBot UI
2. User scans with phone camera
3. CA profile downloads automatically
4. One-tap install

## 3. Architecture

### 3.1 High-Level Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                         ProxyBot                                │
│  ┌───────────────┐    ┌───────────────┐    ┌───────────────┐   │
│  │  CA Generator │───▶│  QR Encoder   │───▶│   UI/TUI      │   │
│  │  (cert.rs)    │    │  (qrcode)     │    │   Display     │   │
│  └───────────────┘    └───────────────┘    └───────────────┘   │
│         │                   │                     │             │
│         │                   │                     │             │
│         ▼                   ▼                     ▼             │
│  ┌───────────────┐    ┌───────────────┐    ┌───────────────┐   │
│  │  cert.pem     │    │  QR Image     │    │   Mobile      │   │
│  │  (PEM format) │    │  (PNG/HTML)   │    │   Camera      │   │
│  └───────────────┘    └───────────────┘    └───────────────┘   │
│                                                    │            │
│                                                    ▼            │
│                                           ┌───────────────┐    │
│                                           │  CA Profile   │    │
│                                           │  Downloaded   │    │
│                                           └───────────────┘    │
└─────────────────────────────────────────────────────────────────┘
```

### 3.2 Components

| Component | Responsibility |
|-----------|----------------|
| `cert.rs` | Existing CA generation and storage |
| `qr_gen.rs` | NEW: QR code image generation from PEM |
| `cert_server.rs` | Existing HTTP server for CA download |
| `qr_display.rs` | NEW: TUI/UI components for QR display |
| `mobile_profile.rs` | NEW: Generate mobile-installable CA profile |

## 4. QR Code Generation

### 4.1 QR Code Format

The QR code will encode a URL that the mobile device's camera can open:
- Format: `https://{local_ip}:19876/ca` (or custom port)
- The URL downloads the CA certificate directly
- Camera prompts "Open in Settings?" to install profile

### 4.2 Data Size Considerations

A QR code has limited data capacity:
- Version 40 QR code: ~2,953 bytes
- A typical CA PEM file: ~1,500-2,000 bytes (base64 encoded)

**Solution:** Use URL redirection, not direct PEM encoding.
- QR encodes a short URL
- URL redirects to cert download endpoint
- Mobile browser downloads the .crt file

### 4.3 Implementation

**File:** `src-tauri/src/qr_gen.rs`

```rust
use qrcode::QrCode;
use qrcode::render::svg;
use base64::{Engine as _, engine::general_purpose};

pub struct QrGenerator {
    cert_pem: String,
    local_ip: String,
    port: u16,
}

impl QrGenerator {
    pub fn new(cert_pem: String, local_ip: String, port: u16) -> Self {
        Self {
            cert_pem,
            local_ip,
            port,
        }
    }

    /// Generate a QR code as SVG string for display in TUI/UI
    pub fn generate_svg(&self) -> Result<String, QrError> {
        let download_url = self.download_url();

        let code = QrCode::new(download_url.as_bytes())
            .map_err(|e| QrError::GenerationFailed(e.to_string()))?;

        let svg_image = code.render()
            .min_dimensions(200, 200)
            .max_dimensions(400, 400)
            .dark_color(svg::Color("#000000"))
            .light_color(svg::Color("#FFFFFF"))
            .build();

        Ok(svg_image)
    }

    /// Generate a QR code as PNG bytes
    pub fn generate_png(&self, size: u32) -> Result<Vec<u8>, QrError> {
        use qrcode::render::png;

        let download_url = self.download_url();

        let code = QrCode::new(download_url.as_bytes())
            .map_err(|e| QrError::GenerationFailed(e.to_string()))?;

        let png_image = code.render()
            .min_dimensions(size, size)
            .max_dimensions(size, size)
            .build();

        Ok(png_image)
    }

    /// Get the download URL encoded in the QR code
    pub fn download_url(&self) -> String {
        format!("http://{}:{}/ca", self.local_ip, self.port)
    }

    /// Get the download URL for manual access (no QR)
    pub fn download_url_with_fallback(&self) -> String {
        format!("http://{}:{}/ca", self.local_ip, self.port)
    }
}

#[derive(Debug)]
pub enum QrError {
    GenerationFailed(String),
    RenderFailed(String),
}
```

### 4.4 QR Code Display

**File:** `src-tauri/src/tui/qr_panel.rs`

Display QR code in TUI:

```rust
use ratatui::widgets::Widget;
use ratatui::prelude::*;

pub struct QrCodeWidget {
    svg_data: String,
}

impl QrCodeWidget {
    pub fn new(svg_data: String) -> Self {
        Self { svg_data }
    }
}

impl Widget for QrCodeWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Render QR code using ASCII art fallback for terminals
        // Or embed SVG/PNG if terminal supports images
        let style = Style::default().fg(Color::White);
        let text = Text::raw(self.ascii_art());
        text.render(area, buf);
    }
}

impl QrCodeWidget {
    fn ascii_art(&self) -> String {
        // Simple ASCII representation for terminal display
        // For actual display, prefer embedded image support
        let code = QrCode::new(self.download_url().as_bytes()).unwrap();

        // Generate a block-character QR code
        let modules = code.to_light_modules();
        let mut art = String::new();

        for row in modules {
            for dark in row {
                art.push(if dark { "██" } else { "  " });
            }
            art.push('\n');
        }

        art
    }
}
```

### 4.5 Web UI Integration

**File:** `frontend/components/QrCodeDisplay.tsx`

```tsx
interface QrCodeDisplayProps {
  localIp: string;
  port: number;
}

export function QrCodeDisplay({ localIp, port }: QrCodeDisplayProps) {
  const downloadUrl = `http://${localIp}:${port}/ca`;

  return (
    <div className="qr-code-container">
      <h2>Install CA Certificate</h2>
      <div className="qr-code-wrapper">
        <img
          src={`data:image/svg+xml,<svg xmlns="http://www.w3.org/2000/svg">...</svg>`}
          alt="QR Code"
          className="qr-code-image"
        />
        {/* Actually fetch from API that returns generated QR */}
      </div>
      <div className="install-instructions">
        <h3>Installation Steps:</h3>
        <ol>
          <li>Open your phone's camera</li>
          <li>Scan the QR code above</li>
          <li>Tap the notification to download the profile</li>
          <li>Go to Settings → General → VPN & Device Management</li>
          <li>Tap "Install" on the ProxyBot CA profile</li>
        </ol>
      </div>
      <div className="fallback-link">
        <p>Can't scan? <a href={downloadUrl}>Download CA directly</a></p>
      </div>
    </div>
  );
}
```

## 5. Mobile CA Profile

### 5.1 iOS CA Profile Format

iOS expects a specific .mobileconfig XML format:

**File:** `src-tauri/src/mobile_profile.rs`

```rust
pub fn generate_ios_profile(cert_pem: &str) -> String {
    let cert_der = pem_to_der(cert_pem);
    let cert_base64 = general_purpose::STANDARD.encode(&cert_der);

    format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>PayloadContent</key>
    <array>
        <dict>
            <key>PayloadCertificateFileName</key>
            <string>ProxyBot_CA.crt</string>
            <key>PayloadContent</key>
            <data>{cert_base64}</data>
            <key>PayloadDescription</key>
            <string>ProxyBot Root CA Certificate</string>
            <key>PayloadDisplayName</key>
            <string>ProxyBot CA</string>
            <key>PayloadIdentifier</key>
            <string>com.proxybot.ca</string>
            <key>PayloadType</key>
            <string>com.apple.security.root</string>
            <key>PayloadUUID</key>
            <string>{uuid}</string>
            <key>PayloadVersion</key>
            <integer>1</integer>
        </dict>
    </array>
    <key>PayloadDisplayName</key>
    <string>ProxyBot CA</string>
    <key>PayloadIdentifier</key>
    <string>com.proxybot.ca.profile</string>
    <key>PayloadRemovalDisallowed</key>
    <false/>
    <key>PayloadType</key>
    <string>Configuration</string>
    <key>PayloadUUID</key>
    <string>{profile_uuid}</string>
    <key>PayloadVersion</key>
    <integer>1</integer>
</dict>
</plist>"#,
        cert_base64 = cert_base64,
        uuid = generate_uuid(),
        profile_uuid = generate_uuid(),
    )
}

fn pem_to_der(pem: &str) -> Vec<u8> {
    // Parse PEM and extract DER bytes
    // PEM format: -----BEGIN CERTIFICATE-----\n ... \n-----END CERTIFICATE-----
    let contents = pem
        .lines()
        .filter(|l| !l.starts_with("-----"))
        .collect::<Vec<_>>()
        .join("");

    general_purpose::STANDARD.decode(contents.as_bytes()).unwrap()
}
```

### 5.2 Endpoints on Cert Server

**File:** `src-tauri/src/cert_server.rs`

Add these endpoints:

```rust
// In the existing request handler:

match request.url().path() {
    "/ca" => {
        // Existing: serve raw PEM certificate
        serve_cert_pem(request, cert_path.clone())
    }
    "/ca.cer" => {
        // NEW: Serve DER-encoded certificate for Android
        serve_cert_der(request, cert_path.clone())
    }
    "/ca.mobileconfig" => {
        // NEW: Serve iOS profile
        serve_ios_profile(request, cert_path.clone())
    }
    "/qr.svg" => {
        // NEW: Generate and serve QR code SVG
        serve_qr_svg(request, cert_path.clone(), local_ip.clone())
    }
    _ => serve_404(request),
}
```

## 6. Installation Instructions by Platform

### 6.1 iOS

1. Scan QR code with Camera app
2. Notification appears: "Profile Downloaded"
3. Tap "Close" then Settings → General → VPN & Device Management
4. Find "Profile Downloaded" section
5. Tap "Install" and confirm

**Alternative direct download:**
1. Open Safari (not Chrome)
2. Navigate to `http://{ip}:19876/ca.mobileconfig`
3. Follow install prompts

### 6.2 Android

1. Scan QR code with Camera or QR Scanner app
2. Browser downloads `ca.cer` file
3. Go to Settings → Security → Install certificates
4. Select the downloaded certificate
5. Name it "ProxyBot CA" and confirm

**Note:** Android requires user to set up a screen lock for certificate installation.

## 7. UI/UX Design

### 7.1 First-Time Setup Flow

When user enables MITM for the first time:
1. Show dialog explaining CA certificate is needed
2. Generate and save CA to filesystem
3. Display QR code in modal
4. Show platform-specific instructions
5. "I've installed it" button to confirm

### 7.2 Settings Panel

Add dedicated section in settings:
- CA Certificate status (installed/not installed)
- "Show QR Code" button
- "Reinstall CA" option (regenerates if needed)
- Device compatibility check

## 8. Security Considerations

### 8.1 Local-Only Access

- Cert server binds to `localhost` or LAN IP only
- No external access
- QR code is only scannable on the same network

### 8.2 Certificate Validation

- Verify CA certificate before generating QR code
- Check certificate hasn't expired
- Warn if CA is close to expiration (2 years default)

### 8.3 Network Security

- Plain HTTP for download (no TLS needed since CA is public anyway)
- Consider TLS for enterprises (self-signed on localhost)

## 9. Dependencies

```toml
# Cargo.toml additions
qrcode = "0.14"
base64 = "0.22"
image = { version = "0.25", default-features = false, features = ["png"] }
```

## 10. Error Handling

| Scenario | Handling |
|----------|----------|
| CA file not found | Generate new CA or show error |
| QR generation fails | Show download URL instead |
| Mobile device incompatible | Show manual download instructions |
| Network interface down | Detect and show static IP instructions |

## 11. Testing

### Manual Testing Checklist

- [ ] QR code scans correctly on iOS
- [ ] QR code scans correctly on Android
- [ ] iOS profile installs correctly
- [ ] Android certificate installs correctly
- [ ] TTY display renders correctly (ASCII fallback)
- [ ] Web UI displays QR correctly

### Edge Cases

- Multiple network interfaces (choose correct one)
- Firewall blocking port
- Very long hostnames in URL
- CA renewal flow