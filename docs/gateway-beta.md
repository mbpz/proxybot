# Gateway (Beta)

Gateway routes a test device's TCP traffic through macOS TUN + sing-box into
ProxyBot. It is **Beta**, not the Core supported mode. Explicit HTTP proxy
remains the default and safest workflow.

## Prerequisites

- macOS with root access for TUN/pf.
- ProxyBot running on `127.0.0.1:8088` (start it as the normal user first).
- A user-supplied **sing-box v1.14.4-compatible** binary built with
  `-tags with_gvisor`; ProxyBot does not bundle it.
- Test device on the same LAN, CA installed/trusted for HTTPS.

## Start

```bash
# terminal 1 (normal user)
proxybot start

# terminal 2 (root)
sudo proxybot gateway start
# or with QUIC fallback policy:
sudo proxybot gateway start --block-quic
# or with per-device identity routing:
sudo proxybot gateway start --devices minimal/examples/devices.json
```

On the device set the Wi-Fi **gateway** to the Mac's LAN IP. HTTP proxy OFF,
DNS can stay on the router.

`gateway start` waits for a real `running` state, writes
`/tmp/proxybot-gateway/state.json`, and logs under
`/tmp/proxybot-gateway/instances/<instance>/`.

## Status and stop

```bash
proxybot gateway status
sudo proxybot gateway stop
```

`stop` signals the supervisor and restores pf/forwarding. Repeated stop is
idempotent. If the supervisor was killed, stop recovers stale state and only
kills an orphan sing-box whose command still references this instance's config.

## Observability

- HTTPS requests: MITM, `client_ip` only when trusted metadata exists.
- HTTP/80: MITM, identity unknown.
- UDP/QUIC/DNS: direct; by default not decrypted or represented as HTTP.
- `--block-quic`: reject UDP/443 at the sing-box route layer to test TCP
  fallback; this is a test policy, not a promise that every app falls back.
- `confidence=inferred`: static per-device routing is association evidence, not
  a per-connection authenticated device identity.
- `device_id` is NULL until a stable device registry exists.
- `connection_id` is proxy-side diagnostic only.

## Recovery and limits

- Supervisor records exit code/signal/reason and restores network state.
- Startup mutex and log rotation are implemented.
- launchd, sleep/wake, Wi-Fi switch and multi-device-at-scale are not yet
  supported gates. See `docs/release-checklist.md`.

## Legacy PF warning

The old `proxybot start --gateway` / `proxybot gateway install|uninstall`
pf `rdr` path is **legacy and unsupported**. It is not the current gateway; do
not use it for new testing. It remains only for cleanup/diagnostics.

## Licensing

ProxyBot is MIT; sing-box is GPL-3.0 and is not bundled in the beta. See
`docs/legal/sing-box.md`.
