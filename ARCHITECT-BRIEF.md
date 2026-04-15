# Architect Brief — ProxyBot

## Step 9 — 导出 HAR 文件 ✅ (完成)

---

## Step 10 — CA 安装指引 UI

目标：在 Setup 面板内直接展示 iOS/Android 的 CA 证书安装步骤，用户不需要离开 App 就能完成安装配置。

### UI 实现

**Setup 面板新增「证书安装指引」区域**

分为两个 Tab：**iOS** 和 **Android**

**iOS 步骤：**
1. 点击「下载 CA 证书」按钮 → 调用 `get_ca_cert_path()` 获取路径
2. 用 `tauri-plugin-opener` 或 `open` 命令在浏览器打开 `~/.proxybot/ca.crt`
3. 或者直接显示：`safari://open?url=http://{PC_IP}:8080/ca.crt`
4. iOS Safari 会提示「此网站下载一个配置描述文件」
5. 用户依次：设置 → 通用 → VPN与设备管理 → 安装描述文件 → 通用 → 关于本机 → 证书信任设置 → 开启完全信任

**Android 步骤：**
1. 点击「下载 CA 证书」
2. 打开下载的 `ca.crt` 文件 → Android 提示输入锁屏密码 → 安装成功
3. Android 7+：部分 App 不信任用户 CA（安全限制），需额外开启「安装未知应用」或使用 ADB

**UI 设计：**

```tsx
<div className="ca-guide">
  <div className="ca-guide-tabs">
    <button className={activeTab === 'ios' ? 'active' : ''} onClick={() => setActiveTab('ios')}>iOS</button>
    <button className={activeTab === 'android' ? 'active' : ''} onClick={() => setActiveTab('android')}>Android</button>
  </div>

  {activeTab === 'ios' && (
    <ol className="ca-steps">
      <li>点击「下载 CA 证书」，Safari 会打开证书页面</li>
      <li>Safari 提示「此网站下载一个配置描述文件」→ 允许</li>
      <li>打开「设置」→「通用」→「VPN与设备管理」→ 找到 ProxyBot CA</li>
      <li>点击「安装」→ 输入锁屏密码 → 完成后在「关于本机」→「证书信任设置」开启完全信任</li>
    </ol>
  )}

  {activeTab === 'android' && (
    <ol className="ca-steps">
      <li>点击「下载 CA 证书」，下载完成后打开文件</li>
      <li>提示「为VPN和应用配置CA」→ 确认安装 → 输入锁屏密码</li>
      <li>部分Android 7+ App默认不信任用户CA，需在「设置」→「安全」→「凭证」→「从存储设备安装」</li>
    </ol>
  )}

  <button className="btn-download-ca" onClick={downloadCa}>
    📥 下载 CA 证书
  </button>
</div>
```

**下载 CA 功能：**

```tsx
const downloadCa = async () => {
  const path = await invoke<string>("get_ca_cert_path");
  // Use tauri-plugin-opener to open the file, or open in browser
  await invoke("plugin:opener|open", { path: `file://${path}` });
};
```

### 不做

- 扫码安装（二维码生成较复杂）
- 自动安装（iOS/Android 都不支持静默安装）
- Android ADB 辅助安装

### 验收标准

1. iOS Tab 下显示完整 4 步安装流程
2. Android Tab 下显示完整安装流程
3. 「下载 CA 证书」按钮点击后可打开/下载证书文件
4. 证书路径从 `get_ca_cert_path()` 获取
