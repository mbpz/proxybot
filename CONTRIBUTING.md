# Contributing to ProxyBot

Thanks for helping improve ProxyBot. Bug reports, documentation fixes, tests, and focused code changes are welcome.

## Before opening a change

1. Search existing GitHub issues and pull requests.
2. Open an issue first for security-sensitive work, large user-facing changes, or changes to the proxy, certificate, packet-filter, or release architecture.
3. Keep pull requests focused on one coherent outcome.

## Development setup

ProxyBot currently targets macOS. Install a stable Rust toolchain, Node.js 20 or newer, pnpm 10, and the Tauri prerequisites.

```bash
pnpm install --frozen-lockfile
pnpm ci:local
```

The APK patching assets are not stored in Git. They are only needed for Tauri bundles:

```bash
pnpm resources:fetch
pnpm build:tauri
```

Downloads are pinned and verified against `src-tauri/resources/resources.lock`.

## Change expectations

- Add or update tests for observable behavior.
- Run `pnpm ci:local` before requesting review.
- Update user documentation when commands, configuration, or workflows change.
- Do not commit generated outputs, downloaded bundle resources, credentials, certificates, captured traffic, or user data.
- Explain any new unsafe code, elevated-privilege behavior, network listener, or certificate handling in the pull request.

By participating, you agree to follow [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).
