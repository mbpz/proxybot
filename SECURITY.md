# Security policy

ProxyBot handles decrypted traffic, local certificates, network routing, and elevated macOS packet-filter operations. Please report suspected vulnerabilities privately.

## Reporting a vulnerability

Use GitHub's **Security** tab to submit a private vulnerability report. Do not open a public issue for an unpatched vulnerability and do not include real captured traffic, private keys, access tokens, or personal data in a report.

Include the affected version or commit, impact, reproduction steps, and any suggested mitigation. Maintainers will acknowledge a complete report as soon as practical, validate the issue, and coordinate disclosure and a fix.

## Supported versions

Until ProxyBot reaches a stable 1.x release, security fixes are made on the latest release line and `main`. Older prereleases may be asked to upgrade before a fix is provided.

## Scope note

Installing a local CA and intercepting traffic intentionally changes the device trust model. Only inspect devices, applications, and traffic you own or are explicitly authorized to test.
