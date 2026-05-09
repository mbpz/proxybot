# Tauri GUI v0.8.0 Phase 1 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现Tauri GUI的Sidebar导航 + Rules编辑器 + Certs管理页面

**Architecture:** 使用React Router v6实现页面路由，Sidebar作为layout wrapper，Rules和Certs作为独立页面组件

**Tech Stack:** React 19, React Router v6, TypeScript, Tailwind CSS, Lucide React

---

## 文件变更概览

| 操作 | 文件 |
|------|------|
| Modify | `package.json` - 添加 react-router-dom |
| Create | `src/components/layout/Sidebar.tsx` |
| Create | `src/components/layout/Layout.tsx` |
| Create | `src/components/rules/RulesPage.tsx` |
| Create | `src/components/rules/RuleCard.tsx` |
| Create | `src/components/rules/RuleModal.tsx` |
| Create | `src/components/certs/CertsPage.tsx` |
| Modify | `src/main.tsx` - 添加Router |
| Modify | `src/index.css` - 样式 |

---

## Task 1: 添加 React Router 依赖

**Files:**
- Modify: `package.json`

- [ ] **Step 1: 添加 react-router-dom 依赖**

运行: `cd /Users/doug/ai/system/proxybot/src && npm install react-router-dom`

- [ ] **Step 2: 验证安装**

```bash
cat package.json | grep react-router
```

Expected: `"react-router-dom": "^6.x"`

- [ ] **Step 3: Commit**

```bash
git add package.json package-lock.json
git commit -m "feat(gui): add react-router-dom for routing"
```

---

## Task 2: 创建 Sidebar 组件

**Files:**
- Create: `src/components/layout/Sidebar.tsx`

- [ ] **Step 1: 创建 Sidebar 组件代码**

```tsx
import { useState } from "react";
import { Link, useLocation } from "react-router-dom";
import {
  Menu,
  X,
  List,
  Shield,
  Smartphone,
  Globe,
  AlertTriangle,
  PlayCircle,
  GitBranch,
  Wand2,
} from "lucide-react";

interface NavItem {
  path: string;
  label: string;
  icon: React.ReactNode;
}

const navItems: NavItem[] = [
  { path: "/", label: "Traffic", icon: <List size={20} /> },
  { path: "/rules", label: "Rules", icon: <Shield size={20} /> },
  { path: "/certs", label: "Certs", icon: <Shield size={20} /> },
  { path: "/devices", label: "Devices", icon: <Smartphone size={20} /> },
  { path: "/dns", label: "DNS", icon: <Globe size={20} /> },
  { path: "/alerts", label: "Alerts", icon: <AlertTriangle size={20} /> },
  { path: "/replay", label: "Replay", icon: <PlayCircle size={20} /> },
  { path: "/graph", label: "Graph", icon: <GitBranch size={20} /> },
  { path: "/gen", label: "Gen", icon: <Wand2 size={20} /> },
];

export function Sidebar() {
  const [collapsed, setCollapsed] = useState(false);
  const location = useLocation();

  return (
    <aside
      className={`flex flex-col bg-gray-900 text-white h-screen transition-all duration-200 ${
        collapsed ? "w-16" : "w-52"
      }`}
    >
      {/* Header */}
      <div className="flex items-center justify-between p-4 border-b border-gray-700">
        {!collapsed && <span className="font-bold">ProxyBot</span>}
        <button
          onClick={() => setCollapsed(!collapsed)}
          className="p-1 hover:bg-gray-700 rounded"
        >
          {collapsed ? <Menu size={20} /> : <X size={20} />}
        </button>
      </div>

      {/* Nav Items */}
      <nav className="flex-1 py-4">
        {navItems.map((item) => {
          const isActive = location.pathname === item.path;
          return (
            <Link
              key={item.path}
              to={item.path}
              className={`flex items-center gap-3 px-4 py-3 hover:bg-gray-800 transition-colors ${
                isActive ? "bg-gray-800 border-l-2 border-blue-500" : ""
              }`}
            >
              {item.icon}
              {!collapsed && <span>{item.label}</span>}
            </Link>
          );
        })}
      </nav>
    </aside>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add src/components/layout/Sidebar.tsx
git commit -m "feat(gui): add Sidebar component with collapsible navigation"
```

