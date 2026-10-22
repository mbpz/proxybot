# Contributing to ProxyBot

Thanks for helping improve ProxyBot. Bug reports, documentation fixes, tests,
and focused code changes are welcome.

ProxyBot is deliberately lightweight: a macOS MITM traffic debugger built around
one explicit-proxy workflow. Please keep changes inside that scope, and check
[docs/adr/](docs/adr/README.md) before proposing an architectural change.

## Before opening a change

1. Search existing GitHub issues and pull requests.
2. Open an issue first for security-sensitive work, large user-facing changes,
   or changes to the proxy, certificate, persistence, IPC contract, or release
   architecture.
3. Keep pull requests focused on one coherent outcome.
4. Do not reintroduce a removed capability (for example a removed experimental
   toolchain) or a Labs feature into the default navigation without a new ADR and
   a supported user journey.

## Development setup

ProxyBot currently targets macOS. Install a stable Rust toolchain, Node.js 20 or
newer, pnpm 10, and the Tauri prerequisites. Source builds are self-contained:
there are no resource downloads and no optional native runtime feature.

```bash
pnpm install --frozen-lockfile
pnpm ci:local
pnpm tauri dev
```

`pnpm ci:local` runs the local gate: version consistency, formatting, Rust
tests, clippy, TypeScript checks, UI tests, and the frontend production build.
CI additionally runs the generated-contract check, parity checks, the Playwright
e2e suite, and the packaged desktop acceptance journey.

## Change expectations

- Add or update tests for observable behavior.
- Run `pnpm ci:local` before requesting review, and report exactly what you ran.
- Update user documentation when commands, configuration, or workflows change.
- Keep the generated Desktop Contract generated: change Rust, run the contract
  generator, and commit the regenerated TypeScript. Do not hand-edit it.
- Do not convert IPC failures into `null` or empty state; surface typed,
  retryable errors.
- Do not commit generated outputs, downloaded bundle resources, credentials,
  certificates, captured traffic, or user data.
- Explain any new unsafe code, elevated-privilege behavior, network listener, or
  certificate handling in the pull request.

## Architecture decisions (ADRs)

Write an ADR in [`docs/adr/`](docs/adr/README.md) when a change:

- alters a public interface or the desktop IPC contract;
- changes the persistence model or the owner of persisted facts;
- changes a security boundary (certificates, listeners, privileges, CSP,
  capabilities, secrets);
- changes the release or version contract;
- moves a capability between Core, Advanced, and Labs.

Use the next ADR number, follow the existing format (Status / Context / Decision
/ Consequences / Evidence), link it from the ADR index, and supersede rather than
rewrite accepted decisions.

## Documentation map

- [`CONTEXT.md`](CONTEXT.md) — domain terminology (use these terms in code,
  tests, issues, and docs).
- [`docs/architecture.md`](docs/architecture.md) — current runtime composition.
- [`docs/roadmap.md`](docs/roadmap.md) — product tiers and direction.
- [`docs/adr/`](docs/adr/README.md) — binding decisions.
- [`docs/history/`](docs/history/README.md) — historical records; not supported
  behavior and not part of the published site.
- [`SUPPORT.md`](SUPPORT.md), [`CHANGELOG.md`](CHANGELOG.md),
  [`SECURITY.md`](SECURITY.md).

## Clean-room compatibility work

- Only public Community behavior at the pinned evidence snapshot is eligible for
  compatibility work.
- Do not copy Rockxy source, tests, fixtures, copy, icons, images, private or
  Pro behavior, or results from reverse engineering an official DMG into
  ProxyBot.
- Independently author implementations and fixtures from public protocols,
  ProxyBot requirements, and observable behavior.
- Compatibility pull requests must update the parity row, cite pinned evidence,
  name the acceptance test, and preserve ProxyBot's MIT-compatible provenance.
  See [ADR-0008](docs/adr/0008-rockxy-clean-room-parity.md).

By participating, you agree to follow [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).
