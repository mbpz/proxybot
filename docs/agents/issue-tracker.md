# Issue tracker: GitHub

Issues, bug reports, and feature requests for this repository live as GitHub
issues. Use the `gh` CLI for all operations, from inside the clone.

## Conventions

- **Create an issue**: `gh issue create --title "..." --body "..."`. Use a
  heredoc for multi-line bodies. Prefer the repository issue forms
  (`bug_report.yml`, `feature_request.yml`) so version/environment fields are
  captured.
- **Read an issue**: `gh issue view <number> --comments --json
  number,title,body,labels,comments`.
- **List issues**: `gh issue list --state open --json
  number,title,body,labels,comments --jq '[...]'` with explicit `--label` /
  `--state` filters.
- **Comment on an issue**: `gh issue comment <number> --body "..."`.
- **Apply / remove labels**: `gh issue edit <number> --add-label "..."` /
  `--remove-label "..."`.
- **Close**: `gh issue close <number> --comment "..."`.

`gh` infers the repository from `git remote -v` when run inside the clone.

## Triage flow

1. New issues from the forms start with `needs-triage`.
2. A maintainer either asks for more information (`question`), marks it ready
   (`ready-for-human`), or closes it (`wontfix`, `invalid`, `duplicate`).
3. `good first issue` and `help wanted` are applied only to scoped work that a
   contributor can complete from repository documentation.
4. See [`triage-labels.md`](triage-labels.md) for the exact label strings this
   repository currently defines.

## When a skill says "publish to the issue tracker"

Create a GitHub issue. When a skill says "fetch the relevant ticket", run
`gh issue view <number> --comments`.
