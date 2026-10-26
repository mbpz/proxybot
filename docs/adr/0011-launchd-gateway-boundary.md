# ADR-0011: launchd boundary for Gateway

- **Status:** Accepted (service lifecycle tested; sleep/Wi-Fi under launchd still gated)
- **Date:** 2026-10-26
- **Phase:** lifecycle implementation design
- **Scope:** launchd ownership, privileges, restart policy, wake behaviour
- **Out of scope:** implementing automatic network reconfiguration

## Context

Gateway Beta now has a Supervisor that owns sing-box, applies TUN/pf network
state, records exit diagnostics, restores networking, and reports
`stale`/`recovery_failed`. Sleep/wake, Wi-Fi loss, different-Wi-Fi stale
detection and manual stop were verified.

The remaining release gate is system lifecycle: launchd, sleep/wake recovery,
Wi-Fi switch handling and uninstall. launchd must not become a second owner of
network state or silently restart a gateway the user stopped.

## Decision

### 1. Single owner

- The **Supervisor** is the only owner of sing-box and network recovery.
- launchd only manages whether the Supervisor process exists.
- The CLI/state file is the control interface; launchd must not maintain a
  parallel `running` state.

### 2. Explicit start only

- No `RunAtLoad`/unconditional `KeepAlive` for Gateway.
- Boot/login must not start Gateway by itself.
- `gateway start` is an explicit root action that loads/starts the service.
- `gateway stop` records user intent `stopped` and must prevent wake/launchd
  from restarting it.
- A crash is reported (`exited`/`recovery_failed`/`stale`); it is not
  automatically restarted in the first implementation.

### 3. Privilege model

- TUN/pf require root. Use a **system LaunchDaemon** for the Supervisor path,
  not a user LaunchAgent, and not the whole UI/app as root.
- ProxyBot UI/HTTP proxy remains a user process.
- Service plist, state, config and logs must have explicit ownership/permissions
  documented before installation.
- `gateway start` must not silently escalate; the user explicitly invokes root.

### 4. Wake / network change

- launchd does not restore network state.
- After wake, the Supervisor/status must check TUN, child, forwarding and current
  interface/IP/CIDR. Mismatch is `stale`, not a false `running`.
- No automatic reconfiguration in the first launchd version; the user runs
  `gateway stop` then `gateway start` after a network change.
- A future auto-reconfigure design needs explicit steps, rollback and tests.

### 5. Uninstall

`gateway uninstall-service` (design name) must:

1. stop the Supervisor and wait;
2. unload/remove the LaunchDaemon plist;
3. flush the Gateway pf anchor and restore forwarding;
4. remove state/TUN leftovers;
5. leave ProxyBot user data/CA intact unless the user explicitly removes it.

## Implementation plan (after design approval)

1. Add a foreground supervisor entry point suitable for launchd.
2. Add plist generation/install/uninstall and `gateway service status`.
3. Make `gateway start/stop` delegate to launchd only when installed, preserving
   current supervisor behavior when not installed.
4. Test install/start/stop/repeat/crash/uninstall, then sleep/wake and Wi-Fi
   switch under the service.
5. Update release checklist only after all tests pass.

## Consequences

- launchd does not hide failures or auto-restart; diagnostics remain visible.
- User must explicitly start/stop; no background service after reboot.
- Network changes require manual recovery in this phase.
- More privileged install/uninstall work remains before a public release.

## Evidence

- `docs/status/2026-10-lifecycle-reliability.md`
- `docs/status/2026-10-26-p3-gateway-lifecycle.md`
- `minimal/src/gateway_run.rs`
