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

## Failure injection still to perform

1. normal start/stop;
2. kill sing-box (not supervisor) while running → `exited`, exit code/signal,
   recovery success, Mac/iPhone return to normal;
3. sing-box startup failure → start returns non-zero and no pf remains;
4. repeated stop → idempotent;
5. stale state → status reports stale, does not kill unrelated PIDs;
6. forced recovery failure → `recovery_failed` is visible.

## Known limits

- Supervisor SIGKILL/crash or macOS reboot cannot run recovery; launchd or an
  independent boot-time recovery is a later phase.
- No automatic restart, sleep/wake or Wi-Fi-switch handling yet.
- Log rotation is not implemented.
- PID-reuse protection is first-pass (instance id + live process check).