---

## Task 3: 创建 Layout wrapper

**Files:**
- Create: `src/components/layout/Layout.tsx`

- [ ] **Step 1: 创建 Layout 组件**

```tsx
import { Outlet } from "react-router-dom";
import { Sidebar } from "./Sidebar";

export function Layout() {
  return (
    <div className="flex h-screen">
      <Sidebar />
      <main className="flex-1 overflow-auto bg-gray-100">
        <Outlet />
      </main>
    </div>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add src/components/layout/Layout.tsx
git commit -m "feat(gui): add Layout wrapper component"
```

---

## Task 4: 配置 React Router

**Files:**
- Modify: `src/main.tsx`

- [ ] **Step 1: 更新 main.tsx 添加路由**

```tsx
import React from "react";
import ReactDOM from "react-dom/client";
import { BrowserRouter, Routes, Route } from "react-router-dom";
import { Layout } from "./components/layout/Layout";
import { TrafficPage } from "./components/traffic/TrafficPage";
import { RulesPage } from "./components/rules/RulesPage";
import { CertsPage } from "./components/certs/CertsPage";
import "./index.css";

// Placeholder pages for now - will implement in later tasks
function PlaceholderPage({ name }: { name: string }) {
  return (
    <div className="p-8">
      <h1 className="text-2xl font-bold">{name}</h1>
    </div>
  );
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <BrowserRouter>
      <Routes>
        <Route path="/" element={<Layout />}>
          <Route index element={<div className="p-8"><h1 className="text-2xl">Traffic Page - Coming Soon</h1></div>} />
          <Route path="rules" element={<RulesPage />} />
          <Route path="certs" element={<CertsPage />} />
          <Route path="devices" element={<PlaceholderPage name="Devices" />} />
          <Route path="dns" element={<PlaceholderPage name="DNS" />} />
          <Route path="alerts" element={<PlaceholderPage name="Alerts" />} />
          <Route path="replay" element={<PlaceholderPage name="Replay" />} />
          <Route path="graph" element={<PlaceholderPage name="Graph" />} />
          <Route path="gen" element={<PlaceholderPage name="Gen" />} />
        </Route>
      </Routes>
    </BrowserRouter>
  </React.StrictMode>
);
```

- [ ] **Step 2: 编译检查**

```bash
npm run build 2>&1 | head -30
```

Expected: TypeScript 错误（RulesPage, CertsPage 未定义）

- [ ] **Step 3: Commit**

```bash
git add src/main.tsx
git commit -m "feat(gui): configure React Router with Layout and routes"
```

---

## Task 5: 创建 RulesPage 组件

**Files:**
- Create: `src/components/rules/RulesPage.tsx`

- [ ] **Step 1: 创建 RulesPage 组件**

