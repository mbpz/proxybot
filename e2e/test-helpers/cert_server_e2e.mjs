import http from "node:http";

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
