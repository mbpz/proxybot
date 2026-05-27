import { Button } from "../ui/Button";

type TabId = "traffic" | "dns" | "rules" | "devices" | "replay" | "ai";

interface AppHeaderProps {
  running: boolean;
  activeTab: TabId;
  alertCount: number;
  caDate?: string;
  onStartProxy: () => void;
  onDownloadCa: () => void;
  onTabChange: (tab: TabId) => void;
}

const tabs: { id: TabId; label: string }[] = [
  { id: "traffic", label: "Traffic" },
  { id: "dns", label: "DNS" },
  { id: "rules", label: "Rules" },
  { id: "devices", label: "Devices" },
  { id: "replay", label: "Replay" },
  { id: "ai", label: "AI" },
];

export function AppHeader({
  running,
  activeTab,
  alertCount,
  caDate,
  onStartProxy,
  onDownloadCa,
  onTabChange,
}: AppHeaderProps) {
  return (
    <div
      style={{
        background: "var(--bg-secondary)",
        borderBottom: "1px solid var(--border)",
      }}
    >
      {/* Top bar */}
      <div
        className="flex items-center justify-between px-4 py-2"
        style={{ borderBottom: "1px solid var(--border)" }}
      >
        {/* Left: Logo */}
        <div className="flex items-center gap-3">
          <span
            className="font-bold text-lg"
            style={{ color: "var(--accent-blue)" }}
          >
            ProxyBot
          </span>
          <span
            className="text-xs font-mono"
            style={{
              background: running
                ? "rgba(62, 207, 142, 0.15)"
                : "rgba(96, 96, 96, 0.15)",
              color: running ? "var(--accent-green)" : "var(--text-muted)",
              padding: "2px 8px",
              borderRadius: "var(--radius-sm)",
            }}
          >
            {running ? "Running :8080" : "Stopped"}
          </span>
        </div>

        {/* Right: Actions */}
        <div className="flex items-center gap-2">
          {caDate && (
            <span className="text-xs text-muted font-mono">{caDate}</span>
          )}
          <Button variant="secondary" size="sm" onClick={onDownloadCa}>
            CA Cert
          </Button>
          <Button
            variant={running ? "danger" : "primary"}
            size="sm"
            onClick={onStartProxy}
            disabled={running}
          >
            {running ? "Running" : "Start Proxy"}
          </Button>
        </div>
      </div>

      {/* Tab bar */}
      <div className="flex px-4">
        {tabs.map((tab) => (
          <button
            key={tab.id}
            className={`tab ${activeTab === tab.id ? "active" : ""}`}
            onClick={() => onTabChange(tab.id)}
          >
            {tab.label}
            {tab.id === "ai" && alertCount > 0 && (
              <span
                style={{
                  background: "var(--accent-yellow)",
                  color: "#000",
                  padding: "1px 6px",
                  borderRadius: "10px",
                  fontSize: "10px",
                  fontWeight: 700,
                  marginLeft: 6,
                }}
              >
                {alertCount}
              </span>
            )}
          </button>
        ))}
      </div>
    </div>
  );
}
