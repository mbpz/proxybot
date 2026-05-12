# HTTP/3 and QUIC Support for ProxyBot: Technical Assessment

**Document Type:** Solution Design Document (SDD)
**Status:** Research Only (No Implementation)
**Date:** 2026-05-10

---

## Executive Summary

**Feasibility: Conditionally Feasible as Observer, Not True MITM Interceptor**

QUIC/HTTP/3 support for ProxyBot faces a fundamental architectural barrier. Unlike TCP-based TLS (where ProxyBot acts as a legitimate Certificate Authority and re-encrypts with a forged certificate), QUIC's encryption is end-to-end at the transport layer (RFC 9000). The initial handshake (1-RTT, 0-RTT) cannot be intercepted and decrypted without breaking QUIC's core security property.

**Verdict:** ProxyBot can function as a **QUIC pass-through observer** (logging SNI from the QUIC header, connection timing, byte counts), but cannot perform true HTTPS-style MITM decryption on QUIC connections. Any "MITM" on QUIC requires terminating QUIC at the proxy and re-encrypting to the destination — a fundamentally different architecture than the current TCP/TLS interceptor.

**Key Caveats:**
- SNI is visible in QUIC's initial packet header (before encryption is complete)
- Connection migration (RFC 9000 s.9) makes tracking flows harder
- macOS `pf` redirects TCP, not UDP — UDP redirection for QUIC requires additional work
- Both major Rust QUIC stacks (quinn/h3, quiche) are designed for clients/servers, not proxies

---

## 1. Technical Analysis

### 1.1 QUIC Protocol Fundamentals (RFC 9000, RFC 9114)

QUIC is a UDP-based multiplexed multi-stream transport protocol that encapsulates HTTP/3 (RFC 9114). Key properties relevant to MITM interception:

**Connection Establishment (RFC 9000 s.7):**
- 1-RTT handshake: Client sends CRYPTO frames with TLS ClientHello in the first flight
- 0-RTT: Early data with resumption tickets (vulnerable to replay attacks)
- Server certificates are encrypted in CRYPTO frames — not visible to intermediaries
- Connection IDs (CID) are used for connection migration (RFC 9000 s.9)

**Encryption Levels (RFC 9000 s.4.4):**
| Level | Data Protected | Visible to Intermediary |
|-------|---------------|------------------------|
| 0-RTT | Early data | Encrypted (AES-GCM/AES-CCM) |
| 1-RTT | All stream data | Encrypted |
| Handshake | ClientHello, server certificates | Partially visible (Initial packet is " unprotected" but contains only the CRYPTO frame pointer — actual certs are in encrypted CRYPTO frames) |

**Initial Packet Protection (RFC 9000 s.7.2):**
The Initial packet uses a salt-derived key from the destination Connection ID. The contents are XORed with a key derived from the packet number space. While the Initial packet is technically "unprotected" at the header level, the CRYPTO frames inside carry the TLS handshake which is itself encrypted. An intermediary cannot extract server certificates from Initial packets without the salts and shared secrets from the handshake.

**SNI Visibility (RFC 9000 s.8.2):**
The Server Name Indication (SNI) is transmitted in the TLS ClientHello inside CRYPTO frames. In QUIC v1 (RFC 9000), the Initial packet header contains the destination CID, version, and packet number — but the SNI is in the encrypted payload. **SNI is NOT visible in the QUIC packet header** — contrary to some earlier QUIC drafts.

### 1.2 quinn + h3 Crate Assessment

**Crate Versions (as of 2026):**
- `quinn`: v0.11+ (crates.io)
- `h3`: v0.1+ (separate crate for HTTP/3 over quinn)

**API Design:**
quinn is designed for client and server use cases. The `Endpoint` type represents a QUIC connection endpoint. For a proxy:
- `quinn::Endpoint` accepts incoming connections (server-side)
- `quinn::Connection` represents an established QUIC connection
- `quinn::IncomingStreams` provides bidirectional stream access

```rust
// quinn server-side pseudocode
let endpoint = quinn::Endpoint::server(config, local_addr)?;
while let Some(conn) = endpoint.accept().await {
    tokio::spawn(handle_connection(conn));
}
```

**MITM Incompatibility:**
quinn's security model assumes endpoints are either clients or servers with legitimate credentials. There is no "MITM mode" because:
1. The TLS handshake is managed internally — no access to the raw handshake to forge certificates
2. crypto::Session from rustls is embedded — no hook for certificate interception
3. The connection crypto state is managed by quinn, not exposed to external code

