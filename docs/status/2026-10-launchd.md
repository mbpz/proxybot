# launchd Gateway service (implementation)

## Commands

```bash
# install (root; does not start the gateway)
sudo proxybot gateway install-service --singbox /path/to/sing-box

# explicit start (root; delegates to launchd)
sudo proxybot gateway start

# stop/cleanup
sudo proxybot gateway stop

# remove service and cleanup
sudo proxybot gateway uninstall-service
```

The plist is `/Library/LaunchDaemons/com.proxybot.gateway.plist`, label
`com.proxybot.gateway`, log `/Library/Logs/ProxyBot/gateway-service.log`.

## Behavior

- `RunAtLoad=false` and `KeepAlive=false`: no boot/login start, no automatic
  restart.
- `gateway start` checks `service_installed()`; when installed it delegates to
  `launchctl kickstart system/com.proxybot.gateway` and waits for the supervisor
  state to become `running`.
- `gateway service` is the foreground entry run by launchd; it owns the same
  Supervisor/state/network code as the direct path.
- `gateway stop` signals the supervisor; launchd sees the process exit and does
  not restart it.
- `gateway uninstall-service` stops the supervisor, bootouts the job, and removes
  the plist.
- The plist pins the executable path and a limited `gateway service` argument
  set; it does not provide an arbitrary root shell helper.
- When the service is installed, `gateway start` ignores direct-path options
  because the installed plist owns them. Reinstall to change sing-box/TUN/device
  arguments.

## Test matrix (pending)

1. Install service while stopped: job loaded, gateway remains `stopped`.
2. Explicit `gateway start`: `running` via launchd; only one supervisor.
3. Repeated `gateway start`: rejected, no second supervisor/network init.
4. `gateway stop`: `stopped`, TUN gone, forwarding 0, Mac online.
5. Wait after stop: no automatic restart.
6. `kill -9 sing-box`: `exited`/signal/recovery; launchd does not restart;
   explicit start works again.
7. Bad sing-box path/install: install fails; no plist/network change.
8. Uninstall while stopped/crashed: service removed; TUN/pf/forwarding clean.
9. Direct path is not used when service is installed.

## Known boundaries

- No automatic restart or automatic network reconfiguration.
- Sleep/Wi-Fi follow the tested Supervisor health/stale behavior; launchd does
  not repair network state.
- Plist args are fixed at install time.
