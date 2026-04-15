# Architect Brief — ProxyBot

## Step 7 — Production Build（macOS .app 打包）

目标：生成可分发的 macOS .app 安装包，用户双击即可运行，无需命令行。

### 1. 配置 tauri.conf.json

```json
{
  "productName": "ProxyBot",
  "version": "0.1.0",
  "identifier": "com.proxybot.app",
  "build": {
    "devtools": true
  },
  "app": {
    "windows": [{
      "title": "ProxyBot",
      "width": 1100,
      "height": 750,
      "minWidth": 900,
      "minHeight": 600,
      "center": true,
      "resizable": true
    }],
    "security": {
      "csp": null
    }
  },
  "bundle": {
    "active": true,
    "targets": ["dmg", "app"],
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns"
    ],
    "category": "Developer Tools",
    "shortDescription": "HTTPS MITM Proxy Tool",
    "longDescription": "A macOS desktop proxy tool for developers. Captures and decrypts HTTPS/WSS traffic from mobile devices on the same LAN."
  }
}
```

### 2. macOS 代码签名 +公证（可选但推荐）

**签名（开发阶段可跳过）：**
- 在 Apple Developer 申请 Developer ID
- `codesign -f -s "Developer ID Application: Your Name" src-tauri/target/release/bundle/app/ProxyBot.app`
- `codesign -f -s "Developer ID Application: Your Name" -i com.proxybot.app --entitlements src-tauri/entitlements.plist src-tauri/target/release/bundle/dmg/proxybot_0.1.0_aarch64.dmg`

**公证（分发必需）：**
- `xcrun notarytool submit proxybot.dmg --apple-id "your@email.com" --password "app-password" --team-id "TEAMID"`
- 如果不用签名+公证：Xcode 14+ 默认不允许运行未签名 app，需要右键"打开"或 `xattr -cr`

**entitlements.plist（pf 功能必需）：**
```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>com.apple.security.network.client</key>
    <true/>
    <key>com.apple.security.network.server</key>
    <true/>
    <key>com.apple.security.files.user-selected.read-write</key>
    <true/>
</dict>
</plist>
```

### 3. 构建命令

```bash
npm run tauri build
```

输出：`src-tauri/target/release/bundle/dmg/proxybot_0.1.0_aarch64.dmg`

### 4. 自述文件 README

创建 `README.md` 包含：
- 功能介绍 + 截图
- 安装步骤（下载 .dmg，双击安装）
- iOS CA 证书安装指引（截图步骤）
- 手机配置步骤（网关/DNS 设为 PC IP）
- 常见问题（WeChat 证书固定处理）

### 验收标准

1. `npm run tauri build` 成功，无报错
2. 生成的 `.dmg` 文件可用（双击挂载）
3. 运行 ProxyBot.app，核心功能（透明代理/HTTPS 拦截/DNS 记录）正常
