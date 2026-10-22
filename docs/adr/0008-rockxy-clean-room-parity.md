# ADR-0008: Rockxy Community clean-room parity policy

- **Status:** Accepted
- **Date:** 2026-10-22
- **Deciders:** ProxyBot maintainers

## Context

ProxyBot tracks a capability comparison against the public Rockxy Community
repository to understand where the product stands and to plan work. The
reference project is AGPL-3.0-or-later, while ProxyBot is MIT. Copying source,
tests, fixtures, assets, copy, or icons from the reference would create a
licensing and provenance problem and would undermine ProxyBot's independent
implementation.

## Decision

1. Compatibility work uses only public Community behavior at a pinned reference
   commit. The official downstream DMG and private/Pro behavior are explicitly
   excluded and recorded as boundary rows, not implementation targets.
2. Implementations, tests, fixtures, and copy are independently authored from
   public protocols, ProxyBot requirements, and observable behavior. Never copy
   reference source, tests, assets, icons, images, or product copy.
3. The machine-readable parity matrix is the authority for capability evidence,
   status, owner, and acceptance. `Present` requires independently test-backed
   ProxyBot evidence; `Partial` means a source or test seam exists without full
   parity; `Missing` is planned only; scope-boundary rows are recorded
   separately.
4. A parity pull request must update the matrix row, cite the pinned evidence,
   name the acceptance test, and preserve MIT-compatible provenance.
5. The matrix is validated by a repository script that checks the schema,
   pinned reference metadata, status vocabulary, and repository-relative
   evidence paths.
6. Parity work must not waive ProxyBot's first-success, redaction, security, or
   release gates. Feature-count parity is never a reason to ship an unsupported
   journey.

## Consequences

- ProxyBot can learn from the reference's interaction model without inheriting
  its license or implementation.
- Capability claims are backed by repository evidence, not by a README claim.
- Moving or deleting evidence paths (for example, archiving historical plans)
  requires updating the matrix or the check script.
- The parity work is internal tracking, not a public product promise.

## Evidence

- `docs/parity/rockxy-community-evidence.md` (fixed reference and boundaries)
- `docs/parity/rockxy-community-matrix.md` (47-row capability matrix)
- `scripts/check_parity_matrix.mjs` (matrix validation)
- `CONTRIBUTING.md` ("Clean-room compatibility work")
- `docs/history/superpowers/plans/` (archived parity implementation plans)
