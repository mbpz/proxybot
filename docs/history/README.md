# Historical records

This directory preserves research, plans, product proposals, and specifications
written before or during earlier ProxyBot iterations. It exists for decision
archaeology and for parity evidence references — **not** as proof that a feature
is supported, complete, or present in the current product.

These files are excluded from the published documentation site
(`exclude_docs` in `mkdocs.yml`) and are not covered by support or release
claims. They are kept in the repository instead of being deleted so that
historical references (for example, the Rockxy parity evidence ledger) keep
working.

## Layout

| Directory | Contents |
| --- | --- |
| `sdd/` | Earlier product and design research (competitor analysis, HTTP/3 and iOS VPN research, old PRDs). |
| `superpowers/` | Earlier implementation plans and design specifications, including superseded workstreams. |
| `prd/` | Earlier PRDs, including the unrelated "NetMind Agent" vision and an old UI redesign proposal. |
| `tauri/` | A testing strategy for the removed TUI client, retained for context. |

## Rules for readers

1. Anything in this directory may describe removed interfaces, unsupported
   experiments, stale competitor data, or implementations that never reached
   the desktop composition root.
2. When sources disagree, use this order: current code and executable tests →
   [`CONTEXT.md`](../../CONTEXT.md) → [`docs/adr/`](../adr/README.md) →
   [`docs/roadmap.md`](../roadmap.md) → current user documentation → this
   directory.
3. Before implementing an old proposal, open a focused issue and revalidate its
   user need, product tier, security boundary, and real-adapter acceptance test.
4. Do not cite a file here as evidence that a capability is shipped.

## Related

- [`docs/adr/`](../adr/README.md) — the decisions that are still binding.
- [`CONTEXT.md`](../../CONTEXT.md) — current domain language.
- [`docs/roadmap.md`](../roadmap.md) — current product tiers and direction.