**Connection Handling:**
quinn provides `BiDirStreams` and `RecvStream`/`SendStream` for application data. For HTTP/3, `h3` provides the HTTP/3 request/response framing on top of quinn streams.

**Performance:**
- Zero-copy UDP socket I/O via `tokio::net::UdpSocket`
- Connection migration support (RFC 9000 s.9) — reduces reconnect overhead
- Head-of-line blocking reduction (RFC 9000 s.6.9) — streams independent of each other
- 0-RTT resumption — faster subsequent connections

### 1.3 quiche Crate Assessment

**Crate Version:**
- `quiche`: v0.20+ (crates.io) — actively maintained by Cloudflare

**API Design:**
quiche is lower-level than quinn, closer to the QUIC RFC wire format:
- `quiche::Config` for connection configuration
- `quiche::Connection` for individual connections
- `quiche::Header` for parsing QUIC packet headers
- `quiche::send() / quiche::recv()` style API for processing packets

```rust
// quiche server-side pseudocode
let config = quiche::Config::new()?;
let (header, payload) = quiche::parse_header(packet)?;
let conn = quiche::accept(&header, &config)?;
let written = conn.send(&mut out)?;
```

**MITM Incompatibility:**
Same fundamental issue as quinn — QUIC's handshake encryption prevents intermediaries from accessing the certificate chain. quiche's API gives more low-level control but does not provide a MITM interception mode.

**Production Use:**
Cloudflare uses quiche in production for their edge QUIC/HTTP/3 implementation at scale. This is production-hardened code.

**Advantage for ProxyScenarios:**
quiche's low-level API makes it easier to build a pass-through observer (buffering packets, logging metadata) compared to quinn's higher-level abstraction. However, this advantage is marginal.

### 1.4 macOS Network Extension NEPacketTunnelProvider

**TCP vs UDP Redirection:**
The existing ProxyBot architecture uses `pf` (packet filter) to redirect TCP ports 80/443 to the local proxy. The pf rule:
```
pass in proto tcp from any to any port 80,443 divert-to 127.0.0.1 <proxy_port>
```

This works for TCP only. QUIC runs over UDP, so pf rules that redirect TCP will not capture QUIC traffic. For QUIC interception, you would need:

1. **UDP redirection via pf**: Additional rules to redirect UDP 443 traffic
2. **NEPacketTunnelProvider** (for VPN mode): This can capture both TCP and UDP packets at the IP layer
3. **Raw UDP socket**: On macOS, a raw UDP socket can bind to port 443 to receive QUIC traffic (if no other process has claimed it)

**NEPacketTunnelProvider for QUIC:**
NEPacketTunnelProvider operates at the IP packet layer, meaning it receives raw IP packets (both TCP and UDP). This makes it more suitable for QUIC interception than pf, but:
- Requires Apple Developer Program membership + Network Extension entitlement approval
- Requires separate Swift app/package for the packet tunnel
- Memory/CPU constraints for Network Extensions on macOS

---

## 2. QUIC MITM Feasibility Assessment

### 2.1 The Core Problem

True MITM on QUIC is fundamentally not possible without terminating the QUIC connection. Here's why:

**TLS over QUIC vs TLS over TCP:**
In a TCP-based MITM (the current ProxyBot approach):
1. Client connects to ProxyBot (ProxyBot presents its CA-signed certificate)
2. ProxyBot extracts the original SNI/host from the decrypted TLS ClientHello
3. ProxyBot establishes a new TLS connection to the destination server
4. Data flows: Client ←→ ProxyBot ←→ Server

In QUIC MITM:
1. Client sends Initial packet containing CRYPTO frame with ClientHello
2. The ClientHello is encrypted with keys derived from the Initial packet's destination CID salt
3. ProxyBot cannot decrypt the ClientHello without breaking QUIC's key derivation
4. Even if ProxyBot could extract the SNI, it cannot respond as the server without having the server's certificate chain

**What is visible to a QUIC intermediary:**

| Observable | Visibility | Useful for MITM? |
|------------|-----------|-----------------|
| Destination IP:port | Header (plaintext) | Yes — routing |
| QUIC version | Header (plaintext) | Yes — version negotiation |
| Connection ID (CID) | Header (plaintext) | Yes — connection tracking |
| SNI | Encrypted in CRYPTO frame | No |
| Server certificate | Encrypted in CRYPTO frame | No |
| HTTP request/response | Encrypted in 1-RTT streams | No |
| Packet timing/size | Network observable | Yes — fingerprinting |
| Connection migration | Via new CIDs | Yes — tracking |

