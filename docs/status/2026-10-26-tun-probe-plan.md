# TUN forwarding probe plan (2026-10-26)

## Why

Stage 1 proved macOS can forward/NAT iPhone traffic on the same Wi-Fi
interface. Stage 2 showed pf `rdr` never delivers redirected connections to the
local listener. Before integrating a user-space engine or MITM, prove the next
uncertain link: **can forwarded packets be steered into a TUN interface and read
back with correct return path?**

This probe is deliberately smaller than a transparent proxy:

- no MITM, no certificate work, no proxy integration;
- no full user-space TCP stack;
- the only success criterion is packet ingress/egress at the TUN boundary.

## Proposed probe

1. Create a `utun` interface (root), assign a private point-to-point address
   (e.g. `10.255.0.1/30`), MTU ~1500; no default route.
2. Add a pf rule for the phone subnet that sends forwarded traffic to the TUN
   interface (`route-to (utunN)` / equivalent) while leaving Mac's own traffic
   alone.
3. Read IP packets from TUN and print: direction, src/dst, protocol, ports,
   packet count. Do not forward them yet.
4. Verify return path in a second step: inject replies back toward the iPhone
   with correct source translation; until then the phone has no internet.

## Acceptance

- iPhone packets (IP/TCP/UDP) appear on the TUN interface in both directions of
  observation (at least ingress), with correct destination addresses.
- Mac's own traffic is not accidentally routed into TUN.
- Restore removes the interface/route/pf rule and the iPhone regains internet.

## Failure handling

- If packets cannot be steered into TUN, stop: neither Network Extension nor a
  user-space engine can fix the gateway path by itself.
- If ingress works but return fails, isolate routing/NAT before adding any proxy
  or TCP stack.
- Keep the HTTP proxy mode untouched; this is a separate experiment.

## Implementation status (2026-10-26)

- `proxybot tun-probe` creates a utun, prints forwarded packets (IP/TCP/UDP),
  and exits after `--duration`; no return path.
- First live run: TUN ingress worked (63 packets, including iPhone DNS/ICMP),
  which validates the packet path. Bug found: the initial route-to rule matched
  every inbound packet from the LAN, including router DNS/DHCP/ICMP addressed to
  the Mac, so the Mac also lost internet. The rule now excludes LAN destinations
  (`to ! <lan_cidr>`) and only forwards internet-bound phone traffic; a second
  live run then captured 165 packets, all from the iPhone (DNS/ICMP/TCP/UDP to
  public destinations), while the Mac kept its internet access.
- `minimal/pf-tun-test.sh install utunN` configures the utun address
  (`10.255.0.1`/peer `10.255.0.2`) and loads the route-to rule; `restore`
  removes it and restores forwarding. Engine-owned TUNs (tun2socks) start with
  no IPv4 address, so route-to would otherwise point at an unreachable gateway.
- Both are local tools only; not integrated into `proxybot start --gateway`.

## Non-goals

- No full TCP stack implementation from scratch; evaluate mature user-space
  forwarding engines only after ingress/return are proven.
- No Network Extension work until this probe (and a separate NE capability probe)
  says the traffic can actually reach the interception point.
