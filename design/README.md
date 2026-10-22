# Design sources

This directory holds the original Pencil (`.pen`) design sources for ProxyBot:

- `proxybot-desktop.pen` — early desktop layout exploration
- `proxybot-complete-design.pen` — complete UI pass
- `proxybot-final.pen` — final pre-implementation design
- `proxybot-full-design.pen` — full design-system exploration

These files are **design references only**. They are not bundled into the
application, not read at build or runtime, and not published with releases.
The shipped UI lives in `src/`; product scope is defined by
[`docs/roadmap.md`](../docs/roadmap.md) and [`docs/adr/`](../docs/adr/README.md).

To edit them you need the Pencil app (`*.pen` is Pencil's project format). If a
design file is no longer useful, delete it here rather than growing the repo.

Note: these are tracked in git because they are small enough to keep versioned
and are useful when revisiting the UI. No other `*.pen` files are tracked or
ignored by pattern any more — keep any new design sources in this directory.
