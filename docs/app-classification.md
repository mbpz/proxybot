# App Classification Guide

ProxyBot automatically identifies which app generated each request by
correlating DNS queries with observed traffic and matching hostnames against
a built-in domain rule library.

## How It Works

```
Phone DNS query → DNS Server logs "api.weixin.qq.com" from 192.168.1.100
Phone HTTPS request → SNI = "api.weixin.qq.com", client IP = 192.168.1.100
                     → Correlation: this request belongs to WeChat 💬
```

Three-stage classification pipeline:

1. **DNS correlation** — DNS query logs map domain → client IP
2. **SNI inspection** — TLS ClientHello reveals the target hostname
3. **Domain rules** — Known app domains are matched against the hostname

## Built-in App Rules (v1.3+)

ProxyBot ships with **28+** pre-configured app rules covering:

### Social & Communication
- **WeChat** 💬 — `*.weixin.qq.com`, `*.wechat.com`, `*.qq.com`, `*.wechatpay.com` (14 domains)
- **Weibo** 📣 — `*.weibo.com`, `*.sinaimg.cn`, `*.sina.com.cn` (6 domains)
- **QQ** 🐧 — `*.qpic.cn`, `*.qlogo.cn`, `*.tencent.com` (6 domains)

### Short Video
- **Douyin** 🎵 — `*.douyin.com`, `*.tiktokv.com`, `*.bytecdn.com` (10 domains)
- **Kuaishou** 📹 — `*.kuaishou.com`, `*.yximgs.com` (4 domains)

### E-commerce
- **Taobao** 🛒 — `*.taobao.com`, `*.tmall.com`, `*.alicdn.com` (8 domains)
- **JD** 🐕 — `*.jd.com`, `*.360buyimg.com`, `*.jdpay.com` (5 domains)
- **Pinduoduo** 🔶 — `*.pinduoduo.com`, `*.yangkeduo.com` (3 domains)
- **Meituan** 🛵 — `*.meituan.com`, `*.dianping.com` (4 domains)

### Lifestyle
- **Xiaohongshu** 📕 — `*.xiaohongshu.com`, `*.xhscdn.com` (3 domains)
- **Didi** 🚗 — `*.didi.cn`, `*.didiglobal.com` (3 domains)

### Content & Video
- **Bilibili** 📺 — `*.bilibili.com`, `*.biliapi.net`, `*.hdslb.com` (5 domains)
- **iQiyi** 🎬 — `*.iqiyi.com`, `*.iqiyipic.com` (3 domains)
- **Tencent Video** ▶️ — `*.qqvideo.com`, `*.smtcdns.net` (3 domains)
- **NetEase** 🎶 — `*.163.com`, `*.126.net`, `*.netease.com` (5 domains)

### Search & Info
- **Baidu** 🔍 — `*.baidu.com`, `*.bdstatic.com`, `*.bcebos.com` (5 domains)
- **Zhihu** 🤔 — `*.zhihu.com`, `*.zhimg.com` (3 domains)

### Finance
- **Alipay** 💳 — `*.alipay.com`, `*.antgroup.com`, `*.mybank.com` (7 domains)

### AI Providers
- **OpenAI** O — `api.openai.com`, `*.openai.com`
- **Anthropic** A — `api.anthropic.com`, `*.anthropic.com`
- **Azure OpenAI** Z — `*.openai.azure.com`, `*.cognitiveservices.azure.com`
- **Google AI** G — `generativelanguage.googleapis.com`
- **Cohere** C — `api.cohere.ai`
- **Groq** Q — `api.groq.com`
- **DeepSeek** D — `api.deepseek.com`, `*.deepseek.com`
- **Moonshot** M — `api.moonshot.cn`, `*.moonshot.cn`
- **Zhipu** Z — `open.bigmodel.cn`, `*.bigmodel.cn`
- **MiniMax** M — `api.minimax.chat`, `*.minimax.chat`

## Customizing App Rules

Add or override rules by creating `~/.proxybot/app_rules.json`:

```json
[
  {
    "name": "MyApp",
    "icon": "🆕",
    "domains": [
      "myapp.com",
      "api.myapp.com",
      "myapp-cdn.com"
    ]
  }
]
```

**Rules in this file replace the built-in defaults entirely.** To extend the
defaults instead, copy the full built-in rules and append your custom entries.

### Domain Matching Rules

- **Exact match**: `"api.myapp.com"` matches only `api.myapp.com`
- **Subdomain match**: `"myapp.com"` matches `api.myapp.com`, `cdn.myapp.com`, etc.
- **False-positive safe**: `"qq.com"` does NOT match `qq.com.evil.com`

### Rule Priority

When a hostname matches multiple apps, the **first matching rule** takes
precedence. Rules are evaluated in the order they appear in the JSON array.

## Testing Classification

Use the `classify_request` MCP tool or check the Traffic tab:

```bash
# In Claude Desktop with ProxyBot MCP Server:
"Classify the host api.weixin.qq.com"
# → WeChat 💬

"Classify the host api.m.jd.com"
# → JD 🐕
```

Or via the TUI — classified requests show the app icon and name badge next
to the hostname in the traffic list.

## Adding New Domains

If you discover domains for an app that aren't covered, you can:

1. **File app_rules.json** — Add to `~/.proxybot/app_rules.json` (takes effect immediately)
2. **GitHub PR** — Submit a PR to `proxybot-core/src/app_classifier.rs` with new domains
3. **Issue** — Open a GitHub issue with the app name and observed domains

Please include:
- App name (Chinese + English)
- Domain list (each on a new line)
- How you observed the traffic (which phone app triggered it)
- Whether the domains are CDN/static or API endpoints
