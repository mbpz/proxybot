# Architect Brief — ProxyBot

## Step 13 — 导出 JSON ✅ (完成)

---

## Step 14 — 深色模式切换

目标：支持手动切换 dark/light 模式，不跟随系统。

### UI 实现

**1. 主题状态**

```tsx
const [theme, setTheme] = useState<'dark' | 'light'>('dark');
```

**2. 切换按钮**

放在 Setup 面板顶部，或者窗口右上角：

```tsx
<button
  className="theme-toggle"
  onClick={() => setTheme(t => t === 'dark' ? 'light' : 'dark')}
>
  {theme === 'dark' ? '☀️ Light' : '🌙 Dark'}
</button>
```

**3. 应用主题**

给 `<html>` 或 `<body>` 加 class：

```tsx
useEffect(() => {
  document.documentElement.classList.remove('dark', 'light');
  document.documentElement.classList.add(theme);
}, [theme]);
```

**4. CSS 变量**

已有 dark/light 覆盖，现有 CSS 不用改，只加：

```css
:root, :root.light {
  --bg-primary: #ffffff;
  --text-primary: #1a1a1a;
  /* ... */
}

:root.dark {
  --bg-primary: #1a1a2e;
  --text-primary: #e0e0e0;
  /* ... */
}
```

### 验收标准

1. 点「☀️ Light」→ 整个界面变为浅色主题
2. 点「🌙 Dark」→ 变回深色主题
3. 切换后主题保持，不重置
