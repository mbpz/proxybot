# Releasing ProxyBot CLI

The minimal CLI is released by `.github/workflows/minimal-release.yml`. The
workflow builds macOS arm64 and x64 archives, verifies the package manifest, and
publishes them with checksums, `install.sh` and `uninstall.sh`.

## Version

`minimal/Cargo.toml` is the CLI version source. Keep `minimal/Cargo.lock` and
`minimal/CHANGELOG.md` in sync.

```bash
# after editing Cargo.toml
cargo test --locked --manifest-path minimal/Cargo.toml
sh minimal/package.sh
sh minimal/rc-smoke-test.sh
```

## Release

1. Bump `minimal/Cargo.toml` (and the `proxybot-min` lock entry).
2. Update `minimal/CHANGELOG.md` and release notes.
3. Commit and push `main`.
4. Trigger the manual workflow:
   ```bash
   gh workflow run minimal-release.yml --ref main -f version=0.1.13
   ```
5. Wait for both build jobs and publish to succeed.
6. Verify the downloaded asset checksum and run the smoke test against the
   GitHub asset:
   ```bash
   gh release download minimal-v0.1.13 -D /tmp/release
   cd /tmp/release
   shasum -a 256 -c proxybot-min-aarch64-apple-darwin.tar.gz.sha256
   ```
7. Update external acceptance docs with the published checksums.

## Assets

- `proxybot-min-aarch64-apple-darwin.tar.gz` + `.sha256`
- `proxybot-min-x86_64-apple-darwin.tar.gz` + `.sha256`
- `install.sh`, `install.sh.sha256`
- `uninstall.sh`, `uninstall.sh.sha256`

Archives contain `proxybot`, `LICENSE`, `THIRD_PARTY.md`, `uninstall.sh`; they
must not contain sing-box.

## CI

`.github/workflows/minimal-ci.yml` runs minimal tests, builds the local archive,
and checks its manifest/no-sing-box invariant. `.github/workflows/pages.yml`
publishes this documentation.
