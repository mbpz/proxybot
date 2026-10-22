# ADR-0005: Product tiers: Core, Advanced, Labs

- **Status:** Accepted
- **Date:** 2026-10-22
- **Deciders:** ProxyBot maintainers

## Context

ProxyBot accumulated a large surface of experiments: multiple capture modes,
protocol decoders, generators, an AI pipeline, a mobile dashboard, and
automation adapters. Mixing them into the default navigation and release claims
made the product look broader than it was reliable. A first-time user must be
able to reach one trustworthy decrypted request without navigating experimental
features, and a release must not imply support for capabilities without a
supported journey.

## Decision

ProxyBot classifies every capability into one of three tiers:

1. **Core** — the supported path. Visible in the default navigation, documented,
   covered by a real-adapter acceptance test, and eligible for release claims.
   Core is the explicit-proxy capture workflow: capture, setup/onboarding,
   inspect, modify, replay, export, and configure.
2. **Advanced** — capabilities that are documented after Core is stable and
   require an explicit user action: macOS `pf` routing, the built-in DNS server,
   MCP stdio, scripting, and the mobile dashboard. They are not required for
   first success.
3. **Labs** — experiments, not product promises. Labs are hidden behind an
   explicit opt-in, excluded from the default navigation, and excluded from
   release claims. A Labs capability graduates only when it has an owner, a
   supported user journey, a failure model, documentation, and an acceptance
   test through its real adapter.

The default navigation exposes only Core destinations. After the 1.3
convergence (ADR-0007) these are Capture, Setup, and Settings; request-derived
views and related tools live inside the Capture context rather than competing
as peers, and existing deep links remain valid.

## Consequences

- Product claims are limited to Core; Advanced is documented with prerequisites;
  Labs never appears in release notes as supported.
- Labs can be deleted without breaking a support promise (as happened with
  Android SSL bypass in ADR-0007).
- Feature work must state its tier and what leaves the default surface.
- "Supported" and "done" have explicit meanings: documented journey,
  real-adapter test, maintained owner, failure and cleanup behavior, and
  evidence on the release artifact.

## Evidence

- `docs/roadmap.md` ("Product contract", "Labs" graduation criteria)
- `docs/comparison.md` ("Default mode policy")
- `CONTEXT.md` (`Product Destination` definition)
- `docs/adr/0007-lightweight-positioning.md`
- `src/main.tsx` (Core navigation plus preserved deep links)
