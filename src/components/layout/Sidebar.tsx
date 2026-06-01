import { Link, useLocation } from "react-router-dom";
import {
  Activity,
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

const monitorItems: NavItem[] = [
  { path: "/", label: "Traffic", icon: <Activity size={18} /> },
  { path: "/rules", label: "Rules", icon: <Shield size={18} /> },
];

const toolsItems: NavItem[] = [
  { path: "/certs", label: "Certs", icon: <Key size={18} /> },
  { path: "/devices", label: "Devices", icon: <Smartphone size={18} /> },
  { path: "/dns", label: "DNS", icon: <Globe size={18} /> },
  { path: "/alerts", label: "Alerts", icon: <AlertTriangle size={18} /> },
  { path: "/replay", label: "Replay", icon: <PlayCircle size={18} /> },
  { path: "/graph", label: "Graph", icon: <GitBranch size={18} /> },
  { path: "/composer", label: "Composer", icon: <Send size={18} /> },
  { path: "/gen", label: "Gen", icon: <Wand2 size={18} /> },
];

export function Sidebar() {
  const location = useLocation();

  return (
    <aside
      className="flex flex-col"
      style={{
        width: 200,
        height: "100vh",
        backgroundColor: "var(--bg-secondary)",
        color: "var(--text-primary)",
        borderRight: "1px solid var(--border)",
      }}
    >
      {/* Header */}
      <div
        className="flex items-center gap-3"
        style={{ height: 56, padding: "20px 16px" }}
      >
        <div
          className="rounded"
          style={{
            width: 28,
            height: 28,
            borderRadius: 6,
            backgroundColor: "var(--accent-blue)",
          }}
        />
        <span
          className="font-bold"
          style={{
            fontSize: 16,
            fontWeight: 700,
            color: "var(--accent-blue)",
            letterSpacing: 3,
          }}
        >
          PROXYBOT
        </span>
      </div>

      {/* Divider */}
      <div className="border-t" style={{ borderColor: "var(--border)" }} />

      {/* Nav Section - MONITOR */}
      <nav className="flex-1 py-4">
        <div className="px-4 pb-2" style={{ paddingBottom: 6 }}>
          <span
            style={{
              fontSize: 10,
              fontWeight: 600,
              color: "var(--text-muted)",
              letterSpacing: 2,
            }}
          >
            MONITOR
          </span>
        </div>
        {monitorItems.map((item) => {
          const isActive = location.pathname === item.path;
          return (
            <Link
              key={item.path}
              to={item.path}
              className="flex items-center rounded transition-all duration-200"
              style={{
                padding: "10px 16px",
                gap: 12,
                marginLeft: 16,
                marginRight: 16,
                backgroundColor: isActive ? "rgba(0, 212, 255, 0.08)" : "transparent",
                borderLeft: isActive ? "2px solid var(--accent-blue)" : "2px solid transparent",
                color: isActive ? "var(--accent-blue)" : "var(--text-secondary)",
              }}
            >
              <span style={{ color: isActive ? "var(--accent-blue)" : "var(--text-secondary)" }}>
                {item.icon}
              </span>
              <span>{item.label}</span>
            </Link>
          );
        })}

        {/* Divider */}
        <div className="border-t my-3" style={{ borderColor: "var(--border)" }} />

        <div className="px-4 pb-2" style={{ paddingBottom: 6 }}>
          <span
            style={{
              fontSize: 10,
              fontWeight: 600,
              color: "var(--text-muted)",
              letterSpacing: 2,
            }}
          >
            TOOLS
          </span>
        </div>
        {toolsItems.map((item) => {
          const isActive = location.pathname === item.path;
          return (
            <Link
              key={item.path}
              to={item.path}
              className="flex items-center rounded transition-all duration-200"
              style={{
                padding: "10px 16px",
                gap: 12,
                marginLeft: 16,
                marginRight: 16,
                backgroundColor: isActive ? "rgba(0, 212, 255, 0.08)" : "transparent",
                borderLeft: isActive ? "2px solid var(--accent-blue)" : "2px solid transparent",
                color: isActive ? "var(--accent-blue)" : "var(--text-secondary)",
              }}
            >
              <span style={{ color: isActive ? "var(--accent-blue)" : "var(--text-secondary)" }}>
                {item.icon}
              </span>
              <span>{item.label}</span>
            </Link>
          );
        })}
      </nav>

      {/* Footer - Settings */}
      <div className="border-t" style={{ borderColor: "var(--border)" }}>
        <Link
          to="/settings"
          className="flex items-center rounded transition-all duration-200"
          style={{
            padding: "10px 16px",
            gap: 12,
            marginLeft: 16,
            marginRight: 16,
            marginTop: 12,
            marginBottom: 12,
            backgroundColor:
              location.pathname === "/settings"
                ? "rgba(0, 212, 255, 0.08)"
                : "transparent",
            borderLeft:
              location.pathname === "/settings"
                ? "2px solid var(--accent-blue)"
                : "2px solid transparent",
            color: location.pathname === "/settings" ? "var(--accent-blue)" : "var(--text-secondary)",
          }}
        >
          <span
            style={{
              color: location.pathname === "/settings" ? "var(--accent-blue)" : "var(--text-secondary)",
            }}
          >
            <Settings size={18} />
          </span>
          <span>Settings</span>
        </Link>
      </div>
    </aside>
  );
}