# Security policy

ProxyBot handles decrypted traffic, local certificates, network routing, and elevated macOS packet-filter operations. Please report suspected vulnerabilities privately.

## Reporting a vulnerability

Use GitHub's **Security** tab to submit a private vulnerability report. Do not open a public issue for an unpatched vulnerability and do not include real captured traffic, private keys, access tokens, or personal data in a report.

Include the affected version or commit, impact, reproduction steps, and any suggested mitigation. Maintainers will acknowledge a complete report as soon as practical, validate the issue, and coordinate disclosure and a fix.

## Supported versions

ProxyBot is on the 1.x release line. Security fixes are prepared on `main` and
released on the latest stable release line.

- **Supported:** the latest published release line and `main`.
- **Not supported:** older releases and preview/prerelease artifacts — upgrade to
  the latest release before requesting a security fix.

Until the first signed, notarized, and install-verified release is published,
published artifacts are development previews; support evidence for them is
limited to the latest preview line and `main`.

## Scope note

Installing a local CA and intercepting traffic intentionally changes the device trust model. Only inspect devices, applications, and traffic you own or are explicitly authorized to test.
