# App Fingerprint Extension Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 扩展App分类引擎，基于TLS握手特征识别App，支持更多App和用户自定义规则

**Architecture:** Rust端实现AppClassifier + TLS指纹提取，前端显示App Badge

**Tech Stack:** Rust, React, Tauri IPC

---

## File Structure

| 操作 | 文件 | 职责 |
|------|------|------|
| Create | `src-tauri/src/classifier/app_classifier.rs` | App分类器 |
| Create | `src-tauri/src/classifier/tls_fingerprint.rs` | TLS指纹提取 |
| Create | `src-tauri/src/classifier/signatures.rs` | 内置App特征库 |
| Create | `src-tauri/src/classifier/mod.rs` | 模块导出 |
| Modify | `src-tauri/src/lib.rs` | 注册分类器 |
| Create | `src/components/shared/AppBadge.tsx` | App Badge组件 |
| Modify | `src-tauri/src/proxy.rs` | 集成分类器 |

---

## Task 1: 创建AppClassifier核心

**Files:**
- Create: `src-tauri/src/classifier/app_classifier.rs`

- [ ] **Step 1: 创建app_classifier.rs**

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppMatch {
    #[serde(rename = "appId")]
    pub app_id: String,
    #[serde(rename = "appName")]
    pub app_name: String,
    pub confidence: f32,
    pub source: MatchSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MatchSource {
    Sni,
    Fingerprint,
    Custom,
    Dns,
}

pub struct AppClassifier {
    signatures: Vec<AppSignature>,
    custom_rules: Vec<AppRule>,
}

#[derive(Debug, Clone)]
pub struct AppSignature {
    pub app_id: String,
    pub app_name: String,
    pub sni_patterns: Vec<String>,
    pub fingerprints: Vec<TlsFingerprint>,
    pub cipher_suites: Vec<String>,
    pub elliptic_curves: Vec<String>,
    pub alpn: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TlsFingerprint {
    pub client_version: String,
    pub cipher_suites: Vec<String>,
    pub extensions: Vec<String>,
    pub elliptic_curves: Vec<String>,
    pub alpn: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AppRule {
    pub app_id: String,
    pub app_name: String,
    pub conditions: Vec<RuleCondition>,
    pub confidence: f32,
}

#[derive(Debug, Clone)]
pub enum RuleCondition {
    Sni { pattern: String },
    CipherSuite { value: String },
    Extension { name: String },
    Alpn { value: String },
}

impl AppClassifier {
    pub fn new() -> Self {
        Self {
            signatures: crate::classifier::signatures::builtin_signatures(),
            custom_rules: Vec::new(),
        }
    }

    pub fn classify(&self, hello: &ClientHello) -> Option<AppMatch> {
        // 1. Check SNI patterns (fast path)
        if let Some(sni) = &hello.sni {
            for sig in &self.signatures {
                for pattern in &sig.sni_patterns {
                    if glob_match(pattern, sni) {
                        return Some(AppMatch {
                            app_id: sig.app_id.clone(),
                            app_name: sig.app_name.clone(),
                            confidence: 0.9,
                            source: MatchSource::Sni,
                        });
                    }
                }
            }
        }

        // 2. Check TLS fingerprint (精确匹配)
        let fp = extract_fingerprint(hello);
        for sig in &self.signatures {
            if sig.fingerprints.contains(&fp) {
                return Some(AppMatch {
                    app_id: sig.app_id.clone(),
                    app_name: sig.app_name.clone(),
                    confidence: 1.0,
                    source: MatchSource::Fingerprint,
                });
            }
        }

        // 3. Check custom rules
        for rule in &self.custom_rules {
            if rule.matches(hello) {
                return Some(AppMatch {
                    app_id: rule.app_id.clone(),
                    app_name: rule.app_name.clone(),
                    confidence: rule.confidence,
                    source: MatchSource::Custom,
                });
            }
        }

        None
    }

    pub fn add_custom_rule(&mut self, rule: AppRule) {
        self.custom_rules.push(rule);
    }
}

impl AppRule {
    pub fn matches(&self, hello: &ClientHello) -> bool {
        for condition in &self.conditions {
            match condition {
                RuleCondition::Sni { pattern } => {
                    if let Some(sni) = &hello.sni {
                        if !glob_match(pattern, sni) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                RuleCondition::CipherSuite { value } => {
                    if !hello.cipher_suites.contains(value) {
                        return false;
                    }
                }
                RuleCondition::Extension { name } => {
                    if !hello.extensions.contains(name) {
                        return false;
                    }
                }
                RuleCondition::Alpn { value } => {
                    if !hello.alpn.contains(value) {
                        return false;
                    }
                }
            }
        }
        true
    }
}

struct ClientHello {
    sni: Option<String>,
    cipher_suites: Vec<String>,
    extensions: Vec<String>,
    elliptic_curves: Vec<String>,
    alpn: Vec<String>,
}

fn extract_fingerprint(hello: &ClientHello) -> TlsFingerprint {
    TlsFingerprint {
        client_version: hello.client_version.clone(),
        cipher_suites: hello.cipher_suites.clone(),
        extensions: hello.extensions.clone(),
        elliptic_curves: hello.elliptic_curves.clone(),
        alpn: hello.alpn.clone(),
    }
}

fn glob_match(pattern: &str, value: &str) -> bool {
    let pattern = pattern.replace("*.", ".*").replace("*", ".*");
    if let Ok(re) = regex::Regex::new(&format!("^{}$", pattern)) {
        re.is_match(value)
    } else {
        false
    }
}
```

- [ ] **Step 2: Commit**

```bash
git add src-tauri/src/classifier/app_classifier.rs
git commit -m "feat(classifier): add AppClassifier core"
```

---

## Task 2: 创建内置App特征库

**Files:**
- Create: `src-tauri/src/classifier/signatures.rs`

- [ ] **Step 1: 创建signatures.rs**

```rust
use super::app_classifier::{AppSignature, TlsFingerprint};

pub fn builtin_signatures() -> Vec<AppSignature> {
    vec![
        AppSignature {
            app_id: "tiktok".to_string(),
            app_name: "TikTok".to_string(),
            sni_patterns: vec![
                "*.tiktokv.com".to_string(),
                "*.tiktok.com".to_string(),
                "*.byteoversea.com".to_string(),
            ],
            fingerprints: vec![],
            cipher_suites: vec![
                "TLS_AES_128_GCM_SHA256".to_string(),
                "TLS_AES_256_GCM_SHA384".to_string(),
            ],
            elliptic_curves: vec!["x25519".to_string()],
            alpn: vec![],
        },
        AppSignature {
            app_id: "wechat".to_string(),
            app_name: "WeChat".to_string(),
            sni_patterns: vec![
                "*.weixin.qq.com".to_string(),
                "*.wechat.com".to_string(),
                "*.qq.com".to_string(),
            ],
            fingerprints: vec![],
            cipher_suites: vec!["TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384".to_string()],
            elliptic_curves: vec![],
            alpn: vec![],
        },
        AppSignature {
            app_id: "douyin".to_string(),
            app_name: "Douyin".to_string(),
            sni_patterns: vec![
                "*.douyin.com".to_string(),
                "*.bytedance.com".to_string(),
                "*.tiktokv.com".to_string(),
            ],
            fingerprints: vec![],
            cipher_suites: vec!["TLS_AES_128_GCM_SHA256".to_string()],
            elliptic_curves: vec!["x25519".to_string()],
            alpn: vec![],
        },
        AppSignature {
            app_id: "alipay".to_string(),
            app_name: "Alipay".to_string(),
            sni_patterns: vec![
                "*.alipay.com".to_string(),
                "*.alipayusercontent.com".to_string(),
            ],
            fingerprints: vec![],
            cipher_suites: vec!["TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384".to_string()],
            elliptic_curves: vec![],
            alpn: vec![],
        },
        AppSignature {
            app_id: "amazon".to_string(),
            app_name: "Amazon".to_string(),
            sni_patterns: vec![
                "*.amazon.com".to_string(),
                "*.amazonaws.com".to_string(),
                "*.amazon.co.uk".to_string(),
            ],
            fingerprints: vec![],
            cipher_suites: vec![
                "TLS_AES_256_GCM_SHA384".to_string(),
                "TLS_CHACHA20_POLY1305_SHA256".to_string(),
            ],
            elliptic_curves: vec![],
            alpn: vec!["h2".to_string()],
        },
        AppSignature {
            app_id: "apple".to_string(),
            app_name: "Apple Services".to_string(),
            sni_patterns: vec![
                "*.apple.com".to_string(),
                "*.icloud.com".to_string(),
                "*.mzstatic.com".to_string(),
            ],
            fingerprints: vec![],
            cipher_suites: vec!["TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384".to_string()],
            elliptic_curves: vec![],
            alpn: vec!["h2".to_string(), "http/1.1".to_string()],
        },
    ]
}
```

- [ ] **Step 2: Commit**

```bash
git add src-tauri/src/classifier/signatures.rs
git commit -m "feat(classifier): add builtin app signatures"
```

---

## Task 3: 创建模块导出

**Files:**
- Create: `src-tauri/src/classifier/mod.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 创建mod.rs**

```rust
pub mod app_classifier;
pub mod tls_fingerprint;
pub mod signatures;

pub use app_classifier::*;
```

- [ ] **Step 2: 注册模块**

在 `src-tauri/src/lib.rs` 中添加:
```rust
pub mod classifier;
pub use classifier::*;
```

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/classifier/mod.rs src-tauri/src/lib.rs
git commit -m "feat(classifier): register classifier module"
```

---

## Task 4: 创建AppBadge组件

**Files:**
- Create: `src/components/shared/AppBadge.tsx`

- [ ] **Step 1: 创建AppBadge**

```tsx
interface AppBadgeProps {
  appId: string;
  appName: string;
  confidence?: number;
  size?: "sm" | "md";
}

const appColors: Record<string, string> = {
  wechat: "bg-green-100 text-green-800",
  douyin: "bg-pink-100 text-pink-800",
  tiktok: "bg-pink-100 text-pink-800",
  alipay: "bg-blue-100 text-blue-800",
  amazon: "bg-orange-100 text-orange-800",
  apple: "bg-gray-100 text-gray-800",
};

export function AppBadge({ appId, appName, confidence, size = "sm" }: AppBadgeProps) {
  const colorClass = appColors[appId] || "bg-gray-100 text-gray-800";
  const sizeClass = size === "sm" ? "px-2 py-0.5 text-xs" : "px-3 py-1 text-sm";

  return (
    <span className={`inline-flex items-center rounded-full font-medium ${colorClass} ${sizeClass}`}>
      {appName}
      {confidence !== undefined && confidence < 1.0 && (
        <span className="ml-1 opacity-60">{Math.round(confidence * 100)}%</span>
      )}
    </span>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add src/components/shared/AppBadge.tsx
git commit -m "feat(classifier): add AppBadge component"
```

---

## Task 5: 集成到Proxy流程

**Files:**
- Modify: `src-tauri/src/proxy.rs`

- [ ] **Step 1: 集成分类器**

在处理新连接时调用分类器:

```rust
use crate::classifier::AppClassifier;

struct ProxyState {
    classifier: AppClassifier,
    // ...
}

impl ProxyState {
    fn classify_connection(&self, hello: &ClientHello) -> Option<AppMatch> {
        self.classifier.classify(hello)
    }
}
```

- [ ] **Step 2: Commit**

```bash
git add src-tauri/src/proxy.rs
git commit -m "feat(classifier): integrate classifier into proxy"
```

---

## Task 6: 编译验证

- [ ] **Step 1: 运行测试**

```bash
cd src-tauri && cargo test classifier -- --nocapture
```

- [ ] **Step 2: 编译**

```bash
npm run build 2>&1 | tail -20
```

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat(classifier): complete app fingerprint extension"
```

---

## 验证清单

- [ ] AppBadge正确显示颜色和名称
- [ ] TikTok/WeChat/Douyin/Alipay分类正常
- [ ] 自定义规则添加成功
- [ ] TLS指纹匹配正确
- [ ] 编译通过
