# ProxyBot 更新检测 + 图标设计

**Date:** 2026-05-14
**Status:** Approved

---

## 1. App 图标设计

### 设计方向
- **风格:** 数据包流动 (Packet Flow)
- **概念:** 上下流动的数据点穿过中央节点，代表流量拦截和分析

### 配色方案 - 赛博霓虹
- **Primary:** `#a855f7` (紫色)
- **Secondary:** `#f472b6` (粉色)
- **Accent:** `#22d3ee` (青色)
- **Background:** 深色 `#1e1b4b`
- **效果:** 霓虹发光效果 (Neon glow filter)

### 视觉元素
1. **三个数据包节点** - 上中下三个圆点，渐变大小
2. **两条连接线** - 虚线表示数据流动
3. **箭头指示** - 表示流量方向
4. **发光滤镜** - 霓虹发光效果

### 图标尺寸要求
- 16x16, 32x32, 64x64, 128x128, 256x256, 512x512, 1024x1024

---

## 2. 更新检测功能

### 功能概述
检测 GitHub Releases 获取最新版本，提示用户更新。

### 检查时机
- **自动检查:** App 启动时后台检查
- **手动检查:** 设置页面有 "检查更新" 按钮

### 触发逻辑
```
启动时:
  - 调用 GitHub API 获取最新 release tag
  - 比较版本号 (语义化版本)
  - 如果有新版本，设置 hasUpdate = true

手动检查:
  - 用户点击 "检查更新"
  - 显示 loading 状态
  - 调用 API，获取结果
  - 更新 UI 状态
```

### API 调用
```typescript
GET https://api.github.com/repos/mbpz/proxybot/releases/latest
Response: { tag_name: "v1.3.0", body: "...", html_url: "..." }
```

### 版本比较
- 去掉 "v" 前缀
- 使用 semver 规则比较 (major.minor.patch)

---

## 3. 更新提示 UI - 设置页红点

### 布局位置
设置页面顶部，显示 "检查更新" 行

### 状态展示

| 状态 | 显示内容 |
|------|----------|
| 无更新 | "检查更新" + "当前版本 vX.X.X" |
| 有更新 | "检查更新" + 红色 "NEW" 标签 + "vX.X.X 可用" |
| 检查中 | "检查更新" + 加载动画 |
| 检查失败 | "检查更新" + 灰色 "检查失败" |

### 点击行为
有更新时点击，跳转到 release 页面:
```
https://github.com/mbpz/proxybot/releases/latest
```

或显示复制命令:
```
brew upgrade --cask mbpz/proxybot/proxybot
```

---

## 4. 实现文件

### 前端 (React)
- `src/components/settings/UpdateSettings.tsx` - 更新设置组件
- `src/hooks/useUpdateCheck.ts` - 更新检查 hook
- `src/App.tsx` - 启动时调用检查

### 后端 (Tauri/Rust)
- 无需后端支持，纯前端 API 调用

---

## 5. 依赖

- `@tauri-apps/api/core` - invoke (如需 Rust 端协助)
- `semver` - 版本比较 (如需精确比较)
- shadcn/ui - UI 组件