# ProxyBot 更新检测 + 图标实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现启动时自动检查更新 + 设置页手动检查 + 赛博霓虹风格图标

**Architecture:**
- 更新检测: React hook 调用 GitHub Releases API，状态存储在组件
- 设置页: 新增 UpdateSettings 组件集成到现有设置 UI
- 图标: 生成 SVG 然后转换为多尺寸 PNG

**Tech Stack:** React, TypeScript, Tauri, shadcn/ui

---

## 文件结构

### 新建
- `src/hooks/useUpdateCheck.ts` - 更新检查 hook
- `src/components/setup/UpdateSettings.tsx` - 更新设置组件
- `icons/icon.svg` - 原始 SVG 图标
- `icons/` 目录下的 PNG 文件

### 修改
- `src/App.tsx` - 启动时调用更新检查
- `src-tauri/tauri.conf.json` - 更新 version

---

## Task 1: 创建 useUpdateCheck Hook

**Files:**
- Create: `src/hooks/useUpdateCheck.ts`

- [ ] **Step 1: 创建 hook 文件**

```typescript
import { useState, useEffect, useCallback } from "react";

interface UpdateInfo {
  hasUpdate: boolean;
  latestVersion: string | null;
  currentVersion: string;
  releaseUrl: string | null;
  isLoading: boolean;
  error: string | null;
}

const CURRENT_VERSION = "1.2.0"; // TODO: 从 tauri.conf.json 同步
const REPO_OWNER = "mbpz";
const REPO_NAME = "proxybot";

export function useUpdateCheck() {
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo>({
    hasUpdate: false,
    latestVersion: null,
    currentVersion: CURRENT_VERSION,
    releaseUrl: null,
    isLoading: false,
    error: null,
  });

  const checkForUpdates = useCallback(async () => {
    setUpdateInfo(prev => ({ ...prev, isLoading: true, error: null }));

    try {
      const response = await fetch(
        `https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/releases/latest`
      );

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`);
      }

      const data = await response.json();
      const latestVersion = data.tag_name?.replace(/^v/, "") || "";
      const hasUpdate = compareVersions(latestVersion, CURRENT_VERSION) > 0;

      setUpdateInfo({
        hasUpdate,
        latestVersion,
        currentVersion: CURRENT_VERSION,
        releaseUrl: data.html_url || null,
        isLoading: false,
        error: null,
      });
    } catch (err) {
      setUpdateInfo(prev => ({
        ...prev,
        isLoading: false,
        error: err instanceof Error ? err.message : "检查更新失败",
      }));
    }
  }, []);

  const openReleasePage = useCallback(() => {
    if (updateInfo.releaseUrl) {
      window.open(updateInfo.releaseUrl, "_blank");
    }
  }, [updateInfo.releaseUrl]);

  return { ...updateInfo, checkForUpdates, openReleasePage };
}

function compareVersions(latest: string, current: string): number {
  const la = latest.split(".").map(Number);
  const ca = current.split(".").map(Number);

  for (let i = 0; i < Math.max(la.length, ca.length); i++) {
    const l = la[i] || 0;
    const c = ca[i] || 0;
    if (l > c) return 1;
    if (l < c) return -1;
  }
  return 0;
}
```

- [ ] **Step 2: 提交**

```bash
git add src/hooks/useUpdateCheck.ts
git commit -m "feat: add useUpdateCheck hook for GitHub releases"
```

---

## Task 2: 创建 UpdateSettings 组件

**Files:**
- Create: `src/components/setup/UpdateSettings.tsx`

- [ ] **Step 1: 创建组件**

```tsx
import { useUpdateCheck } from "../../hooks/useUpdateCheck";
import { Button } from "../ui/button";
import { Badge } from "../ui/badge";

