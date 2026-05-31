import { useState } from "react";
import { Link, useLocation } from "react-router-dom";
import {
  Menu,
  X,
  List,
  Shield,
  Key,
  Smartphone,
  Globe,
  AlertTriangle,
  PlayCircle,
  GitBranch,
  Wand2,
  Send,
  Settings,
} from "lucide-react";

interface NavItem {
  path: string;
  label: string;
  icon: React.ReactNode;
}

const navItems: NavItem[] = [
  { path: "/", label: "Traffic", icon: <List size={20} /> },
  { path: "/rules", label: "Rules", icon: <Shield size={20} /> },
  { path: "/certs", label: "Certs", icon: <Key size={20} /> },
  { path: "/devices", label: "Devices", icon: <Smartphone size={20} /> },
  { path: "/dns", label: "DNS", icon: <Globe size={20} /> },
  { path: "/alerts", label: "Alerts", icon: <AlertTriangle size={20} /> },
  { path: "/replay", label: "Replay", icon: <PlayCircle size={20} /> },
  { path: "/graph", label: "Graph", icon: <GitBranch size={20} /> },
  { path: "/composer", label: "Composer", icon: <Send size={20} /> },
  { path: "/gen", label: "Gen", icon: <Wand2 size={20} /> },
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
          className="p-1 hover:bg-surface-tertiary rounded text-text-secondary hover:text-text-primary transition-colors"
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
              title={collapsed ? item.label : undefined}
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
