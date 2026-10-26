# sing-box dependency and GPL-3.0 decision

## Current facts

| Item | Value |
| --- | --- |
| Component | sing-box |
| Version used for validation | v1.14.4 |
| Revision | `c005f35300ce76427dc285481ec5695f5b179cd2` |
| License | GPL-3.0 |
| Build | Go 1.27.2, `-tags with_gvisor` (from upstream source) |
| Modifications | None made by ProxyBot |
| Integration | separate process, JSON config, loopback HTTP CONNECT |
| Distribution in ProxyBot beta | **not bundled** |

ProxyBot itself remains MIT-licensed. The current beta does not ship a sing-box
binary in its release archive or install script. The operator installs sing-box
separately (for example from a package manager or an upstream release) and passes
it via `PROXYBOT_SINGBOX_BIN` or `--singbox`.

## Decision (beta)

Use **Option B: user-supplied dependency** for the beta.

- `proxybot gateway start` discovers `sing-box` on PATH or a known path;
  otherwise it fails with an actionable error.
- The user is responsible for obtaining sing-box and for compliance with the
  GPL for their copy.
- ProxyBot must not claim that the dependency is licensed under ProxyBot's MIT
  license.
- Release archives and package scripts must not include sing-box, its source, or
  its binaries.

This is an engineering distribution decision, not a legal opinion. The fact that
the programs communicate as separate processes is relevant to the analysis but
does not by itself establish every obligation; do not treat it as a blanket
exemption.

## If ProxyBot later bundles sing-box (Option A)

Before shipping a bundle:

1. Record the exact version, source URL, source revision, build tags and any
   modifications.
2. Include the GPL-3.0 license text and copyright notices.
3. Provide the corresponding source (or a valid written offer) for the exact
   binary, including ProxyBot's build scripts/patches where required.
4. Preserve installation information for any covered user product if applicable.
5. Have the distribution reviewed by someone qualified in open-source licensing
   before a commercial release.

## Verification

```bash
proxybot gateway start --singbox /path/to/sing-box
sing-box version
```

If the installed binary was built without `with_gvisor`, the gateway fails at
startup with `gVisor is not included in this build`; rebuild with
`-tags with_gvisor` or use an upstream build that includes it.
