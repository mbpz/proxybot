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

No stale state, TUN and child survived; phone regained internet after wake.

### Wi-Fi off/on while running: PASS (detection)

After turning Wi-Fi off:

```text
gateway: stale
health tun=up child=alive proxy=ok
network unknown (Wi-Fi down or default route missing)
stale state: default network/CIDR changed or unavailable
```

After reconnecting to the same network:

```text
gateway: running
network en1 192.168.3.0/24
TUN up, forwarding 1, Mac https 200
```

This verifies detection, not automatic reconfiguration. Phone traffic after
reconnect is still to be explicitly recorded.

### Manual stop, then sleep/wake: PASS

```text
gateway: stopped
exit reason supervisor_stop_requested
recovery success
```

Sleep/wake did not restart the gateway.

### Cross-network / manual-stop matrix

Completed lifecycle runner run (`/tmp/proxybot-lifecycle-20261026-234650`):

```text
PASS=13 FAIL=0 SKIP=0
```

Covered: start, sleep/wake, Wi-Fi off/on, different Wi-Fi/CIDR stale detection,
manual stop, TUN removal, forwarding restore, Mac internet. Earlier direct tests
also covered manual-stop sleep/wake and kill -9 recovery; recovery-failure
injection was verified in P3.

Known boundary: the runner aborted at the second sleep prompt in one run, and
phone browsing after each network event remains a manual check. These are
recorded limitations, not silent passes.

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

## One-command test runner

```bash
sudo sh minimal/gateway-lifecycle-test.sh
```

It captures status/TUN/forwarding/Mac/pf at each phase and pauses only for the
operations a script cannot safely perform alone:

1. confirm Mac sleep;
2. switch to a different Wi-Fi/hotspot and back;
3. confirm the second sleep.

It tests start, sleep/wake, Wi-Fi off/on, cross-network stale detection, manual
stop + sleep, and `kill -9` sing-box recovery. Logs are written under
`/tmp/proxybot-lifecycle-<timestamp>/`. The script prints stage expectations and
asks for explicit iPhone Safari confirmations in B/C/D/E; an unconfirmed item is
`NOT_VERIFIED` and fails the run. It always attempts to stop the gateway and
power Wi-Fi back on at exit.

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

## launchd service (implementation pending tests)

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

The service command/plist/install/uninstall implementation is present; see
`docs/status/2026-10-launchd.md`. launchd remains unverified until the test matrix
passes; the release checklist still lists it as blocking.
