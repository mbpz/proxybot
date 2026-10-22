# Tauri GUI v0.8.0 设计方案

## Status: Approved

## 1. Overview

为 ProxyBot Tauri GUI 实现完整的 9-tab 界面，复用 Rust 核心逻辑，与 TUI 功能对齐。

**Phase 1 范围**: Rules编辑器 + Certs管理

## 2. 架构

### 2.1 路由结构

使用 React Router 实现页面路由：

| 路径 | 页面 | 组件 |
|------|------|------|
| `/` | Traffic | TrafficPage |
| `/rules` | Rules | RulesPage |
| `/certs` | Certs | CertsPage |
| `/devices` | Devices | DevicesPage |
| `/dns` | DNS | DnsPage |
| `/alerts` | Alerts | AlertsPage |
| `/replay` | Replay | ReplayPage |
| `/graph` | Graph | GraphPage |
| `/gen` | Gen | GenPage |

### 2.2 组件结构

```
src/
├── components/
│   ├── layout/
│   │   ├── Sidebar.tsx      # 可折叠侧边栏
│   │   └── Layout.tsx       # 路由layout wrapper
│   ├── rules/
│   │   ├── RulesPage.tsx    # 规则列表页
│   │   ├── RuleCard.tsx     # 规则卡片
│   │   └── RuleModal.tsx    # 规则编辑modal
│   ├── certs/
│   │   └── CertsPage.tsx   # CA证书信息页
│   └── traffic/             # 现有，保持不变
│       └── ...
```

### 2.3 IPC 通信

复用现有 Rust Tauri commands：
- `get_rules()` - 获取规则列表
- `save_rule()` - 保存规则
- `delete_rule()` - 删除规则
- `list_rule_files()` - 列出规则文件
- `get_ca_metadata()` - 获取CA元数据
- `export_cert()` - 导出CA
- `regenerate_ca()` - 重新生成CA

## 3. Sidebar 组件

### 3.1 布局

- 宽度：展开时 200px，收起时 60px
- 顶部：Logo + 应用名称
- 中部：导航菜单项（图标 + 文字）
- 底部：折叠/展开按钮

### 3.2 状态

```typescript
interface SidebarState {
  collapsed: boolean;  // 默认 false
}
```

### 3.3 样式

- 深色主题背景
- 选中项高亮（背景色 + 左边框）
- 图标使用 Lucide React

## 4. Rules 页面

### 4.1 规则卡片

```typescript
interface Rule {
  pattern: RulePattern;  // DOMAIN | DOMAIN-SUFFIX | DOMAIN-KEYWORD | IP-CIDR | GEOIP | RULE-SET
  value: string;
  action: RuleAction;    // DIRECT | PROXY | REJECT | MAPREMOTE | MAPLOCAL | BREAKPOINT
  name: string;
  priority: number;
  enabled: boolean;
  comment: string;
}
```

### 4.2 卡片显示

- 顶部：name + enabled toggle
- 主体：pattern badge + value
- 底部：action badge + 编辑/删除按钮

### 4.3 RuleModal 编辑

**字段**：
- Pattern: 下拉选择
- Value: 文本输入
- Action: 下拉选择
- Name: 文本输入
- Priority: 数字输入
- Comment: 文本输入

**按钮**：保存 / 取消

## 5. Certs 页面

### 5.1 CA信息卡片

```typescript
interface CaMetadata {
  created_at: number;   // timestamp
  serial: string;
  fingerprint?: string;
  expires_at?: number;
}
```

### 5.2 显示内容

- 创建时间（格式化日期）
- 序列号
- 指纹（SHA256）
- 过期时间（如果适用）

### 5.3 操作按钮

- **导出CA** - 调用 `export_cert()`
- **重新生成CA** - 调用 `regenerate_ca()`

## 6. 技术栈

- React 18+
- React Router v6
- TypeScript
- Tailwind CSS
- Lucide React (图标)

## 7. 实施步骤

### Phase 1: Rules + Certs
1. 实现 Layout + Sidebar 组件
2. 配置 React Router
3. 实现 RulesPage + RuleCard + RuleModal
4. 实现 CertsPage
5. 测试验证

### Phase 2-4: 其他Tabs
- Devices, DNS, Alerts, Replay, Graph, Gen
- 复用 Phase 1 的架构模式

## 8. 验证

```bash
cd src-tauri
cargo build --bin proxybot-gui --release
# 启动应用，测试各tab切换
# 验证Rules添加/编辑/删除
# 验证Certs导出功能
```
