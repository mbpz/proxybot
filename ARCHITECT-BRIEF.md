# Architect Brief — ProxyBot

## Step 1 — Tauri 骨架 + HTTPS MITM 代理 ✅ (完成验收)

## Step 2 — pf 透明代理 + IP 转发 ✅ (完成验收)

## Step 3 — 内置 DNS 服务器 ✅ (完成验收)

---

## Step 4 — App 分类规则库

目标：根据请求域名，将流量归类到对应的 App（微信/抖音/支付宝）。

### 规则库设计

**文件：** `src-tauri/src/app_rules.rs`（新建）

```rust
pub struct AppRule {
    pub name: &'static str,          // "WeChat" / "Douyin" / "Alipay"
    pub icon: &'static str,          // emoji 或颜色标识
    pub domains: Vec<&'static str>,  // 匹配规则
}

// 预置规则
pub static APP_RULES: &[AppRule] = &[
    AppRule {
        name: "WeChat",
        icon: "💬",
        domains: vec![
            "weixin.qq.com",
            "wechat.com",
            "qq.com",
            "wechatcdn.com",
            "wxs.qq.com",
            "longurl.cn", // 微信长连接域名
        ],
    },
    AppRule {
        name: "Douyin",
        icon: "🎵",
        domains: vec![
            "douyin.com",
            "tiktokv.com",
            "bytecdn.com",
            "douyinvod.com",
            "byted-static.com",
        ],
    },
    AppRule {
        name: "Alipay",
        icon: "💳",
        domains: vec![
            "alipay.com",
            "alipayusercontent.com",
            "alipay.com.cn",
            "alicdn.com",
        ],
    },
];
```

**域名匹配逻辑：**
- 精确匹配：`domain == rule.domain`
- 通配符匹配：`domain.ends_with(rule.domain)` 或 `domain == rule.domain`（支持 `*.qq.com` 简写）
- 不在规则库中的请求：归类为 "Unknown"

**每个拦截请求附加分类信息：**
```rust
struct InterceptedRequest {
    // ... 现有字段 ...
    pub app_name: Option<String>,   // "WeChat" / "Douyin" / "Alipay" / null
    pub app_icon: Option<String>,   // emoji
}
```

### Rust 后端修改

1. **`app_rules.rs`**（新建）：规则库 + 匹配函数
2. **`proxy.rs`**：每个新请求通过 `classify_request(host: &str) -> Option<AppInfo>` 分类，结果写入 Tauri event 一起推送
3. **`AppRules` 状态**：无需持久化，每次请求只读规则做匹配，无状态

```rust
// 分类函数签名
pub fn classify_host(host: &str) -> Option<(&'static str, &'static str)> {
    // 遍历 APP_RULES，返回 (app_name, app_icon)
    // 不匹配返回 None
}
```

### UI 修改

- 现有请求列表增加两列：**App**（emoji + 名称）和 **域名**
- 或者：按 App 分组展示，每个 App 一个折叠面板，下方列出该 App 的所有请求
- 推荐：**分组视图**，顶部 Tab 切换：「全部」/「WeChat」/「Douyin」/「Alipay」/「Unknown」

### 不做

- 持久化规则库
- 规则在线更新
- DPI / TLS 指纹识别（纯域名匹配足够）

### 验收标准

1. 打开微信，微信相关域名（`*.weixin.qq.com`、`*.qq.com`）出现在 UI，标记为 WeChat 💬
2. 打开抖音，抖音相关域名出现在 UI，标记为 Douyin 🎵
3. 打开支付宝，支付宝相关域名出现在 UI，标记为 Alipay 💳
4. 打开 Safari 访问 `baike.baidu.com`，标记为 Unknown
