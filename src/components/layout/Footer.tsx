import type { NetworkInfo } from "../../types";

interface FooterProps {
  networkInfo: NetworkInfo | null;
  pfEnabled: boolean;
  pfLoading: boolean;
  tunEnabled: boolean;
  tunLoading: boolean;
  dashboardRunning: boolean;
  dashboardUrl: string;
  onEnablePf: () => void;
  onDisablePf: () => void;
  onEnableTun: () => void;
  onDisableTun: () => void;
  onToggleDashboard: () => void;
}

export function Footer({
  networkInfo, pfEnabled, pfLoading, tunEnabled, tunLoading,
  dashboardRunning, dashboardUrl,
  onEnablePf, onDisablePf, onEnableTun, onDisableTun, onToggleDashboard,
}: FooterProps) {
  return (
    <div style={{
      position: "fixed", bottom: 0, left: 0, right: 0,
      padding: "var(--space-3) var(--space-4)",
      background: "var(--bg-secondary)", borderTop: "1px solid var(--border)",
      display: "flex", alignItems: "center", justifyContent: "space-between",
      fontSize: "var(--text-sm)", zIndex: 50,
    }}>
      <div style={{ display: "flex", gap: "var(--space-6)", alignItems: "center" }}>
        <div>
          <span className="text-muted">LAN IP: </span>
          <span className="font-mono">{networkInfo?.lan_ip || "—"}</span>
        </div>
        <div>
          <span className="text-muted">pf: </span>
          <span style={{ color: pfEnabled ? "var(--accent-green)" : "var(--text-muted)" }}>
            {pfEnabled ? "Enabled" : "Disabled"}
          </span>
        </div>
        <div>
          <span className="text-muted">TUN: </span>
          <span style={{ color: tunEnabled ? "var(--accent-green)" : "var(--text-muted)" }}>
            {tunEnabled ? "Enabled" : "Disabled"}
          </span>
        </div>
        {dashboardRunning && (
          <div>
            <span className="text-muted">Dashboard: </span>
            <span className="font-mono" style={{ color: "var(--accent-green)", fontSize: "var(--text-xs)" }}>
              {dashboardUrl}
            </span>
          </div>
        )}
      </div>
      <div style={{ display: "flex", gap: "var(--space-2)" }}>
        {!pfEnabled ? (
          <button className="btn btn-sm btn-secondary" onClick={onEnablePf} disabled={pfLoading || !networkInfo}>
            {pfLoading ? "..." : "Enable pf"}
          </button>
        ) : (
          <button className="btn btn-sm btn-secondary" onClick={onDisablePf} disabled={pfLoading}>
            {pfLoading ? "..." : "Disable pf"}
          </button>
        )}
        {!tunEnabled ? (
          <button className="btn btn-sm btn-secondary" onClick={onEnableTun} disabled={tunLoading}>
            {tunLoading ? "..." : "TUN Mode"}
          </button>
        ) : (
          <button className="btn btn-sm btn-secondary" onClick={onDisableTun} disabled={tunLoading}>
            {tunLoading ? "..." : "Disable TUN"}
          </button>
        )}
        <button
          className={`btn btn-sm ${dashboardRunning ? "btn-danger" : "btn-primary"}`}
          onClick={onToggleDashboard}
          title={dashboardRunning ? "Stop mobile dashboard" : "Start mobile dashboard for phone access"}
        >
          {dashboardRunning ? "Stop Dashboard" : "📱 Mobile Dashboard"}
        </button>
      </div>
    </div>
  );
}