```tsx
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { RuleCard } from "./RuleCard";
import { RuleModal } from "./RuleModal";

type RulePattern = "DOMAIN" | "DOMAIN-SUFFIX" | "DOMAIN-KEYWORD" | "IP-CIDR" | "GEOIP" | "RULE-SET";
type RuleAction = "DIRECT" | "PROXY" | "REJECT";

interface Rule {
  pattern: RulePattern;
  value: string;
  action: RuleAction;
  name: string;
  priority: number;
  enabled: boolean;
  comment: string;
}

export function RulesPage() {
  const [rules, setRules] = useState<Rule[]>([]);
  const [modalOpen, setModalOpen] = useState(false);
  const [editingRule, setEditingRule] = useState<Rule | null>(null);

  useEffect(() => {
    loadRules();
  }, []);

  async function loadRules() {
    try {
      const result = await invoke<Rule[]>("get_rules");
      setRules(result);
    } catch (err) {
      console.error("Failed to load rules:", err);
    }
  }

  function handleAddRule() {
    setEditingRule(null);
    setModalOpen(true);
  }

  function handleEditRule(rule: Rule) {
    setEditingRule(rule);
    setModalOpen(true);
  }

  async function handleSaveRule(rule: Rule) {
    try {
      await invoke("save_rule", { rule, filename: "custom.yaml" });
      setModalOpen(false);
      loadRules();
    } catch (err) {
      console.error("Failed to save rule:", err);
    }
  }

  async function handleDeleteRule(rule: Rule) {
    try {
      await invoke("delete_rule", { rule, filename: "custom.yaml" });
      loadRules();
    } catch (err) {
      console.error("Failed to delete rule:", err);
    }
  }

  return (
    <div className="p-6">
      <div className="flex justify-between items-center mb-6">
        <h1 className="text-2xl font-bold">Rules</h1>
        <button
          onClick={handleAddRule}
          className="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700"
        >
          Add Rule
        </button>
      </div>

      <div className="grid gap-4 grid-cols-1 md:grid-cols-2 lg:grid-cols-3">
        {rules.map((rule, index) => (
          <RuleCard
            key={index}
            rule={rule}
            onEdit={() => handleEditRule(rule)}
            onDelete={() => handleDeleteRule(rule)}
          />
        ))}
      </div>

      {rules.length === 0 && (
        <p className="text-gray-500 text-center py-8">No rules configured yet.</p>
      )}

      {modalOpen && (
        <RuleModal
          rule={editingRule}
          onSave={handleSaveRule}
          onClose={() => setModalOpen(false)}
        />
      )}
    </div>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add src/components/rules/RulesPage.tsx
git commit -m "feat(gui): add RulesPage component with rule list"
```

---

## Task 6: 创建 RuleCard 组件

**Files:**
- Create: `src/components/rules/RuleCard.tsx`

- [ ] **Step 1: 创建 RuleCard 组件**

```tsx
import { Pencil, Trash2 } from "lucide-react";

interface Rule {
  pattern: string;
  value: string;
  action: string;
  name: string;
  priority: number;
  enabled: boolean;
  comment: string;
}

interface RuleCardProps {
  rule: Rule;
  onEdit: () => void;
  onDelete: () => void;
}

export function RuleCard({ rule, onEdit, onDelete }: RuleCardProps) {
  const actionColors: Record<string, string> = {
    DIRECT: "bg-green-100 text-green-800",
    PROXY: "bg-blue-100 text-blue-800",
    REJECT: "bg-red-100 text-red-800",
    MAPREMOTE: "bg-purple-100 text-purple-800",
    MAPLOCAL: "bg-orange-100 text-orange-800",
  };

  return (
    <div className={`bg-white rounded-lg shadow p-4 ${!rule.enabled ? "opacity-50" : ""}`}>
      <div className="flex justify-between items-start mb-2">
        <h3 className="font-semibold text-gray-900">{rule.name || "Unnamed Rule"}</h3>
        <span className={`px-2 py-1 rounded text-xs font-medium ${actionColors[rule.action] || "bg-gray-100"}`}>
          {rule.action}
        </span>
      </div>

      <div className="space-y-1 text-sm text-gray-600 mb-3">
        <p>
          <span className="font-medium">{rule.pattern}:</span> {rule.value}
        </p>
        {rule.comment && <p className="text-gray-500">{rule.comment}</p>}
      </div>

      <div className="flex justify-end gap-2">
        <button
          onClick={onEdit}
          className="p-2 text-gray-600 hover:text-blue-600 hover:bg-gray-100 rounded"
        >
          <Pencil size={16} />
        </button>
        <button
          onClick={onDelete}
          className="p-2 text-gray-600 hover:text-red-600 hover:bg-gray-100 rounded"
        >
          <Trash2 size={16} />
        </button>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add src/components/rules/RuleCard.tsx
git commit -m "feat(gui): add RuleCard component"
```

---

## Task 7: 创建 RuleModal 组件

**Files:**
- Create: `src/components/rules/RuleModal.tsx`

- [ ] **Step 1: 创建 RuleModal 组件**

