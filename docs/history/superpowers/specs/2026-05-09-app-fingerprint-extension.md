# App Fingerprint Extension v0.9.0 设计方案

## Status: Draft

## 1. Overview

扩展App分类引擎，基于TLS握手特征/证书链识别App，不依赖DNS更准确。

**当前问题：**
- 依赖DNS相关性识别App
- DNS查询时机不对会失效
- 仅支持微信/抖音/支付宝

**目标：**
- 基于TLS ClientHello指纹
- 基于证书链特征
- 扩展支持更多App: TikTok, Amazon, etc.

---

## 2. 竞品分析

| 竞品 | 分类方案 |
|------|---------|
| ProxyBot (当前) | DNS相关性 |
| mitmproxy | 无App分类 |
| Proxyman | 基础分类 |

**ProxyBot独家: 基于TLS指纹的App识别** - 竞品无此功能

---

## 3. 技术方案

### 3.1 TLS ClientHello 指纹

TLS握手时，ClientHello包含：
- TLS版本
- 加密套件列表
- 椭圆曲线/扩展
- SNI (域名)
- 应用层协议 (ALPN)

这些特征的组合形成"指纹"：

```rust
struct TlsFingerprint {
    client_version: String,      // TLS 1.3
    cipher_suites: Vec<String>,  // ["TLS_AES_128_GCM_SHA256", ...]
    extensions: Vec<String>,    // ["server_name", "application_layer_protocol_negotiation"]
    elliptic_curves: Vec<String>, // ["x25519", "secp256r1"]
    alpn: Vec<String>,           // ["h2", "http/1.1"]
}
```

### 3.2 App特征库

```rust
struct AppSignature {
    app_id: String,           // "tiktok"
    app_name: String,         // "TikTok"
    fingerprints: Vec<TlsFingerprint>,
    sni_patterns: Vec<String>, // ["*.tiktokv.com", "*.tiktok.com"]
    issuer_patterns: Vec<String>, // CN pattern in cert chain
}

impl AppSignature {
    fn matches(&self, hello: &ClientHello) -> bool {
        // Check SNI first (fast path)
        if let Some(sni) = &hello.sni {
            if self.sni_patterns.iter().any(|p| glob_match(p, sni)) {
                return true;
            }
        }

        // Check fingerprint (精确匹配)
        let fp = extract_fingerprint(hello);
        if self.fingerprints.contains(&fp) {
            return true;
        }

        false
    }
}
```

### 3.3 内置App列表

```json
{
  "apps": [
    {
      "id": "tiktok",
      "name": "TikTok",
      "sni_patterns": ["*.tiktokv.com", "*.tiktok.com", "*.byteoversea.com"],
      "cipher_suites": ["TLS_AES_128_GCM_SHA256", "TLS_AES_256_GCM_SHA384"],
      "elliptic_curves": ["x25519"]
    },
    {
      "id": "wechat",
      "name": "WeChat",
      "sni_patterns": ["*.weixin.qq.com", "*.wechat.com", "*.qq.com"],
      "cipher_suites": ["TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384"]
    },
    {
      "id": "douyin",
      "name": "Douyin",
      "sni_patterns": ["*.douyin.com", "*.bytedance.com", "*.tiktokv.com"],
      "cipher_suites": ["TLS_AES_128_GCM_SHA256"]
    },
    {
      "id": "alipay",
      "name": "Alipay",
      "sni_patterns": ["*.alipay.com", "*.alipayusercontent.com"],
      "cipher_suites": ["TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384"]
    },
    {
      "id": "amazon",
      "name": "Amazon",
      "sni_patterns": ["*.amazon.com", "*.amazonaws.com", "*.amazon.co.uk"],
      "cipher_suites": ["TLS_AES_256_GCM_SHA384", "TLS_CHACHA20_POLY1305_SHA256"]
    },
    {
      "id": "apple",
      "name": "Apple Services",
      "sni_patterns": ["*.apple.com", "*.icloud.com", "*.mzstatic.com"],
      "cipher_suites": ["TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384"],
      "alpn": ["h2", "http/1.1"]
    }
  ]
}
```

---

## 4. 实现设计

### 4.1 Rust 结构

```rust
// app_classifier.rs

pub struct AppClassifier {
    signatures: Vec<AppSignature>,
    custom_rules: Vec<AppRule>,
}

impl AppClassifier {
    pub fn classify(&self, hello: &ClientHello) -> Option<AppMatch> {
        // 1. Check SNI patterns
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

        // 2. Check TLS fingerprint
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
}
```

### 4.2 分类结果

```rust
struct AppMatch {
    app_id: String,
    app_name: String,
    confidence: f32,  // 0.0 - 1.0
    source: MatchSource,
}

enum MatchSource {
    Sni,          // Based on SNI hostname
    Fingerprint,  // Based on TLS fingerprint
    Custom,       // Based on user rules
    Dns,          // Legacy DNS correlation
}
```

### 4.3 自定义规则

用户可以添加自定义规则:

```json
{
  "custom_rules": [
    {
      "app_id": "myapp",
      "app_name": "My Custom App",
      "conditions": [
        { "type": "sni", "pattern": "*.mycompany.com" },
        { "type": "cipher_suite", "value": "TLS_AES_128_GCM_SHA256" }
      ],
      "confidence": 0.8
    }
  ]
}
```

---

## 5. IPC 命令

```rust
#[tauri::command]
fn get_app_signatures() -> Result<Vec<AppSignature>, String>;

#[tauri::command]
fn add_custom_rule(rule: CustomAppRule) -> Result<(), String>;

#[tauri::command]
fn remove_custom_rule(app_id: String) -> Result<(), String>;
```

---

## 6. GUI 集成

在 Traffic/Devices 页面显示 App Badge:
- Badge 颜色按 App 区分
- Hover 显示置信度
- 点击显示详情

---

## 7. 验证

```bash
# 用TikTok app测试
# 检查TLS指纹是否匹配
# 验证fallback到DNS相关性
```