### 2.2 Practical Approaches

#### Approach A: Full QUIC Termination + Re-encryption (Observer-as-Server)

**Description:** ProxyBot terminates the QUIC connection (acting as the server), extracts the HTTP traffic, then establishes a new QUIC (or TCP/HTTP) connection to the destination.

```
Phone → [Initial packet] → ProxyBot
ProxyBot → [new Initial to server] → Server
```

**Challenges:**
- Requires ProxyBot to have a certificate trusted by the phone (or be a CA)
- QUIC handshake tokens and source address tokens are not reusable across ProxyBot's address
- 0-RTT data cannot be proxied (replay risk)
- Requires implementing full QUIC server stack — complex

**Pros:** Full visibility into HTTP traffic
**Cons:** Technically very complex, fragile handshake handling

#### Approach B: Pass-Through Observer

**Description:** ProxyBot forwards QUIC packets without decryption but logs observable metadata (connection timing, packet sizes, destination IP/port, CID tracking).

```
Phone → [QUIC packets] → ProxyBot → [forward] → Server
        └──────── logging only ────────┘
```

**What can be logged:**
- Connection establishment timing (handshake latency)
- Total bytes transferred (upload/download)
- Destination IP and port
- QUIC version
- Connection migration events (CID changes)
- Stream-level timing (when streams open/close)

**What cannot be logged:**
- SNI (encrypted)
- HTTP headers and body
- Server certificate

**Pros:** Simple, works with any QUIC connection, no handshake interference
**Cons:** No HTTP visibility — limited analytical value

#### Approach C: Connection Migration Manipulation

**Description:** Exploit QUIC's connection migration (RFC 9000 s.9) to force connections through ProxyBot. If the phone's QUIC stack migrates connections based on network path changes, ProxyBot could potentially manipulate path changes.

This is speculative and not reliable — connection migration is designed to be transparent to the endpoint, not manipulable by intermediaries.

**Verdict:** Not practical.

---

## 3. Implementation Options (If Feasibility Is Assumed)

Assuming the goal is to add QUIC support as an **observer** (not true MITM):

### Option 1: UDP Pass-Through Observer with quiche

**Rank:** 1st (Recommended)

**Approach:**
- Add a UDP listener on port 443 alongside the existing TCP listener
- Use quiche to parse QUIC packet headers (without full connection termination)
- Log observable metadata (destination, CID, timing, byte counts)
- Forward packets using raw UDP socket (Linux `SO_ATTACH_BPF` / macOS equivalent)

**Phased Implementation:**
```
Phase 1: UDP listener + quiche header parsing + logging
Phase 2: Connection tracking via CID correlation
Phase 3: Integration with existing DNS correlation for app identification
```

**Complexity:** Medium — uses existing proxy architecture, adds UDP path

**Limitations:** No HTTP content visibility

### Option 2: Full QUIC Proxy with quinn + h3

**Rank:** 2nd

**Approach:**
- Implement a QUIC server endpoint using quinn
- Terminate incoming QUIC connections
- Extract HTTP/3 requests via h3
- Re-establish QUIC to destination server (or downgrade to HTTP/1.1/2)

**Challenges:**
- Certificate management becomes complex — ProxyBot CA must sign for all destinations
- 0-RTT handling requires careful replay protection
- QUIC handshake token reuse is not reliable through MITM
- h3 is relatively young compared to HTTP/2

**Complexity:** High

**Advantages:** Full HTTP visibility if implemented correctly

### Option 3: NEPacketTunnelProvider QUIC Capture (VPN Mode)

**Rank:** 3rd (Long-term)

**Approach:**
- Build a Swift NEPacketTunnelProvider that captures IP packets
- Forward captured packets to ProxyBot over a TLS tunnel
- ProxyBot processes both TCP and UDP (QUIC) at the IP layer

**Prerequisites:**
- Apple Developer Program + Network Extension entitlement
- Separate Swift package development
- .mobileconfig provisioning profile

**Complexity:** Very High (requires iOS/macOS Network Extension expertise)

**Advantages:** Captures QUIC without pf redirection, more reliable

---

## 4. Recommended Approach

**Short Term (v1.x):** Pass-Through Observer with quiche

The immediate recommendation is to implement a UDP-based QUIC observer that:
1. Adds a UDP socket listener on port 443 ( alongside TCP )
2. Uses quiche to parse QUIC packet headers
3. Logs connection metadata without decryption
4. Correlates with existing DNS query logs for app identification