```tsx
import { useState } from "react";

interface Rule {
  pattern: string;
  value: string;
  action: string;
  name: string;
  priority: number;
  enabled: boolean;
  comment: string;
}

interface RuleModalProps {
  rule: Rule | null;
  onSave: (rule: Rule) => void;
  onClose: () => void;
}

const patterns = ["DOMAIN", "DOMAIN-SUFFIX", "DOMAIN-KEYWORD", "IP-CIDR", "GEOIP", "RULE-SET"];
const actions = ["DIRECT", "PROXY", "REJECT", "MAPREMOTE", "MAPLOCAL", "BREAKPOINT"];

export function RuleModal({ rule, onSave, onClose }: RuleModalProps) {
  const [formData, setFormData] = useState<Rule>({
    pattern: rule?.pattern || "DOMAIN-SUFFIX",
    value: rule?.value || "",
    action: rule?.action || "DIRECT",
    name: rule?.name || "",
    priority: rule?.priority || 100,
    enabled: rule?.enabled ?? true,
    comment: rule?.comment || "",
  });

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    onSave(formData);
  }

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
      <div className="bg-white rounded-lg shadow-xl w-full max-w-md p-6">
        <h2 className="text-xl font-bold mb-4">{rule ? "Edit Rule" : "Add Rule"}</h2>

        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">Name</label>
            <input
              type="text"
              value={formData.name}
              onChange={(e) => setFormData({ ...formData, name: e.target.value })}
              className="w-full px-3 py-2 border rounded focus:outline-none focus:ring-2 focus:ring-blue-500"
              placeholder="My Rule"
            />
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">Pattern</label>
            <select
              value={formData.pattern}
              onChange={(e) => setFormData({ ...formData, pattern: e.target.value })}
              className="w-full px-3 py-2 border rounded focus:outline-none focus:ring-2 focus:ring-blue-500"
            >
              {patterns.map((p) => (
                <option key={p} value={p}>{p}</option>
              ))}
            </select>
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">Value</label>
            <input
              type="text"
              value={formData.value}
              onChange={(e) => setFormData({ ...formData, value: e.target.value })}
              className="w-full px-3 py-2 border rounded focus:outline-none focus:ring-2 focus:ring-blue-500"
              placeholder="example.com"
              required
            />
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">Action</label>
            <select
              value={formData.action}
              onChange={(e) => setFormData({ ...formData, action: e.target.value })}
              className="w-full px-3 py-2 border rounded focus:outline-none focus:ring-2 focus:ring-blue-500"
            >
              {actions.map((a) => (
                <option key={a} value={a}>{a}</option>
              ))}
            </select>
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">Priority</label>
            <input
              type="number"
              value={formData.priority}
              onChange={(e) => setFormData({ ...formData, priority: parseInt(e.target.value) || 100 })}
              className="w-full px-3 py-2 border rounded focus:outline-none focus:ring-2 focus:ring-blue-500"
              min="1"
              max="255"
            />
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">Comment</label>
            <input
              type="text"
              value={formData.comment}
              onChange={(e) => setFormData({ ...formData, comment: e.target.value })}
              className="w-full px-3 py-2 border rounded focus:outline-none focus:ring-2 focus:ring-blue-500"
              placeholder="Optional comment"
            />
          </div>

          <div className="flex justify-end gap-3 pt-4">
            <button
              type="button"
              onClick={onClose}
              className="px-4 py-2 text-gray-700 hover:bg-gray-100 rounded"
            >
              Cancel
            </button>
            <button
              type="submit"
              className="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700"
            >
              Save
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add src/components/rules/RuleModal.tsx
git commit -m "feat(gui): add RuleModal component for rule editing"
```

---

## Task 8: 创建 CertsPage 组件

**Files:**
- Create: `src/components/certs/CertsPage.tsx`

- [ ] **Step 1: 创建 CertsPage 组件**

