# Domain documentation

How engineering skills and agents should consume this repository's domain
knowledge. This is a **single-context** repository: one `CONTEXT.md` at the
repository root and one shared ADR directory.

## Before exploring, read these

1. [`CONTEXT.md`](../../CONTEXT.md) at the repository root — canonical domain
   vocabulary. Use its terms (and avoid the listed synonyms) in code, tests,
   issue titles, refactor proposals, and documentation.
2. [`docs/adr/`](../adr/README.md) — binding architecture and product decisions.
   Read the ADRs that touch the area you are about to work on. If your work
   contradicts an accepted ADR, surface it explicitly and propose a superseding
   ADR rather than silently overriding the decision.
3. [`docs/roadmap.md`](../roadmap.md) — product tiers (Core / Advanced / Labs)
   and current direction.

Do not create a second `CONTEXT.md`, a `CONTEXT-MAP.md`, or context-scoped ADR
directories. ProxyBot is one context.

## Vocabulary discipline

- If a domain concept you need is not in `CONTEXT.md`, either the term is
  invented (reconsider) or the glossary needs a change (propose it with the
  code change that introduces the concept).
- When proposing a capability, state its product tier: Core, Advanced, or Labs.
  Tier definitions are in [ADR-0005](../adr/0005-product-tiers.md).

## When to record a decision

Write an ADR when a change alters a public interface, the desktop IPC contract,
the persistence model, a security boundary, the release contract, or a product
tier. Do not write one for behavior-preserving refactors or routine fixes.
