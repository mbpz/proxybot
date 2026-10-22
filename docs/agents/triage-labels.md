# Triage labels

This file maps the canonical triage roles used by engineering skills to the
labels that actually exist in this repository. It reflects `gh label list`; when
the label set changes, update this table.

| Canonical role | Label in this repository | Meaning |
| --- | --- | --- |
| `needs-triage` | `needs-triage` | Maintainer needs to evaluate this issue. Applied by the bug/feature forms. |
| `needs-info` | *(not defined)* | Use `question` plus a comment stating exactly what information is missing. |
| `ready-for-agent` | *(not defined)* | Not in use. Maintainers mark scoped work `ready-for-human` instead. |
| `ready-for-human` | `ready-for-human` | Fully specified; requires human implementation or external access (for example physical-device validation). |
| `wontfix` | `wontfix` | Will not be actioned. |

Other labels currently defined and used for routing:

- `bug`, `documentation`, `enhancement`, `question` — issue type.
- `good first issue`, `help wanted` — contributor-ready work.
- `duplicate`, `invalid` — issue state.
- `dependencies`, `rust`, `javascript`, `github_actions` — dependency/automation
  PRs (created by Dependabot).

If a skill asks for a canonical role that is not defined here, apply the closest
existing label and say so in a comment; do not invent a label that is not in
`gh label list`.