```tsx
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Download, RefreshCw } from "lucide-react";

interface CaMetadata {
  created_at: number;
  serial: string;
  fingerprint?: string;
}

export function CertsPage() {
  const [caMetadata, setCaMetadata] = useState<CaMetadata | null>(null);
  const [loading, setLoading] = useState(true);
  const [exporting, setExporting] = useState(false);
  const [regenerating, setRegenerating] = useState(false);

  useEffect(() => {
    loadCaMetadata();
  }, []);

  async function loadCaMetadata() {
    try {
      const result = await invoke<CaMetadata | null>("get_ca_metadata");
      setCaMetadata(result);
    } catch (err) {
      console.error("Failed to load CA metadata:", err);
    } finally {
      setLoading(false);
    }
  }

  async function handleExport() {
    setExporting(true);
    try {
      const path = await invoke<string>("export_cert");
      alert(`CA exported to: ${path}`);
    } catch (err) {
      console.error("Failed to export CA:", err);
      alert("Failed to export CA");
    } finally {
      setExporting(false);
    }
  }

  async function handleRegenerate() {
    if (!confirm("This will regenerate the CA. Existing certificates will be invalidated. Continue?")) {
      return;
    }
    setRegenerating(true);
    try {
      await invoke("regenerate_ca");
      alert("CA regenerated successfully");
      loadCaMetadata();
    } catch (err) {
      console.error("Failed to regenerate CA:", err);
      alert("Failed to regenerate CA");
    } finally {
      setRegenerating(false);
    }
  }

  function formatDate(timestamp: number): string {
    return new Date(timestamp * 1000).toLocaleString();
  }

  if (loading) {
    return <div className="p-6">Loading...</div>;
  }

  return (
    <div className="p-6">
      <h1 className="text-2xl font-bold mb-6">Certificates</h1>

      <div className="bg-white rounded-lg shadow p-6 max-w-2xl">
        <h2 className="text-lg font-semibold mb-4">Root CA Certificate</h2>

        {caMetadata ? (
          <div className="space-y-3">
            <div className="flex justify-between">
              <span className="text-gray-600">Created:</span>
              <span className="font-medium">{formatDate(caMetadata.created_at)}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-600">Serial:</span>
              <span className="font-mono text-sm">{caMetadata.serial}</span>
            </div>
            {caMetadata.fingerprint && (
              <div className="flex justify-between">
                <span className="text-gray-600">Fingerprint:</span>
                <span className="font-mono text-xs">{caMetadata.fingerprint}</span>
              </div>
            )}
          </div>
        ) : (
          <p className="text-gray-500">No CA certificate found. Generate one to get started.</p>
        )}

        <div className="flex gap-3 mt-6">
          <button
            onClick={handleExport}
            disabled={exporting || !caMetadata}
            className="flex items-center gap-2 px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 disabled:opacity-50"
          >
            <Download size={16} />
            {exporting ? "Exporting..." : "Export CA"}
          </button>
          <button
            onClick={handleRegenerate}
            disabled={regenerating}
            className="flex items-center gap-2 px-4 py-2 bg-orange-600 text-white rounded hover:bg-orange-700 disabled:opacity-50"
          >
            <RefreshCw size={16} />
            {regenerating ? "Regenerating..." : "Regenerate CA"}
          </button>
        </div>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add src/components/certs/CertsPage.tsx
git commit -m "feat(gui): add CertsPage component"
```

---

## Task 9: 编译验证

**Files:**
- Modify: `src/components/rules/RulesPage.tsx` (if needed)

- [ ] **Step 1: 运行编译**

```bash
npm run build 2>&1 | head -50
```

Expected: 无TypeScript错误

- [ ] **Step 2: 检查Tauri构建**

```bash
cargo build --bin proxybot-gui 2>&1 | tail -20
```

Expected: 编译成功

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat(gui): complete Phase 1 - Sidebar, Rules, Certs pages"
```

---

## 验证清单

- [ ] Sidebar 可折叠/展开
- [ ] 路由切换正常（/rules, /certs 等）
- [ ] Rules 列表显示
- [ ] 添加/编辑/删除规则
- [ ] Certs 显示 CA 信息
- [ ] 导出 CA 功能
- [ ] 重新生成 CA 功能