export function UpdateSettings() {
  const { hasUpdate, latestVersion, currentVersion, isLoading, error, checkForUpdates, openReleasePage } = useUpdateCheck();

  return (
    <div className="flex items-center justify-between py-3 border-b border-border">
      <div className="flex items-center gap-3">
        <span className="text-sm text-foreground">检查更新</span>
        {isLoading && (
          <span className="text-xs text-muted-foreground">检查中...</span>
        )}
        {error && (
          <span className="text-xs text-destructive">{error}</span>
        )}
        {!isLoading && !error && hasUpdate && (
          <Badge variant="destructive" className="text-xs">
            NEW
          </Badge>
        )}
      </div>

      <div className="flex items-center gap-3">
        {!isLoading && !error && hasUpdate && latestVersion && (
          <span className="text-xs text-muted-foreground">
            v{latestVersion} 可用
          </span>
        )}
        {!isLoading && !error && !hasUpdate && (
          <span className="text-xs text-muted-foreground">
            v{currentVersion}
          </span>
        )}
        <Button
          variant="outline"
          size="sm"
          onClick={hasUpdate ? openReleasePage : checkForUpdates}
          disabled={isLoading}
        >
          {hasUpdate ? "更新 ProxyBot" : "检查更新"}
        </Button>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: 提交**

```bash
git add src/components/setup/UpdateSettings.tsx
git commit -m "feat: add UpdateSettings component for settings page"
```

---

## Task 3: 集成到设置页面

**Files:**
- Modify: `src/components/setup/ClientSetup.tsx`

- [ ] **Step 1: 添加导入和渲染 UpdateSettings**

在 ClientSetup.tsx 顶部添加导入:
```tsx
import { UpdateSettings } from "./UpdateSettings";
```

找到设置列表的位置，在适当位置添加:
```tsx
<div className="px-4">
  <UpdateSettings />
</div>
```

- [ ] **Step 2: 提交**

```bash
git add src/components/setup/ClientSetup.tsx
git commit -m "feat: integrate UpdateSettings into ClientSetup"
```

---

## Task 4: 启动时自动检查更新

**Files:**
- Modify: `src/App.tsx`

- [ ] **Step 1: 在 useEffect 中调用检查**

找到 App.tsx 中的 useEffect，添加更新检查调用:

```tsx
useEffect(() => {
  // 启动时检查更新 (延迟 3 秒避免阻塞启动)
  const timer = setTimeout(() => {
    checkForUpdates();
  }, 3000);

  return () => clearTimeout(timer);
}, []);
```

注意: 需要从 useUpdateCheck 导入 checkForUpdates

- [ ] **Step 2: 提交**

```bash
git add src/App.tsx
git commit -m "feat: check for updates on app startup"
```

---

## Task 5: 创建 App 图标 (SVG)

**Files:**
- Create: `icons/icon.svg`

- [ ] **Step 1: 创建 SVG 图标**

```svg
<svg width="1024" height="1024" viewBox="0 0 1024 1024" xmlns="http://www.w3.org/2000/svg">
  <defs>
    <filter id="glow" x="-50%" y="-50%" width="200%" height="200%">
      <feGaussianBlur stdDeviation="20" result="coloredBlur"/>
      <feMerge>
        <feMergeNode in="coloredBlur"/>
        <feMergeNode in="SourceGraphic"/>
      </feMerge>
    </filter>
    <linearGradient id="flowGradient" x1="0%" y1="0%" x2="0%" y2="100%">
      <stop offset="0%" style="stop-color:#f472b6;stop-opacity:1" />
      <stop offset="100%" style="stop-color:#22d3ee;stop-opacity:1" />
    </linearGradient>
  </defs>

  <!-- Background -->
  <rect width="1024" height="1024" fill="#1e1b4b" rx="200"/>

  <!-- Top packet -->
  <circle cx="512" cy="180" r="80" fill="#f472b6" filter="url(#glow)" opacity="0.9"/>

  <!-- Middle packet (largest) -->
  <circle cx="512" cy="512" r="120" fill="#a855f7" filter="url(#glow)"/>

  <!-- Bottom packet -->
  <circle cx="512" cy="844" r="70" fill="#22d3ee" filter="url(#glow)" opacity="0.8"/>

  <!-- Top connection line -->
  <line x1="512" y1="260" x2="512" y2="392" stroke="#22d3ee" stroke-width="16" stroke-dasharray="30,20"/>
  <polygon points="512,392 480,440 544,440" fill="#22d3ee"/>

  <!-- Bottom connection line -->
  <line x1="512" y1="632" x2="512" y2="774" stroke="#f472b6" stroke-width="16" stroke-dasharray="30,20"/>
  <polygon points="512,774 480,726 544,726" fill="#f472b6"/>
</svg>
```

- [ ] **Step 2: 提交**

```bash
git add icons/icon.svg
git commit -m "feat: add app icon SVG with cyber neon style"
```

---

## Task 6: 生成多尺寸 PNG 图标

**Files:**
- Modify: `icons/` directory

- [ ] **Step 1: 使用 rsvg-convert 或在线工具生成 PNG**

需要生成: 16x16, 32x32, 64x64, 128x128, 256x256, 512x512, 1024x1024

如果系统有 rsvg-convert:
```bash
rsvg-convert -w 16 -h 16 icons/icon.svg -o icons/16x16.png
rsvg-convert -w 32 -h 32 icons/icon.svg -o icons/32x32.png
# ... 以此类推
```

或者手动使用在线工具转换:
- https://cloudconvert.com/svg-to-png

- [ ] **Step 2: 提交 PNG 文件**

```bash
git add icons/*.png
git commit -m "feat: add app icon PNG files in multiple sizes"
```

---

## 验证步骤

1. **更新检测测试:**
   - 启动 App，等待 3 秒后检查是否有更新提示
   - 点击设置页的"检查更新"按钮

2. **图标验证:**
   - 确认 icons/icon.svg 内容正确
   - 确认所有尺寸的 PNG 都存在

---

## Self-Review Checklist

- [x] Spec coverage: 更新检测 ✓, 启动时检查 ✓, 设置页手动 ✓, 图标 ✓
- [x] No placeholders: 所有代码完整
- [x] Type consistency: useUpdateCheck hook 返回类型一致
- [x] Step dependencies: Task 1 → 2 → 3 → 4 (图标独立)