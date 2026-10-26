# P3 gateway lifecycle: supervisor

## Architecture

`gateway start` launches a detached **supervisor** process. The supervisor owns
sing-box, applies pf route-to, writes heartbeats, records the child exit status,
and restores network state when the child exits or stop is requested.

```text
proxybot gateway start
  └── supervisor (root, detached)
        ├── sing-box (TUN)
        ├── pf route-to (com.apple/proxybot-tun)
        └── state/logs under /tmp/proxybot-gateway/
```

`gateway start` waits for a real `running` state before reporting success. If
health checks fail, it reports the supervisor/sing-box logs and returns non-zero.

## State fields

`state.json` contains: `instance_id`, `supervisor_pid`, `singbox_pid`,
`started_at`, `last_heartbeat_at`, `state`, `exit_code`, `exit_signal`,
`exit_reason`, `recovery_status`, interface/CIDR/TUN, previous forwarding and
config/log paths.

States: `starting`, `running`, `stopping`, `exited`, `stopped`, `recovering`,
`recovery_failed`, and stale detection in `gateway status`.

Exit reasons: `normal_exit`, `signal_terminated`, `startup_failed`,
`healthcheck_failed`, `unexpected_child_exit`, `supervisor_stop_requested`.

## Logs

Per instance:

```text
/tmp/proxybot-gateway/instances/<instance_id>/
  supervisor.log
  sing-box.log
  recovery.log
  sing-box.json
  pf.conf
```

`/tmp/proxybot-gateway/state.json` is atomically replaced. No automatic restart
is performed: an unexpected child exit records the reason, restores networking,
and leaves the state visible for diagnosis.

## Commands

```bash
# terminal 1
proxybot start

# terminal 2
sudo proxybot gateway start
proxybot gateway status
sudo proxybot gateway stop
```

## P3 hardening

### Startup mutex

- `gateway start` takes `/tmp/proxybot-gateway/start.lock` with `flock(LOCK_NB)`
  for the whole startup window.
- The detached supervisor takes `/tmp/proxybot-gateway/lock` for its lifetime.
- A second start fails before touching TUN/pf; the lock is released
  automatically if the owner exits (including SIGKILL).

### Log rotation

- Per-instance `supervisor.log`, `sing-box.log`, `recovery.log`.
- 2 MiB cap; rotated as `.log.1` / `.log.2`; recent exit/recovery lines are kept.
- sing-box stdout/stderr are read through pipes so rotation happens while the
  child is running.

### Recovery failure injection

Set `PROXYBOT_RECOVERY_FAIL=1` to append a controlled failure after the real
restore commands:

```bash
sudo PROXYBOT_RECOVERY_FAIL=1 proxybot gateway start
sudo PROXYBOT_RECOVERY_FAIL=1 proxybot gateway stop
proxybot gateway status             # recovery_failed
sudo env -u PROXYBOT_RECOVERY_FAIL proxybot gateway stop
proxybot gateway status             # stopped
```

The real network restore still runs; the injected failure tests state/log/status
handling without breaking Mac networking. Retry without the variable must clear
the failed state.

## Failure injection evidence

1. normal start/status → `running`, supervisor and child PIDs recorded.
2. `sudo kill -TERM <sing-box-pid>` while running:
   - supervisor recorded `exited`, exit code 0 (sing-box handles TERM
     gracefully), recovery `success`;
   - `utun6` disappeared, `net.inet.ip.forwarding` restored to 0, Mac HTTPS 200.
   - Note: because sing-box exits 0 on graceful TERM, the supervisor cannot
     distinguish this from a self-exit; it reports `unexpected_child_exit`
     rather than `signal_terminated`.
3. `--singbox /nonexistent`: start returns non-zero before spawning a supervisor;
   no new pf/TUN remains.
4. Normal stop: `stopped`, recovery success; second stop is idempotent.
5. `kill -9 <sing-box>`: `exit_signal=9`, `exit_reason=signal_terminated`,
   recovery success, `utun6` gone, forwarding restored, Mac 200.
6. Repeated stop is idempotent (`gateway is already stopped`).
7. Stale exited state transitions to `stopped` after recovery.
8. Supervisor `kill -9`: `status` reports `stale`; `stop` kills only the
   orphan child whose command contains this instance's config path, restores
   pf/forwarding, and ends in `stopped`. Verified: no sing-box process, no
   `utun6`, forwarding 0, Mac HTTPS 200.
9. Forced recovery failure: `PROXYBOT_RECOVERY_FAIL=1` produced
   `gateway: recovery_failed`; retry without the variable produced `stopped`,
   `recovery success`. The real network was restored even during injection.
10. Startup mutex: two concurrent starts produced one `gateway started` and one
    `Error: another gateway start is already in progress`.
11. Log rotation unit test passes at the 2 MiB boundary.

Phase 1 acceptance: normal lifecycle, exit diagnostics, unexpected TERM/kill -9,
startup failure, stale supervisor, idempotent stop, startup mutex, log rotation
and recovery-failure injection all pass. Automatic restart, launchd and
sleep/Wi-Fi handling remain future work.

## Known limits

- Supervisor SIGKILL/crash or macOS reboot cannot run recovery; launchd or an
  independent boot-time recovery is a later phase.
- No automatic restart, sleep/wake or Wi-Fi-switch handling yet.
- Log rotation is not implemented.
- PID-reuse protection is first-pass (instance id + live process check).
