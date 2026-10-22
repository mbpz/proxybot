# Network Conditions Design Specification

**Goal:** Simulate real-world network conditions (latency, bandwidth, packet loss) during proxy interception for testing app behavior under degraded connectivity.

**Architecture:** NetworkConditionEngine holds presets and per-host rules. Injection happens at the TCP pipe level — `pipe_tcp_bidirectional` and `handle_https_connect` apply latency/bandwidth/loss between read and write operations.

**Tech Stack:** Rust (tokio::time::sleep for latency, rand for packet loss), YAML for profiles

---

## 1. Data Structures

### 1.1 NetworkProfile

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkProfile {
    pub name: String,
    pub latency_ms: u64,       // 0-10000
    pub bandwidth_kbps: u64,   // 0 = unlimited
    pub packet_loss_pct: u8,    // 0-100
}
```

### 1.2 ConditionRule

```rust
#[derive(Clone, Debug)]
pub struct ConditionRule {
    pub id: u64,
    pub pattern: RulePattern,   // reuse plugin rule_engine
    pub profile: String,        // profile name
    pub enabled: bool,
}
```

### 1.3 NetworkConditionEngine

```rust
pub struct NetworkConditionEngine {
    profiles: RwLock<HashMap<String, NetworkProfile>>,
    rules: RwLock<Vec<ConditionRule>>,
    active_profile: RwLock<Option<NetworkProfile>>,
}
```

---

## 2. Built-in Presets

```yaml
presets:
  - name: "2G"
    latency_ms: 800
    bandwidth_kbps: 50
    packet_loss_pct: 2
  - name: "3G"
    latency_ms: 300
    bandwidth_kbps: 750
    packet_loss_pct: 1
  - name: "4G"
    latency_ms: 100
    bandwidth_kbps: 10000
    packet_loss_pct: 0
  - name: "WiFi"
    latency_ms: 5
    bandwidth_kbps: 0
    packet_loss_pct: 0
  - name: "Edge"
    latency_ms: 1200
    bandwidth_kbps: 30
    packet_loss_pct: 5
```

---

## 3. Injection

### 3.1 Latency

After `read()` and before `write_all()`, if `latency_ms > 0`:
```rust
tokio::time::sleep(Duration::from_millis(profile.latency_ms)).await;
```

### 3.2 Bandwidth

After read of N bytes, compute minimum time to transmit at capped rate:
```rust
let min_transfer_ms = (n as u64 * 8000) / profile.bandwidth_kbps;
if min_transfer_ms > 0 {
    tokio::time::sleep(Duration::from_millis(min_transfer_ms)).await;
}
```

### 3.3 Packet Loss

Before `write_all()`, randomly drop the chunk:
```rust
if rand::random::<u8>() % 100 < profile.packet_loss_pct {
    continue; // drop this chunk
}
```

---

## 4. API

```rust
impl NetworkConditionEngine {
    pub fn new() -> Self;
    pub fn load_profiles(path: &Path) -> Result<Self, String>;
    pub fn set_active(&self, profile_name: &str) -> Result<(), String>;
    pub fn disable(&self);
    pub fn get_active(&self) -> Option<NetworkProfile>;
    pub fn list_profiles(&self) -> Vec<NetworkProfile>;
    pub fn add_rule(&self, rule: ConditionRule);
    pub fn remove_rule(&self, id: u64);
    pub fn apply(&self, read_size: usize) -> ConditionEffect;
}
```

---

## 5. CLI Commands

```bash
proxybot network preset 3g           # Apply built-in preset
proxybot network latency 500         # Override latency (ms)
proxybot network bandwidth 128       # Override bandwidth (kbps)
proxybot network loss 5              # Override packet loss (%)
proxybot network off                 # Disable conditions
proxybot network status              # Show active profile
```

---

## 6. File Structure

```
src-tauri/src/
├── network/
│   ├── mod.rs          # Module exports
│   ├── profile.rs      # NetworkProfile, presets
│   └── engine.rs       # NetworkConditionEngine, ConditionRule
```

---

## 7. Test Plan

1. Unit test: latency applied (measure elapsed >= latency_ms)
2. Unit test: bandwidth cap (large read takes proportional time)
3. Unit test: packet loss (statistical over many iterations)
4. Integration: preset 3g applies all three conditions
