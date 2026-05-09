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
