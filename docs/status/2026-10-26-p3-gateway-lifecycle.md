# P3 gateway lifecycle

## Commands

```bash
# terminal 1: ProxyBot must already be listening on 127.0.0.1:8088
proxybot start

# terminal 2 (root): start sing-box TUN + pf route-to
sudo proxybot gateway start
sudo proxybot gateway start --block-quic
sudo proxybot gateway start --singbox /path/to/sing-box --tun utun6

sudo proxybot gateway status
sudo proxybot gateway stop
```

## What `start` does

1. Checks root and that ProxyBot is listening on 8088.
2. Generates the sing-box config under `/tmp/proxybot-gateway/`.
3. Starts sing-box with stdout/stderr in `/tmp/proxybot-gateway/singbox.log`.
4. Waits for the TUN interface, configures `10.255.0.1 -> 10.255.0.2`.
5. Validates and installs pf `route-to` rules for the LAN.
6. Saves PID/forwarding/interface state in
   `/tmp/proxybot-gateway/state.json`.
7. Spawns a detached watchdog that runs `gateway stop` if sing-box exits.

## What `stop` does

- Sends TERM/KILL to the sing-box PID.
- Flushes the `com.apple/proxybot-tun` anchor and legacy anchors.
- Restores the previously saved `net.inet.ip.forwarding`.
- Removes state, so a repeated stop is a no-op.

## Known limits

- A start failure after pf installation rolls back pf/forwarding, but a
  `kill -9` of the whole process group can still leave rules behind; run
  `sudo proxybot gateway stop`.
- Sleep/Wi-Fi switch handling is not implemented yet.
- ProxyBot itself is not started by `gateway start` (its data/CA belong to the
  user, not root).
- The watchdog is best-effort, not a full service manager.
