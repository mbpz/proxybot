# Security policy

ProxyBot handles decrypted traffic and local certificates on macOS and Windows,
and elevated packet-filter operations on macOS. Please report suspected
vulnerabilities privately.

## Reporting a vulnerability

Use GitHub's **Security** tab to submit a private vulnerability report. Do not open a public issue for an unpatched vulnerability and do not include real captured traffic, private keys, access tokens, or personal data in a report.

Include the affected version or commit, impact, reproduction steps, and any suggested mitigation. Maintainers will acknowledge a complete report as soon as practical, validate the issue, and coordinate disclosure and a fix.

## Supported versions

The active CLI uses the `minimal-v<version>` release line. Security fixes are
prepared on `main` and released on the latest CLI release line.

- **Supported:** the latest published release line and `main`.
- **Not supported:** older releases and preview/prerelease artifacts — upgrade to
  the latest release before requesting a security fix.

Platform support claims are limited to the tested and published assets. See
[current status](docs/status.md) for the latest verification boundary.

## Scope note

Installing a local CA and intercepting traffic intentionally changes the device trust model. Only inspect devices, applications, and traffic you own or are explicitly authorized to test.
