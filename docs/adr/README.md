# Architecture Decision Records

Architecture Decision Records (ADRs) capture decisions that are binding for
ProxyBot: choices that affect a public interface, a persistence model, a
security boundary, the release contract, or the product tiers. They are
deliberately short; the code, tests, and [`CONTEXT.md`](https://github.com/mbpz/proxybot/blob/main/CONTEXT.md) remain
the authority on current behavior.

## Index

| ADR | Title | Status |
| --- | --- | --- |
| [0001](0001-single-composition-root-and-layering.md) | Single composition root and layered adapters | Accepted |
| [0002](0002-explicit-proxy-first.md) | Explicit proxy is the Core capture mode | Accepted |
| [0003](0003-capture-event-and-persistence-ownership.md) | Capture Event and SQLite persistence ownership | Accepted |
| [0004](0004-generated-desktop-contract.md) | Generated Desktop Contract as the IPC seam | Accepted (migration in progress) |
| [0005](0005-product-tiers.md) | Product tiers: Core, Advanced, Labs | Accepted |
| [0006](0006-single-product-version-and-release-contract.md) | Single Product Version and signed release contract | Accepted |
| [0007](0007-lightweight-positioning.md) | Lightweight positioning and removal of Android SSL bypass | Accepted |
| [0008](0008-rockxy-clean-room-parity.md) | Rockxy Community clean-room parity policy | Accepted |

## When to write an ADR

Write one when a change:

- alters a public interface or the desktop IPC contract;
- changes the persistence model or the owner of persisted facts;
- changes a security boundary (certificates, listeners, privileges, CSP,
  capabilities, secrets);
- changes the release or version contract;
- moves a capability between Core, Advanced, and Labs;
- records a decision the project is likely to revisit.

Bug fixes, refactors that preserve behavior, and routine dependency updates do
not need an ADR.

## Format

Every ADR uses the same sections:

- **Status** — `Proposed`, `Accepted`, `Deprecated`, `Superseded`, or `Rejected`.
- **Context** — the forces and constraints that made a decision necessary.
- **Decision** — what is now binding, in imperative language.
- **Consequences** — what becomes easier, what becomes harder, and what remains
  open.
- **Evidence** — repository paths or commands that verify the decision.

To add an ADR, copy the shape of an existing record, use the next number, and
link it from the index above. To change a decision, write a new ADR that
supersedes the old one instead of rewriting history.
