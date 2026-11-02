# ProxyBot support

ProxyBot's active application is the CLI and local web UI in `minimal/`.

| Capability | Status |
| --- | --- |
| macOS arm64 and x64 explicit HTTP proxy | Supported default path |
| Windows x64 explicit HTTP proxy | Source implementation; Windows CI and device acceptance required before a support claim |
| macOS Gateway Beta (TUN + sing-box) | Beta; see [gateway guide](docs/gateway-beta.md) |
| Windows gateway, sniff, update, uninstall | Not implemented |
| Linux | Not supported |
| Certificate-pinned apps | HTTPS decryption not guaranteed |

The version source is `minimal/Cargo.toml`; releases use `minimal-v<version>`
tags. Build and test with `cargo test --locked --manifest-path minimal/Cargo.toml`.
See [getting started](docs/getting-started.md) and
[current status](docs/status.md) for verified behavior.

For bugs, open a GitHub issue with the CLI version, host OS and architecture,
and test device platform. Do not attach captured payloads, CA keys or tokens.
Report vulnerabilities privately through [SECURITY.md](SECURITY.md).
