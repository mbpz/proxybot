# ProxyBot — repository guide

## What this is

ProxyBot is a **lightweight macOS MITM traffic debugger** for mobile application
developers. A test device (iOS or Android) points its Wi-Fi HTTP proxy at a Mac
running ProxyBot; the Mac terminates TLS with a locally generated CA, then lets
the developer inspect, filter, modify, replay, and export captured HTTP, HTTPS,
and WebSocket traffic.

The product is deliberately narrow: explicit proxy first, one Core workflow,
no feature-count competition. See [docs/roadmap.md](docs/roadmap.md) and
[docs/adr/0005-product-tiers.md](docs/adr/0005-product-tiers.md).

## Stack

- **Core:** Rust — `proxybot-core`, shared MITM engine (hyper, rustls, tokio).
- **Desktop:** Tauri v2 (Rust) + React 19 + TypeScript, Tailwind-based UI.
- **Persistence:** SQLite (captured requests, devices, alerts, DNS observations).
- **IPC:** generated Desktop Contract (Rust → typed TypeScript), not hand-written
  invoke wrappers.

## Repository map

```
proxybot-core/   reusable MITM engine, certificates, rules, analysis models
src-tauri/       desktop composition root (bootstrap.rs), adapters, persistence, MCP
src/             React UI, generated contract, desktop adapters
e2e/             browser-mock Playwright tests
docs/            current docs, adr/, parity/, agents/, history/, zh/
scripts/         development, version, and parity tooling
```

`src-tauri/src/bootstrap.rs` is the single composition root. Do not create a
second entry point that parses config or opens the database.

## Commands

```bash
pnpm install --frozen-lockfile   # install JS deps
pnpm tauri dev                   # run the desktop app
pnpm ci:local                    # local gate: version, fmt, tests, clippy, typecheck, UI tests, build
cargo test --workspace --locked --no-default-features
cargo clippy --workspace --all-targets --locked --no-default-features -- -D warnings
pnpm version:check               # version consistency across metadata
```

There are no downloads of external native toolchains and no optional runtime
feature; source builds are self-contained. Run `pnpm ci:local` before opening a pull request.

## Documentation map

- [`CONTEXT.md`](CONTEXT.md) — canonical domain vocabulary; use its terms.
- [`docs/adr/`](docs/adr/README.md) — binding architecture and product decisions.
- [`docs/roadmap.md`](docs/roadmap.md) — product tiers (Core / Advanced / Labs)
  and direction.
- [`docs/architecture.md`](docs/architecture.md) — current runtime composition.
- [`docs/history/`](docs/history/README.md) — historical research; **not**
  supported behavior and not published to the docs site.
- [`SUPPORT.md`](SUPPORT.md), [`CHANGELOG.md`](CHANGELOG.md),
  [`SECURITY.md`](SECURITY.md), [`CONTRIBUTING.md`](CONTRIBUTING.md).

## Rules

1. **Product tiers are binding.** Only Core is a support claim. Labs never
   appears in the default navigation or release notes; Advanced requires an
   explicit user action.
2. **IPC goes through the generated Desktop Contract** (see ADR-0004). Failures
   stay visible typed errors; do not convert them into `null` or empty data.
3. **Write an ADR** when a change alters a public interface, persistence model,
   security boundary, release contract, or product tier. Decisions live in
   `docs/adr/`; do not rewrite accepted ADRs — supersede them.
4. **One version source.** `package.json` owns the Product Version; use
   `pnpm version:set` / `pnpm version:check`, never hand-edit versions.
5. **Do not commit** generated build outputs, captured traffic, certificates,
   private keys, secrets, or downloaded third-party assets.
6. **Verify what you claim.** Run the commands you report; do not describe tests
   you did not execute. Keep docs truthful — if code changes behavior, update
   the docs in the same change.
