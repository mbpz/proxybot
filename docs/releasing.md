# Releasing ProxyBot CLI

The minimal CLI is released by `.github/workflows/minimal-release.yml`. The
workflow builds macOS arm64 and x64 archives plus a Windows x64 ZIP, runs tests
on each platform, and publishes them with checksums, `install.sh` and
`uninstall.sh` for macOS.

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
3. Commit and push `main`; wait for Minimal CLI CI to pass on that commit.
4. Push an annotated tag matching the Cargo version:
   ```bash
   git tag -a minimal-v2.0.0 -m "ProxyBot CLI 2.0.0"
   git push origin minimal-v2.0.0
   ```
   The tag push starts `minimal-release.yml`. A manual dispatch remains
   available, but the workflow rejects a version that differs from Cargo.
5. Wait for tag/version validation, both macOS builds, the Windows build, and
   publishing to succeed.
6. Verify the downloaded asset checksum and run the smoke test against the
   GitHub asset:
   ```bash
   gh release download minimal-v2.0.0 -D /tmp/release
   cd /tmp/release
   shasum -a 256 -c proxybot-min-aarch64-apple-darwin.tar.gz.sha256
   ```
7. Update external acceptance docs with the published checksums.

## Assets

- `proxybot-min-aarch64-apple-darwin.tar.gz` + `.sha256`
- `proxybot-min-x86_64-apple-darwin.tar.gz` + `.sha256`
- `proxybot-min-x86_64-pc-windows-msvc.zip` + `.sha256`
- `install.sh`, `install.sh.sha256`
- `uninstall.sh`, `uninstall.sh.sha256`

Archives contain `proxybot`, `LICENSE`, `THIRD_PARTY.md`, `uninstall.sh`; they
must not contain sing-box.
The Windows ZIP contains `proxybot.exe`, `LICENSE`, and `THIRD_PARTY.md`.

## CI

`.github/workflows/minimal-ci.yml` runs macOS and Windows tests, builds both
package formats, and checks their manifests. `.github/workflows/pages.yml`
publishes this documentation.