This approach:
- Does not require Apple Network Extension entitlements
- Leverages existing pf architecture for TCP, extends to UDP
- Provides observable metadata without claiming full MITM
- Is implementable in the existing Rust codebase

**Long Term (v2.x):** Evaluate NEPacketTunnelProvider for true IP-layer capture

If QUIC traffic becomes significant and the observer approach proves valuable, invest in NEPacketTunnelProvider development for IP-level capture that handles both TCP and QUIC uniformly.

---

## 5. Limitations and Risks

### Technical Limitations

1. **No HTTP Content Visibility**: Observer mode only logs connection metadata, not HTTP headers/body
2. **No SNI Extraction**: SNI is encrypted in QUIC CRYPTO frames
3. **No Certificate Inspection**: Server certificates are encrypted
4. **Connection Migration Tracking**: QUIC CIDs change — tracking across migration is complex
5. **UDP Fragmentation**: QUIC packets can be fragmented — reassembly required
6. **0-RTT Data**: Early data is encrypted and cannot be inspected (also a security risk in proxy scenarios)

### Architectural Risks

1. **pf Limitation**: macOS `pf` redirects TCP, not UDP — UDP redirect rules must be added separately
2. **QUIC Version Diversity**: Multiple QUIC versions (v1 RFC 9000, v29 draft, experimental) — header parsing must be version-aware
3. **HTTP/3 Adoption Rate**: HTTP/3 adoption is growing but still not dominant — ROI may be low in the near term
4. **Apple Network Extension Gatekeeping**: NEPacketTunnelProvider requires Apple approval — uncertain timeline
5. **Performance Overhead**: QUIC software decoding adds CPU overhead, especially for pass-through observation

### Security Considerations

1. **0-RTT Replay Risk**: If ProxyBot attempts QUIC termination, 0-RTT data could be replayed
2. **Certificate Trust Chain**: If terminating QUIC, ProxyBot's CA must be trusted by the phone
3. **Connection Migration Fingerprinting**: Observable QUIC behavior can be fingerprinted even without decryption

---

## 6. References

### RFCs
- **RFC 9000**: QUIC: A UDP-Based Multiplexed and Secure Transport (https://www.rfc-editor.org/rfc/rfc9000)
- **RFC 9001**: Using TLS to Secure QUIC (https://www.rfc-editor.org/rfc/rfc9001)
- **RFC 9002**: QUIC Loss Detection and Congestion Control (https://www.rfc-editor.org/rfc/rfc9002)
- **RFC 9114**: HTTP/3 (https://www.rfc-editor.org/rfc/rfc9114)

### Rust Crates
- **quinn**: https://docs.rs/quinn (QUIC transport protocol)
- **h3**: https://docs.rs/h3 (HTTP/3 over quinn)
- **quiche**: https://docs.rs/quiche (Cloudflare's QUIC implementation)

### Key Documentation
- **QUIC Observability**: Fastly blog — "What Can Network Observability Tools See in QUIC?"
- **Cloudflare QUIC**: https://blog.cloudflare.com/quic-and-oss/ (Cloudflare's production QUIC deployment)
- **Apple NEPacketTunnelProvider**: https://developer.apple.com/documentation/networkextension/nepackettunnelprovider

### Existing ProxyBot Codebase References
- `/Users/doug/ai/system/proxybot/src-tauri/src/proxy.rs` — existing TCP/HTTP MITM proxy architecture
- `/Users/doug/ai/system/proxybot/docs/sdd/ios-vpn-research.md` — NEPacketTunnelProvider research for VPN mode

---

## 7. Conclusion

Adding QUIC support to ProxyBot is **conditionally feasible** but with significant limitations:

- **Observer mode** (pass-through with metadata logging) is achievable and low-risk
- **True MITM mode** (decryption + re-encryption) is **not feasible** without breaking QUIC's end-to-end encryption model
- The **recommended approach** is a quiche-based UDP observer that logs connection metadata
- **Long-term** investment in NEPacketTunnelProvider could provide true IP-layer capture for both TCP and QUIC

For the immediate term, the analytical value of QUIC observer mode is limited compared to TCP/HTTP MITM — HTTP headers and body are the primary inputs for app identification and traffic classification. QUIC observer mode would primarily add connection metadata (timing, byte counts) which has less discriminatory power for app identification.

**Recommendation:** Prioritize quinn + h3 HTTP/2 and broader TCP/HTTP coverage before investing in QUIC observer mode, unless QUIC traffic becomes dominant in the target use cases (WeChat, Douyin, Alipay).

---

*Research completed 2026-05-10. Document to be reviewed by Arch and Richard.*
