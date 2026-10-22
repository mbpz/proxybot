# ProxyBot support

ProxyBot is a lightweight macOS MITM traffic debugger for mobile application
developers. This file describes what is supported, what is experimental, and
where to get help. It does not replace [SECURITY.md](SECURITY.md) for
vulnerability reports.

## Supported configuration

| Item | Status | Notes |
| --- | --- | --- |
| macOS desktop app | Supported (preview) | Built and tested on GitHub Actions `macos-latest`; no minimum macOS version is declared or promised yet. |
| Apple Silicon (arm64) | Supported (preview) | Release pipeline builds arm64 DMGs. |
| Intel (x86_64) | Supported (preview) | Release pipeline builds x64 DMGs. |
| Explicit HTTP proxy capture | Supported | Core first-capture path; default in the getting-started guide. |
| HTTPS interception with local CA | Supported | Requires installing and trusting the ProxyBot CA on a test device. |
| Certificate export and device-setup server | Supported | Temporary local server; stop it after setup. |
| Routing Rules, breakpoints, replay/Composer, HAR/request-code export | Supported | Core workflow; see the user guide. |
| macOS `pf` routing and built-in DNS server | Advanced | Changes host network state, may require administrator rights, and needs explicit cleanup. Not required for first success. |
| MCP stdio adapter | Advanced | Local, experimental integration over persisted data; no headless capture mode. |
| Scripting (Rhai), mobile dashboard, protocol analysis | Advanced | Documented after the Core workflow; may change. |
| AI analysis, spec/mock/scaffold/deployment generation | Labs | Opt-in experiments; not in the default navigation and not covered by support claims. |
| Windows / Linux | Not supported | A future possibility, not a current promise. |
| Instrumented SSL bypass / APK patching | Removed in 1.3.0 | See [ADR-0007](docs/adr/0007-lightweight-positioning.md); use a dedicated external tool if you need this. |
| Certificate-pinned apps | Not guaranteed | Pinning is an application security boundary; ProxyBot may still expose DNS/SNI metadata. |

The latest published artifact is a development preview (`v1.2.15`,
2026-05-15). Signed, notarized, install-verified releases are only claimed once
the hosted release workflow has produced them and both architectures have been
installed and started successfully; until then treat every artifact as a
preview.

## Version support policy

- Until the first verified stable release, fixes are made on the latest release
  line and `main`. There is no long-term-support branch.
- The Product Version is the single version in `package.json`; release tags must
  match `v<version>`.
- Security fixes follow [SECURITY.md](SECURITY.md). Vulnerabilities are handled
  privately through GitHub Security Advisories.
- Breaking changes to persisted data, the desktop contract, or the release
  contract are recorded in [CHANGELOG.md](CHANGELOG.md) and, where still
  binding, in [docs/adr/](docs/adr/README.md).

## Source builds

Source builds are the currently supported contributor path. Requirements:
macOS, Xcode Command Line Tools, a stable Rust toolchain, Node.js 20 or newer,
and pnpm 10. Builds no longer download external native tooling.

```bash
pnpm install --frozen-lockfile
pnpm ci:local
pnpm tauri dev
```

See [CONTRIBUTING.md](CONTRIBUTING.md) and
[docs/getting-started.md](docs/getting-started.md).

## Getting help

- **Bug reports / feature requests:** open a GitHub issue using the repository
  templates. Include the ProxyBot version, macOS version and architecture, and
  device platform. Never attach captured payloads, private keys, tokens, or
  certificates.
- **Documentation:** [docs/](docs/index.md) (published site), starting with the
  getting-started guide; [CONTEXT.md](CONTEXT.md) for domain terminology;
  [CHANGELOG.md](CHANGELOG.md) for changes.
- **Security vulnerabilities:** do not open a public issue; use GitHub Security
  Advisories as described in [SECURITY.md](SECURITY.md).
- **Historical material:** files under [docs/history/](docs/history/README.md)
  are research records, not supported behavior.

## Known limitations

- Physical iOS and Android device validation is tracked as an open item; the
  supported evidence today is the packaged desktop acceptance journey and unit
  / UI / e2e coverage.
- Browser e2e tests use a mock desktop adapter and do not prove real capture.
- Homebrew is not supported until a maintained tap installs a verified artifact.
