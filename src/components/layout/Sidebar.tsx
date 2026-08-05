import { useState } from "react";
import { Link, useLocation } from "react-router-dom";
import {
  Menu,
  X,
  Radio,
  Shield,
  Smartphone,
  PlayCircle,
  Settings,
} from "lucide-react";

interface NavItem {
  path: string;
  label: string;
  icon: React.ReactNode;
  activePaths: readonly string[];
}

const navItems: NavItem[] = [
  {
    path: "/",
    label: "Capture",
    icon: <Radio size={20} />,
    activePaths: ["/", "/dns", "/alerts", "/graph", "/topology"],
  },
  {
    path: "/setup",
    label: "Setup",
    icon: <Smartphone size={20} />,
    activePaths: ["/setup", "/certs", "/devices"],
  },
  { path: "/rules", label: "Rules", icon: <Shield size={20} />, activePaths: ["/rules"] },
  {
    path: "/replay",
    label: "Replay",
    icon: <PlayCircle size={20} />,
    activePaths: ["/replay", "/composer"],
  },
];

export function Sidebar() {
  const [collapsed, setCollapsed] = useState(false);
  const location = useLocation();

  return (
    <aside
      className={`flex flex-col bg-surface-primary text-text-primary h-screen border-r border-border transition-all duration-200 ${
        collapsed ? "w-16" : "w-52"
      }`}
    >
      {/* Header */}
      <div className="flex items-center justify-between p-4 border-b border-border">
        {!collapsed && (
          <span className="font-bold text-accent-blue tracking-wider">
            PROXYBOT
          </span>
        )}
        <button
          onClick={() => setCollapsed(!collapsed)}
          aria-label={collapsed ? "Expand sidebar" : "Collapse sidebar"}
          className="p-1 hover:bg-surface-tertiary rounded text-text-secondary hover:text-text-primary transition-colors"
        >
          {collapsed ? <Menu size={20} /> : <X size={20} />}
        </button>
      </div>

      {/* Nav Items */}
      <nav className="flex-1 py-4">
        {navItems.map((item) => {
          const isActive = item.activePaths.includes(location.pathname);
          return (
            <Link
              key={item.path}
              to={item.path}
              title={collapsed ? item.label : undefined}
              aria-current={isActive ? "page" : undefined}
              className={`flex items-center gap-3 mx-2 px-4 py-2.5 rounded-lg transition-all duration-200 ${
                isActive
                  ? "bg-[rgba(0,212,255,0.08)] text-accent-blue border-l-2 border-accent-blue"
                  : "border-l-2 border-transparent text-text-secondary hover:bg-surface-secondary hover:text-text-primary"
              }`}
            >
              <span className={isActive ? "text-accent-blue" : ""}>{item.icon}</span>
              {!collapsed && <span>{item.label}</span>}
            </Link>
          );
        })}
      </nav>

      {/* Footer */}
      <div className="p-3 border-t border-border">
        <Link
          to="/settings"
          title={collapsed ? "Settings" : undefined}
          aria-current={location.pathname === "/settings" ? "page" : undefined}
          className={`flex items-center gap-3 px-4 py-2.5 rounded-lg transition-all duration-200 ${
            location.pathname === "/settings"
              ? "bg-[rgba(0,212,255,0.08)] text-accent-blue border-l-2 border-accent-blue"
              : "border-l-2 border-transparent text-text-secondary hover:bg-surface-secondary hover:text-text-primary"
          }`}
        >
          <span className={location.pathname === "/settings" ? "text-accent-blue" : ""}>
            <Settings size={20} />
          </span>
          {!collapsed && <span>Settings</span>}
        </Link>
      </div>
    </aside>
  );
}
