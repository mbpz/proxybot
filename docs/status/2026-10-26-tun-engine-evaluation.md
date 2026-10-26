# TUN forwarding engine evaluation (2026-10-26)

## Goal

Pick a mature user-space forwarding engine for the gateway path instead of
writing a TCP stack / NAT relay. The first milestone is only a TCP/UDP closed
loop from the iPhone through TUN to the internet; proxy/MITM integration comes
after that.

## Candidates

### tun2socks (xjasonlyu)

- **License: MIT** (verified in the repository `LICENSE`; earlier notes said
  GPL, which is not the current license).
- Go + gVisor user-space network stack; macOS build verified locally
  (`darwin/arm64`, 16 MB binary).
- Proxy outlets: `direct://`, `http://`, `socks4://`, `socks5://`,
  `shadowsocks://`, `ssh://`, `relay://`, `reject://`; supports gateway mode.
  Always include `://`: without it tun2socks defaults to `socks5://` and a bare
  `direct` becomes `socks5://direct` (UDP then fails with "missing port").
- Device drivers (from `engine/parse.go`):
  - `tun://utunN` — engine creates/opens a TUN.
  - `fd://N` — attach an existing TUN file descriptor; on Darwin/iOS the
    packet read/write offset is 4 bytes (utun header).
- UDP: `direct` and SOCKS5 (UDP associate) can carry UDP; HTTP proxy is
  TCP/CONNECT only. This must be tested per outlet.
- CLI observed:
  ```text
  -d, --device [driver://]name
  -p, --proxy  [protocol://]host[:port]
      --interface <name>
      --mtu <n>
      --loglevel <level>
  ```

### sing-box

- **License: GPL-3.0** (repository `LICENSE`; no derivative may imply
  association with the project without consent).
- Tun inbound creates/manages its own TUN (`interface_name`, `auto_route`,
  `strict_route`, `stack`); the docs do not expose attaching an existing TUN fd.
- `bridge` outbound can do TCP/UDP/ICMP L3 forwarding on supported platforms,
  but that is direct forwarding, not automatic MITM handoff.
- Valuable as a reference/alternative, but the GPL license and TUN ownership make
  it a heavier product dependency.

## Decision

Use **tun2socks** for the next prototype.

- Start with **engine-owned TUN** (`-device tun://utunN`), because our current
  `tun` crate does not expose the raw fd needed for `fd://` attachment.
- Route iPhone internet traffic into that interface with the already-validated
  `minimal/pf-tun-test.sh` route-to rule.
- Validate in two stages:
  1. `-proxy direct`: TCP/UDP/DNS return path, no proxy/MITM.
  2. `-proxy http://127.0.0.1:8088`: TCP CONNECT through the existing ProxyBot
     HTTP proxy (HTTPS MITM works for TCP); UDP/QUIC is out of scope for the
     HTTP proxy and needs a SOCKS5 UDP outlet later.
- Keep `fd://N` as a follow-up option if engine-owned TUN lifecycle becomes a
  problem; it avoids changing the existing pf route-to target.

## Validation result (2026-10-26)

The `direct://` closed loop passed:

- tun2socks created `utun6`, log showed `[STACK] tun://utun6 <-> direct://`;
- `minimal/pf-tun-test.sh install utun6` configured `10.255.0.1/10.255.0.2`
  and loaded the route-to rule;
- the iPhone got internet again and the engine logged both **TCP** and **UDP**
  sessions (DNS to 223.5.5.5, TCP 80/443, app UDP);
- occasional `connection refused`/`reset` messages came from apps probing
  unavailable endpoints, not from the tunnel;
- a bare `direct` proxy argument is parsed as `socks5://direct` and breaks UDP;
  the scheme-qualified `direct://` is required.

### HTTP proxy outlet result (2026-10-26): not sufficient

Tested with `--proxy http://127.0.0.1:8088` and ProxyBot's HTTP proxy:

- UDP: `dial 223.5.5.5:53: unsupported operation` — the HTTP proxy implementation
  has no UDP support, so DNS fails and the gateway is unusable for normal apps.
- TCP: tun2socks uses HTTP `CONNECT` for every TCP destination, including port
  80. Hudsucker expects absolute-form requests for plain HTTP, so it logged
  `Unknown protocol, read '[50,4F,53,54]' (POST)` / TLS handshake errors.
- Conclusion: an HTTP proxy is **not** a complete gateway outlet. The TUN engine
  needs a SOCKS5 outlet with TCP CONNECT **and UDP ASSOCIATE** (for DNS/QUIC),
  or a different engine/outlet design.

### SOCKS5 outlet validation with sing-box 1.14.4

Built sing-box v1.14.4 from source (GPL-3.0, evaluation only) and ran a
loopback SOCKS inbound:

```json
{
  "inbounds": [{"type":"socks","tag":"socks-in","listen":"127.0.0.1","listen_port":1080}],
  "outbounds": [{"type":"direct","tag":"direct"}]
}
```

Config is kept at `minimal/examples/singbox-socks.json`.

Results on macOS arm64:

- **TCP CONNECT**: `curl --socks5-hostname 127.0.0.1:1080 https://example.com`
  returned 200;
- **UDP ASSOCIATE**: relay allocated; a real DNS query for `example.com` via
  `223.5.5.5:53` returned a response with the matching transaction ID
  (`txid 1234`, ancount 2) — **PASS**;
- helper: `minimal/tools/socks5_udp_test.py`.

This version/config does not show the reported 1.14.x UDP-relay failure; still,
the full path must be validated through tun2socks + iPhone, not just the local
SOCKS test.

Next: `tun2socks --proxy socks5://127.0.0.1:1080` + pf route-to, then verify
TCP, DNS, UDP/QUIC and Mac isolation on the phone.

## Test procedure

```bash
# 0. Stop proxybot, restore any old rules
pkill -f "proxybot" 2>/dev/null || true
sudo sh minimal/pf-tun-test.sh restore 2>/dev/null || true

# 1. Engine-owned TUN, direct outlet (no proxy): TCP/UDP closed loop
#    (local build from source: /tmp/tun2socks/tun2socks)
sudo /tmp/tun2socks/tun2socks \
  -device tun://utun6 -proxy direct:// -interface en1 --loglevel debug

# 2. Second terminal: route forwarded phone traffic into that utun
sudo sh minimal/pf-tun-test.sh install utun6

# 3. iPhone: manual IP, router = Mac, DNS = 223.5.5.5, proxy off, cellular off
#    Open a page; with the direct outlet it should have internet again and
#    tun2socks should log TCP/UDP flows.

# 4. Restore
sudo sh minimal/pf-tun-test.sh restore
# iPhone back to automatic IP
```

Expected results:

| Protocol | First check |
| --- | --- |
| TCP | page/API works; tun2socks logs connection; response returns |
| UDP | DNS works; direct outlet UDP sessions; QUIC may work via direct |
| ICMP | engine may not forward; record actual behavior |
| IPv6 | not enabled in this probe; record as unsupported/blocked |

Then repeat with `-proxy http://127.0.0.1:8088` (scheme included) while `proxybot start` is running
to verify TCP through the existing MITM proxy. Do not assume UDP follows HTTP
CONNECT; test it separately (SOCKS5 UDP associate is the likely requirement).
