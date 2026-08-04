# Product comparison and lessons

ProxyBot is not trying to win a feature-count comparison. Its opportunity is a
focused macOS workflow for debugging mobile application traffic from a test
device.

## Positioning

| Project | Primary strength | What ProxyBot should learn | What ProxyBot should not copy |
| --- | --- | --- | --- |
| [mitmproxy](https://github.com/mitmproxy/mitmproxy) | Mature programmable proxy with console, CLI, and web interfaces | Clear interface roles, explicit setup/verification, progressive capture modes | Presenting every advanced mode during first use |
| [HTTP Toolkit](https://github.com/httptoolkit/httptoolkit) | Guided interception of a selected client | One-click setup mindset and traffic-noise reduction | Broad platform support before macOS is reliable |
| [Proxelar](https://github.com/emanuele-em/proxelar) | Scriptable local traffic workbench with a concise quick start | Certificate install page, smoke-test request, honest limitations, reproducible packaging | Multiple equal-weight interfaces before the desktop journey is complete |
| [whistle](https://github.com/avwo/whistle) | Rule and plugin ecosystem | Deep rule semantics and extension points | Exposing extension complexity as the main product |
| [Anything Analyzer](https://github.com/DeepLifeStudio/anything-analyzer) | Unified capture and AI-assisted analysis Session | A coherent Session boundary | An AI-first, all-source capture scope |

## ProxyBot's focused advantage

ProxyBot can combine four capabilities around one mobile debugging journey:

1. A macOS desktop application with a reusable Rust MITM Runtime.
2. Device-aware Captured Requests with DNS-supported Application Attribution.
3. Routing Rules, breakpoints, replay, Composer, and export in one workflow.
4. Optional transparent routing and automation Adapters after explicit proxy
   capture succeeds.

This advantage only exists when setup and capture are dependable. A non-working
mode or hidden prerequisite is worse than an absent feature.

## Default mode policy

Comparable tools make their simplest reliable mode easy to find. ProxyBot will
therefore use this order:

1. **Explicit proxy** — default and fully documented.
2. **macOS `pf` + DNS** — Advanced, because it changes host networking and may
   require elevated privileges.
3. **MCP, dashboard, and scripting** — Advanced automation Adapters.
4. **TUN, iOS VPN, SSL bypass, AI, generation, and deployment** — Labs until
   their end-to-end path and support boundary are proven.

## Evaluation criteria

Future comparisons should use durable user outcomes instead of star counts or a
large checkbox matrix:

| Outcome | Evidence required |
| --- | --- |
| Install | Signed/notarized artifact, checksum, clean-Mac smoke test |
| First capture | Timed setup journey ending in a known decrypted request |
| Diagnosis | Device/host/method/status filtering and stable request detail |
| Modification | Routing Rule or breakpoint verified against a local fixture |
| Reproduction | Replay result and export verified against the original request |
| Cleanup | Capture stop and network restoration verified after failure and quit |
| Extensibility | Stable Interface with contract tests, not only an exposed screen |

See the [product roadmap](roadmap.md) for the resulting execution order.
