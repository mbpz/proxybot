# Contributing to ProxyBot

ProxyBot's current application is the Rust CLI and embedded web UI in
`minimal/`. The explicit HTTP proxy targets macOS and Windows x64. Gateway Beta
and its system integration target macOS only.

## Development

Install a stable Rust toolchain, then run:

```bash
cargo test --locked --manifest-path minimal/Cargo.toml
cargo build --release --locked --manifest-path minimal/Cargo.toml
```

CI runs these checks on macOS and Windows. For a local release package use
`sh minimal/package.sh` on macOS or `./minimal/package.ps1` in Windows
PowerShell. Test the CLI on each platform before claiming that platform's
runtime behavior is verified.

Keep changes focused, add tests for observable behavior, and update the user
documentation when commands or support boundaries change. Never commit local
CA keys, captured traffic, secrets, generated binaries, or third-party
downloads. Explain new elevated privileges, listeners, or certificate handling
in a pull request. See [the architecture](docs/architecture.md) and
[accepted decisions](docs/adr/README.md).

By participating, you agree to follow [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).
