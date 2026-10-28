# Third-party components

ProxyBot's own source is MIT-licensed. The Gateway Beta does **not** bundle
sing-box or any other third-party binary.

## sing-box (not distributed)

- License: GPL-3.0
- Tested version: v1.14.4
- Build: `-tags with_gvisor`, unmodified upstream source
- The operator must install it separately and is responsible for its GPL
  compliance.
- If ProxyBot later ships a sing-box binary, follow the checklist in
  `docs/legal/sing-box.md` before release.

## Other dependencies

Rust crates are pinned in `minimal/Cargo.lock`. Their licenses are available from
the crates in that lockfile; no third-party native binary is included in the
release archive.
