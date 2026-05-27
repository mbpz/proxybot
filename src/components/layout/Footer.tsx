import type { NetworkInfo } from "../../types";

interface FooterProps {
  networkInfo: NetworkInfo | null;
  pfEnabled: boolean;
  pfLoading: boolean;
  tunEnabled: boolean;
  tunLoading: boolean;
  onEnablePf: () => void;
  onDisablePf: () => void;
  onEnableTun: () => void;
}

export function Footer({
  networkInfo, pfEnabled, pfLoading, tunEnabled, tunLoading,
  onEnablePf, onDisablePf, onEnableTun,
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
        {!tunEnabled && (
          <button className="btn btn-sm btn-secondary" onClick={onEnableTun} disabled={tunLoading}>
            {tunLoading ? "..." : "TUN Mode"}
          </button>
        )}
      </div>
    </div>
  );
}
