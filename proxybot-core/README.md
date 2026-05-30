# proxybot-core

Core MITM proxy engine library for [ProxyBot](https://github.com/mbpz/proxybot).  
No Tauri, GUI, or desktop dependencies — pure Rust.

## Modules

- **types** — Shared data types (InterceptedRequest, Rule, DnsEntry)
- **config** — Centralized configuration with env-var overrides
- **app_classifier** — Domain-based app identification (WeChat, Douyin, Alipay)
- **cert_manager** — Root CA and per-host leaf certificate management
- **rules_engine** — Domain matching and priority-based rule evaluation
- **proxy_engine** — HTTP/HTTPS proxy engine core logic
- **dns_state** — DNS query tracking and correlation

## Usage

```rust
use proxybot_core::{CertManager, RulesEngine, classify_host};

let cert_mgr = CertManager::new(None).unwrap();
let engine = RulesEngine::new();

if let Some((app, icon)) = classify_host("api.weixin.qq.com") {
    println!("Traffic from {} {}", icon, app);
}
```

## License

MIT
