# Step 5 Review Request — WSS (WebSocket over HTTPS) Interception

**Ready for Review: YES**

## Summary

Built WebSocket interception for HTTPS CONNECT tunnels. After TLS MITM handshake in `handle_https_connect`, the proxy reads the HTTP request to detect `Upgrade: websocket` headers. If WebSocket upgrade is detected, the proxy now properly forwards the upgrade request to the upstream server, receives the 101 response, and forwards it to the browser before relaying WebSocket frames bidirectionally using `tokio-tungstenite`. Each Text/Binary frame emits an `intercepted-wss` event to the frontend. Added WSS tab to UI showing message flow.

## New File

None.

## Modified Files

### src-tauri/Cargo.toml (lines 32-36)
- Added `tokio-tungstenite = "0.26"`, `tungstenite = "0.22"`, `sha1 = "0.10"`, `base64 = "0.22"`, `futures-util = "0.3"`

### src-tauri/src/proxy.rs

**Imports (lines 27-29)**
- `use sha1::Digest;`
- `use base64::Engine;`
- `use futures_util::{SinkExt, StreamExt};`

**WssMessage struct (lines 47-57)**
```rust
pub struct WssMessage {
    pub id: String,
    pub timestamp: String,
    pub host: String,
    pub direction: String,  // "up" or "down"
    pub size: usize,
    pub content: String,
    pub app_name: Option<String>,
    pub app_icon: Option<String>,
}
```

**is_websocket_upgrade() (lines 66-97)**
- Parses HTTP request data to find `Upgrade: websocket` and `Connection: Upgrade` headers
- Returns `Some((Sec-WebSocket-Key, Sec-WebSocket-Protocol))` if upgrade detected, None otherwise
- Now extracts both `Sec-WebSocket-Key` and `Sec-WebSocket-Protocol` headers

**compute_ws_accept_key() (lines 99-107)**
- RFC 6455: `base64(SHA1(key + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11"))`
- Now uses `base64::engine::general_purpose::STANDARD.encode()` instead of custom implementation

**handle_websocket_relay() (lines 410-615)**
- Takes browser TLS stream (server role) and upstream TLS stream (client role)
- Creates `WebSocketStream` using `from_raw_socket()` with appropriate Role
- Uses mpsc channels for bidirectional relay between tasks
- Each Text/Binary frame: emits `intercepted-wss` event, forwards to peer
- Ping: responds with Pong directly to sender (does not relay)
- Close: relays to peer, closes local connection

**handle_https_connect() MITM WebSocket fix (lines ~757-845)**
- After TLS handshake, reads HTTP request from `client_tls_stream`
- Calls `is_websocket_upgrade()` to detect WebSocket
- **Proper MITM relay (FIXED)**:
  1. Forwards the HTTP upgrade request to upstream server via `upstream_tls_stream.write_all(&http_data)`
  2. Reads the HTTP response from upstream
  3. Checks if it's a 101 response
  4. If 101: extracts `Sec-WebSocket-Protocol` from upstream response if present, includes in 101 to browser
  5. Sends 101 Switching Protocols response to browser
  6. Calls `handle_websocket_relay()`
  7. If not 101: falls back to blind relay, forwarding the non-101 response to browser first

**base64_encode() (lines 109-111)**
- Replaced custom base64 implementation with `base64::engine::general_purpose::STANDARD.encode()`

### src/App.tsx

**WssMessage interface (lines 19-28)**
```typescript
interface WssMessage {
  id: string;
  timestamp: string;
  host: string;
  direction: string;
  size: number;
  content: string;
  app_name?: string;
  app_icon?: string;
}
```

**State additions (lines 44-45, 57-59)**
- `wssMessages: WssMessage[]` (max 200)
- `selectedWssTab: AppTab`
- Event listener for `intercepted-wss` event

**WSS Tab UI (lines 289-345)**
- Tab filter buttons: All / WeChat / Douyin / Alipay / Unknown
- Table columns: App | Time | Direction | Host | Size | Content Preview
- Direction shown as arrow (up/down) with CSS class
- Content preview truncated to 50 chars

## Bug Fixes Applied

1. **MITM WebSocket relay (proxy.rs)** - Fixed the critical bug where the proxy was sending 101 directly to browser without forwarding the upgrade request to upstream. Now properly forwards the HTTP upgrade request, reads the upstream 101 response, and forwards it to browser before starting WebSocket frame relay.

2. **Sec-WebSocket-Protocol header (proxy.rs)** - The 101 response now includes `Sec-WebSocket-Protocol` if the client offered it and the server accepted it (per RFC 6455).

3. **base64 crate (proxy.rs)** - Replaced custom `base64_encode()` function with `base64::engine::general_purpose::STANDARD.encode()` for standards-compliant encoding.

## Build Verification

- `cargo check` in src-tauri/: **0 errors**
- `npm run build`: **successful**

## Open Questions

1. **Channel-based relay complexity**: The relay uses `tokio::spawn` with `tokio::select!` and mpsc channels. This is more complex than a simple loop but allows concurrent bidirectional relay. Is there a simpler approach that avoids the channel ownership issues?

2. **WSS tab grouping**: Currently messages are shown in a flat list filtered by App. Should WSS connections be grouped by Host for better organization?
