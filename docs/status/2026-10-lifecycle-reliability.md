# Lifecycle reliability (sleep / Wi-Fi / launchd)

## Current Supervisor state machine

```text
starting -> running -> stopping -> stopped
              |  \
              |   +-> recovering -> exited
              |                 \-> recovery_failed
              +-> stale (diagnostic: supervisor/child/TUN/heartbeat/network mismatch)
```

- The supervisor owns sing-box and network restore.
- A normal stop records `supervisor_stop_requested` and restores pf/forwarding.
- An unexpected child exit records exit code/signal and recovers.
- If the supervisor is SIGKILLed, `status` reports `stale`; `stop` kills only an
  orphan whose command contains this instance's config path, then restores.
- There is **no automatic restart** and no sleep/Wi-Fi event handling yet.

`proxybot gateway status` now reports:

```text
health tun=up/absent child=alive/missing proxy=ok/down
network <iface> <cidr> (state expected <iface> <cidr>)
```

and marks `stale` if the supervisor, child, TUN, heartbeat or default network
does not match the recorded state. It diagnoses only; it does not silently
reconfigure.

## Test results (2026-10-26)

### Sleep/wake while running: PASS (baseline)

After `pmset sleepnow` and wake:

```text
gateway: running
health tun=up (10.255.0.1) child=alive proxy=ok
network en1 192.168.3.0/24 (state expected en1 192.168.3.0/24)
forwarding: 1
Mac https: 200
```

No stale state, TUN and child survived. Phone-after-wake and Wi-Fi change tests
remain.

## Required tests (first batch)

Run in a recoverable test environment; do not test during important work.

| # | Scenario | Expected |
| --- | --- | --- |
| 1 | Start gateway, sleep Mac, wake | `status` reflects actual TUN/child; no false healthy |
| 2 | Sleep with Wi-Fi disconnected | status does not claim healthy; TUN/pf state known |
| 3 | Wake with Wi-Fi reconnect | status shows current iface/CIDR or stale mismatch |
| 4 | Switch Wi-Fi network / IP change | status reports interface/CIDR change; no wrong-device routing |
| 5 | Wake after sing-box exited | `exited`/`stale`, recovery result visible |
| 6 | Wake with TUN missing | stale/absent TUN; no blind reuse of old state |
| 7 | Recovery failure injected | `recovery_failed`, retry works |
| 8 | Manual stop, then sleep/wake | remains stopped; no auto-start |
| 9 | Kill supervisor, then sleep/wake | stale status; stop cleans orphan |
| 10 | P3 stop/restore regression | no TUN, forwarding restored, Mac/phone online |

Record actual results here before launchd implementation. Do not infer Wi-Fi
safety from one sleep test.

## Test commands

```bash
# before sleep
sudo proxybot gateway start
proxybot gateway status
ifconfig utun6
sysctl net.inet.ip.forwarding
curl -sS -o /dev/null -w '%{http_code}\n' https://example.com

# sleep (user confirms no critical work is running)
pmset sleepnow

# after wake
proxybot gateway status
ifconfig utun6
sysctl net.inet.ip.forwarding
curl -sS -o /dev/null -w '%{http_code}\n' https://example.com
```

Wi-Fi tests: turn Wi-Fi off/on, switch network, or renew DHCP, then run the same
status/TUN/forwarding/Mac checks. For each test record whether the gateway was
still `running`, `stale`, `exited` or `stopped`, and whether network recovery
needed manual `gateway stop`.

## launchd boundary (design, not implemented)

Decisions required before code:

1. **Single owner:** Supervisor owns sing-box and network recovery; launchd only
   manages whether the supervisor process exists. CLI/status remains the control
   interface. No parallel state.
2. **No unconditional restart:** launchd must not restart a gateway after a
   user-requested stop or a failed recovery. Any keep-alive policy needs an
   explicit state/intent marker and retry limits.
3. **Sleep/wake:** launchd is not a network recovery mechanism. Wake must check
   TUN/pf/interface state and report stale/recovery failure; do not assume restart
   fixes residual routing.
4. **Privilege:** TUN/pf need root. Decide system LaunchDaemon vs privileged
   helper; do not run the whole UI/app as root. Files/logs/state ownership and
   uninstall cleanup must be defined.
5. **Uninstall:** remove service definition, stop supervisor, restore network
   state, and verify no TUN/pf/process remains.

The current beta has no launchd integration; the release checklist still lists
launchd, sleep/wake, Wi-Fi changes and clean-environment uninstall as blocking
gates.
