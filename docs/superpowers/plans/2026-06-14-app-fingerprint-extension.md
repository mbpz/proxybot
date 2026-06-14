# App Fingerprint Extension Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在 app_classifier 引擎中增加基于 TLS ClientHello 指纹的 App 分类能力，扩展内置 App 列表（TikTok/Amazon/Apple），并暴露 3 个 Tauri 命令用于签名查询与自定义规则持久化。

**Architecture:**
- `proxybot-core/src/app_classifier.rs` — 扩展 `AppRule` 增加 `fingerprints: Vec<TlsFingerprint>` 和 `sni_patterns: Vec<String>`；新增 `MatchSource` enum 和 `AppMatch` 结构；实现 `classify(hello: &HelloInfo) -> Option<AppMatch>` 按 SNI → fingerprint → custom 优先级匹配。
- `proxybot-core/src/fingerprint.rs` — 新增独立的 TLS 指纹模块，定义 `TlsFingerprint`、`MatchSource`、`AppMatch`、`HelloInfo` 类型及默认签名表（spec §3.3）。
- `src-tauri/src/commands/app_fingerprint.rs` — 3 个 Tauri 命令（get/add/remove custom rules）。
- `src-tauri/src/lib.rs` — 注册新命令。

**Tech Stack:** Rust, serde, Tauri 2.

---

## File Structure

| 操作 | 文件 | 职责 |
|------|------|------|
| Modify | `proxybot-core/src/app_classifier.rs` | 扩展 `AppRule`，添加指纹字段；新增 `classify()` |
| Create | `proxybot-core/src/fingerprint.rs` | 新增 TLS 指纹类型 + 默认签名表 |
| Modify | `proxybot-core/src/lib.rs` | 导出 fingerprint 模块 |
| Modify | `proxybot-core/src/types.rs` | 给 `AppRule` 加 `#[serde(default)]` 字段 |
| Create | `src-tauri/src/commands/app_fingerprint.rs` | 3 个 Tauri 命令 |
| Modify | `src-tauri/src/commands/mod.rs` | 注册新模块 |
| Modify | `src-tauri/src/lib.rs` | 注册新命令 |

---

## Tasks

### Task 1: 新增 fingerprint.rs 模块

**Files:**
- Create: `proxybot-core/src/fingerprint.rs`

**Step 1: Write the test skeleton (RED)**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tls_fingerprint_eq_and_hash() {
        let a = TlsFingerprint::new(
            "TLS 1.3".into(),
            vec!["TLS_AES_128_GCM_SHA256".into()],
            vec!["server_name".into()],
            vec!["x25519".into()],
            vec!["h2".into()],
        );
        let b = a.clone();
        assert_eq!(a, b);
        let mut set = HashSet::new();
        set.insert(a);
        assert!(set.contains(&b));
    }
}
```

**Step 2: Implement (GREEN)**

Define `TlsFingerprint`, `MatchSource`, `AppMatch`, `HelloInfo`, `AppSignature`, `CustomAppRule`。所有 `TlsFingerprint` 字段使用 `Vec<String>` 便于 JSON 序列化。`impl PartialEq, Eq, Hash` 基于全部字段。

**Step 3: Add `get_default_signatures()` returning TikTok/WeChat/Douyin/Alipay/Amazon/Apple per spec §3.3**

```rust
pub fn get_default_signatures() -> Vec<AppSignature> { vec![ ... ] }
```

---

### Task 2: 扩展 `AppRule` 类型并增加 `classify()`

**Files:**
- Modify: `proxybot-core/src/types.rs` (加 `#[serde(default)]` 字段)
- Modify: `proxybot-core/src/app_classifier.rs`

**Step 1: Tests (RED)**

```rust
#[test]
fn classify_sni_match_returns_sni_source() { ... }
#[test]
fn classify_fingerprint_match_wins_over_sni() { ... }
#[test]
fn classify_no_match_returns_none() { ... }
#[test]
fn classify_custom_rule_runs_last() { ... }
```

**Step 2: Implement `classify(hello: &HelloInfo) -> Option<AppMatch>`** 遵循 spec §4.1 的优先级链：
1. SNI pattern match (confidence 0.9, source Sni)
2. TLS fingerprint match (confidence 1.0, source Fingerprint)
3. Custom rule match (confidence from rule, source Custom)

保留 `classify_host()` 旧 API 不破坏，向后兼容。

---

### Task 3: 扩展 `get_default_rules()` 加入新 App

**Files:**
- Modify: `proxybot-core/src/app_classifier.rs`

**Step 1: Tests (RED)**

```rust
#[test]
fn default_rules_includes_tiktok_amazon_apple() { ... }
```

**Step 2: Add to `get_default_rules()`**：TikTok、Instagram、Snapchat、Telegram、Netflix、Spotify、Amazon、Microsoft、Apple、Twitter/X、Meta、PayPal。完整 SNI 列表参考业内已知域名。

---

### Task 4: 3 个 Tauri 命令

**Files:**
- Create: `src-tauri/src/commands/app_fingerprint.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`

**Step 1: Tests (RED)**

```rust
#[test]
fn get_app_signatures_returns_merged() { ... }
#[test]
fn add_and_remove_custom_rule_persists() { ... }
```

**Step 2: Implement commands** — `~/.proxybot/app_signatures.json` 存储自定义规则。`get_app_signatures` 返回默认+自定义合并列表；`add_custom_rule` 追加；`remove_custom_rule` 按 app_id 移除。

---

### Task 5: 注册命令 + 编译验证

**Files:**
- Modify: `src-tauri/src/lib.rs`

**Step 1: 验证**

```bash
cargo build
cargo test -p proxybot-core
cargo test -p proxybot
```

预期 proxybot-core 新增 ≥8 单元测试（fingerprint 默认签名/classify 优先级/默认 AppRule 列表等），全部通过。

---

## Backward Compat

- 现有 `AppRule` JSON 格式（`name`/`icon`/`domains`）保持有效，新字段默认空。
- `classify_host()` 不动，新 `classify()` 走 hello 路径。
- 现有 `load_app_rules()` 行为不变。

## Out of Scope (yolo mode v0.9.0)

- GUI badge 添加 confidence 展示（spec §6 推迟到 GUI 阶段）
- 真实 ClientHello 解析器（spec §3.1 的 `extract_fingerprint()` 由 proxy 端后续集成）
- 证书链 issuer_patterns 匹配
