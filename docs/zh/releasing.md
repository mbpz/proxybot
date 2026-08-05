# 发布 ProxyBot

公开 Release 只能由 `.github/workflows/release.yml` 生成。本地构建可用于开发，
但只有托管工作流完成签名、公证、验证、证明与发布后，才是公开 Release。

## 产品版本

`package.json` 是规范版本来源。Tauri 与更新界面直接读取它，Rust 与
MCP 版本则由一致性工具校验。稳定的 macOS Bundle Identifier 为
`com.mbpz.proxybot`。

准备版本升级：

```bash
pnpm version:set 1.3.1
pnpm version:check
pnpm ci:local
```

创建 Tag 前先提交版本变更。Release 工作流会拒绝任何不等于
`v<package version>` 的 Tag。

## 必需的仓库 Secrets

缺少以下任一凭据时，工作流会明确失败，而不会发布 ad-hoc 签名的应用：

- `APPLE_CERTIFICATE` — base64 编码的 Developer ID Application 证书
- `APPLE_CERTIFICATE_PASSWORD`
- `APPLE_SIGNING_IDENTITY`
- `APPLE_ID`
- `APPLE_PASSWORD` — App 专用密码
- `APPLE_TEAM_ID`
- `KEYCHAIN_PASSWORD` — CI 临时钥匙串密码

这些值只能保存在 GitHub Actions Secrets 中。不得向仓库提交证书、私钥或密码。
凭据配置与托管证据由 [GitHub issue #27](https://github.com/mbpz/proxybot/issues/27)
跟踪。

## 发布

版本提交进入绿色的 `main` 后，创建并推送准确的 Tag：

```bash
git tag -s v1.3.1 -m "ProxyBot v1.3.1"
git push origin v1.3.1
```

工作流通过 Tauri bundler 构建 Apple Silicon 与 Intel DMG。每个架构都会发布签名
并公证的 `.dmg`、SHA-256、SPDX JSON SBOM 和 GitHub 构建来源证明。上传前，CI
会挂载 DMG，检查 `CFBundleShortVersionString`，并通过 `codesign` 与 `spctl`
以及 `stapler` 验证应用。随后直接运行包内可执行文件的隔离桌面验收旅程：准备 CA、
解密并持久化一个本地 HTTPS 请求、停止抓包、重启并再次停止。Release Notes 由
GitHub 根据 Tag 自动生成。

## 证据边界

存在工作流不等于 Release 已成功。只有记录托管运行 URL，并验证两个架构都能安装
和启动后，才能称该 Release 已验证。Homebrew 仍不受支持，直到有维护中的 Tap 能
安装这些已验证产物并通过自身冒烟测试。
