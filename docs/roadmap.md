# Roadmap and status

Current CLI release: **2.0.0**. Explicit HTTP proxy is the supported Core mode
on macOS. Windows CI packaging passed; physical-device acceptance remains a
support gate. Gateway is macOS Beta.

## Verified

- HTTP/HTTPS capture through an explicit proxy with local CA.
- WebSocket frames, SQLite history, HAR export, rules.
- Gateway TUN/sing-box/Hudsucker path with HTTP/80 adapter and QUIC policy.
- Supervisor start/stop/status, launchd service, stale/orphan recovery.
- Two-device source-IP routing without identity crossover.
- Source/unknown filtering UI and offset pagination.
- Sleep/wake, Wi-Fi loss/reconnect, cross-network stale detection and cleanup.

## Gated / not verified

- Automatic network reconfiguration on Wi-Fi/IP change (currently reports stale
  and requires explicit restart).
- Exact TUN ingress-to-request connection correlation (`connection_id` is
  proxy-side only).
- UDP/QUIC/DNS decryption and full DNS policy.
- More than two device tests; devices behind one source IP/NAT.
- Certificate-pinned apps.
- Clean external Mac, GitHub download path, and 0.1.13 → 2.0.0 upgrade
  acceptance for the current release.
- Signed/notarized distribution and commercial licensing review.
- Windows physical-device HTTPS acceptance before a Windows support claim.

## Next

1. Windows physical-device acceptance for explicit proxy capture.
2. External Mac release-candidate acceptance on the GitHub 2.0.0 assets.
3. Upgrade validation from 0.1.13.
4. Expand lifecycle fault injection and release packaging checks.
5. Decide whether a future ingress adapter is needed for exact connection
   identity.

See [status](status.md) and [release checklist](release-checklist.md).
