# proxybot-core

Core MITM proxy engine library for [ProxyBot](https://github.com/mbpz/proxybot).
No Tauri, GUI, or desktop dependencies — pure Rust.

## Modules

- **types** — shared data types (`InterceptedRequest`, `Rule`, `DnsEntry`, rules,
  analysis models)
- **config** — centralized configuration with environment-variable overrides
- **proxy_engine** — MITM runtime: listeners, TLS interception, HTTP/HTTPS and
  WebSocket forwarding, capture events, breakpoint and hook seams
- **cert_manager** — root CA and per-host leaf certificate management
- **app_classifier** — domain and TLS-signature application attribution with a
  built-in catalogue and custom-rule loading
- **application_identity** — application identity and fingerprint models
- **fingerprint** — TLS/ALPN/cipher fingerprint evidence used by attribution
- **rules_engine** — domain matching and priority-based routing-rule evaluation
- **tls_rules** — per-host TLS decryption/bypass rules
- **body** — body decompression and decoding helpers
- **desktop_contract** — contract traits used to generate typed desktop bindings
- **specgen** — specification-generation primitives

The crate exposes runtime seams (`RuntimeHooks`, `OriginalDestination`) so the
desktop and headless adapters can supply persistence, analysis, and routing
without the engine depending on them.

## Usage

```rust
use proxybot_core::{classify_host, RulesEngine};

let engine = RulesEngine::new();

if let Some((app, icon)) = classify_host("api.weixin.qq.com") {
    println!("Traffic from {icon} {app}");
}
```

`CertManager::new` takes the CA storage directory as a `PathBuf` and returns a
`Result`; see `proxybot-core/src/cert_manager.rs` for the full API.

## License

MIT
