# ADR-0006: Single Product Version and signed release contract

- **Status:** Accepted
- **Date:** 2026-10-22
- **Deciders:** ProxyBot maintainers

## Context

Earlier releases were assembled by ad-hoc scripts, shipped as unsigned ZIPs, and
could disagree with the version shown in the UI, the Rust crates, the MCP
adapter, and the Git tag. That makes support impossible: there is no reliable
answer to "which artifact corresponds to which source?". The project also
handles decrypted traffic and installs a local CA, so distribution must be
verifiable: signed, notarized, checksummed, and inspectable.

## Decision

1. `package.json` holds the single Product Version. The Tauri bundle reads it
   directly, the update UI reads it, and the Rust crates, Cargo.lock, and MCP
   metadata are checked against it by one consistency tool.
2. A release tag must be exactly `v<product version>`. The release workflow
   rejects any mismatch and any missing signing/notarization credential instead
   of publishing an ad-hoc artifact.
3. Public artifacts come only from the hosted Tauri bundler workflow. For each
   supported architecture the workflow builds a DMG through the bundler, signs
   and notarizes it, verifies the mounted application with `codesign`, `spctl`,
   and `stapler`, publishes a SHA-256 checksum, an SPDX SBOM, a build-provenance
   attestation, and runs the packaged desktop acceptance journey.
4. A local bundle is not a release artifact. A workflow definition is not proof
   of a release: evidence requires a hosted run URL plus install/start results
   for both architectures.
5. Until a verified signed release exists, published artifacts are previews.
   They must be marked as prereleases and must not be described as a stable
   distribution. Homebrew is not advertised until a maintained tap installs a
   verified artifact.

## Consequences

- Version changes are a single command (`pnpm version:set`) plus
  `pnpm version:check`; hand-editing versions is rejected by CI.
- A release is blocked, not degraded, when credentials are missing.
- Two architectures must both pass verification before publishing.
- The changelog and support policy must describe preview status honestly.

## Evidence

- `package.json` (`version`, `version:set`, `version:check`)
- `scripts/version.mjs` (consistency checks and tag gate)
- `src-tauri/tauri.conf.json` (`"version": "../package.json"`)
- `.github/workflows/release.yml` (signing, notarization, SBOM, attestation,
  packaged acceptance, single publish job)
- `docs/releasing.md` (maintainer runbook and evidence boundary)
