# Releasing ProxyBot

Public releases are produced only by `.github/workflows/release.yml`. A local
build is useful for development, but it is not a public release unless the
hosted workflow signs, notarizes, verifies, attests, and publishes it.

## Product version

`package.json` is the canonical product version. Tauri and the update UI read
that file directly, and the Rust/MCP versions are checked against it. The stable
macOS bundle identifier is `com.mbpz.proxybot`.

To prepare a version bump:

```bash
pnpm version:set 1.3.1
pnpm version:check
pnpm ci:local
```

Commit the version change before creating the tag. The Release workflow rejects
any tag that does not exactly match `v<package version>`.

## Release gates

The `validate` job runs on a clean runner before any mac build starts:

```bash
node scripts/version.mjs --check --tag "$RELEASE_TAG"
pnpm version:check
pnpm typecheck
pnpm test:ui
pnpm parity:check
```

A failing gate stops the release. The build then runs for both architectures
(Apple Silicon `aarch64-apple-darwin` and Intel `x86_64-apple-darwin`) and uses
the Tauri bundler only — there is no resource download or extra bundle configuration step.

## Required repository secrets

The Release workflow intentionally fails instead of publishing an ad-hoc-signed
application when any credential is absent:

- `APPLE_CERTIFICATE` — base64-encoded Developer ID Application certificate
- `APPLE_CERTIFICATE_PASSWORD`
- `APPLE_SIGNING_IDENTITY`
- `APPLE_ID`
- `APPLE_PASSWORD` — app-specific password
- `APPLE_TEAM_ID`
- `KEYCHAIN_PASSWORD` — ephemeral CI keychain password

Keep these values in GitHub Actions secrets. Never store a certificate, private
key, or password in the repository. Provisioning and hosted evidence are tracked
in [GitHub issue #27](https://github.com/mbpz/proxybot/issues/27); physical-device
evidence is tracked in
[issue #28](https://github.com/mbpz/proxybot/issues/28).

## Provision signing credentials (issue #27)

Signed releases require an Apple Developer Program membership. CI imports the
Developer ID certificate from repository secrets, so the release machine itself
does not need a signing identity. Never commit a certificate, private key, or
password.

1. In [developer.apple.com](https://developer.apple.com/account/resources/certificates/list),
   create a **Developer ID Application** certificate, download the `.cer`,
   import it into Keychain, and export it as a password-protected `.p12`.
   Confirm it is visible with:

   ```bash
   security find-identity -v -p codesigning
   ```

2. Create an app-specific password at [appleid.apple.com](https://appleid.apple.com).
3. Find your Team ID on the Apple Developer account page.
4. Encode the certificate and set all seven repository secrets:

   ```bash
   base64 -i DeveloperID.p12 | tr -d '\n' > cert.b64

   gh secret set APPLE_CERTIFICATE < cert.b64
   gh secret set APPLE_CERTIFICATE_PASSWORD       # .p12 export password
   gh secret set APPLE_SIGNING_IDENTITY --body "Developer ID Application: Your Name (TEAMID)"
   gh secret set APPLE_ID
   gh secret set APPLE_PASSWORD                   # app-specific password
   gh secret set APPLE_TEAM_ID
   gh secret set KEYCHAIN_PASSWORD --body "$(openssl rand -base64 24)"

   rm cert.b64 DeveloperID.p12
   ```

5. Verify names only (never values):

   ```bash
   ./scripts/release-preflight.sh
   gh secret list
   ```

The preflight fails while any secret is absent, which is intentional: the
workflow refuses to publish an ad-hoc-signed application.

## Publish (manual)

The Release workflow is **manual-only**. Pushing a tag no longer starts a
build; the tag must still exist on `origin` because the workflow checks out
`ref: vX.Y.Z`.

After the version commit is on a green `main`:

```bash
git tag -s v1.3.1 -m "ProxyBot v1.3.1"
git push origin v1.3.1

# Checks tag, auth, and secrets, then dispatches the workflow:
./scripts/release-preflight.sh --dispatch

# Or dispatch directly:
gh workflow run release.yml -f version=v1.3.1
gh run list --workflow=release.yml --limit 1
```

The workflow builds both DMGs through the Tauri bundler. For each architecture it
publishes:

- a signed and notarized `.dmg`;
- a SHA-256 checksum;
- an SPDX JSON SBOM;
- a GitHub build-provenance attestation.

Before upload, CI mounts the DMG, checks its `CFBundleShortVersionString` and
bundle identifier, and uses `codesign`, `spctl`, and `stapler` to verify the
application. It then runs the packaged executable's isolated desktop acceptance
journey: prepare the CA, decrypt and persist one local HTTPS request, stop
capture, restart it, and stop again. A single publish job waits for both
architectures and generates release notes from the accepted tag.

## Local builds are not releases

`pnpm build:tauri` (that is, `tauri build`) produces an **unsigned, not
notarized** app bundle for local development. It is useful for checking that the
app packages and starts, but it is not evidence of signing, notarization, or a
verified upgrade path. Only the hosted workflow produces a Release Artifact. The
local output may include an unsigned `.app` and a local DMG; neither should be
linked as a download for users.

The CI `bundle-smoke` job also builds an unsigned app bundle and runs the
packaged acceptance journey. Passing it proves the backend journey from the
packaged binary; it does not prove signed installation or visible UI behavior.

## Release evidence boundary

A workflow definition is not proof that a release succeeded. Record the hosted
run URL and the results of installing and starting both architectures before
calling a release verified. Use the
[device validation template](https://github.com/mbpz/proxybot/blob/main/docs/status/device-validation-template.md) for the
physical iOS/Android evidence tracked in issue #28. Homebrew remains unsupported
until a maintained tap installs one of these verified artifacts and passes its
own smoke test.

Until the first such release exists, all published artifacts are development
previews. The latest published artifact is `v1.2.15` (2026-05-15); the unreleased
1.3.0 line is the current source of truth.
