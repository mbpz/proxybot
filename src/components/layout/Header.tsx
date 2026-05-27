import type { CaMetadata } from "../../types";

interface HeaderProps {
  running: boolean;
  caMetadata: CaMetadata | null;
  onStart: () => void;
  onDownloadCa: () => void;
}

export function Header({ running, caMetadata, onStart, onDownloadCa }: HeaderProps) {
  return (
    <header style={{
      display: "flex", alignItems: "center", justifyContent: "space-between",
      padding: "var(--space-3) var(--space-4)",
      background: "var(--bg-secondary)", borderBottom: "1px solid var(--border)",
    }}>
      <div style={{ display: "flex", alignItems: "center", gap: "var(--space-3)" }}>
        <h1 style={{ fontSize: "var(--text-lg)", fontWeight: 700, margin: 0 }}>ProxyBot</h1>
        <span style={{
          width: 8, height: 8, borderRadius: "50%",
          background: running ? "var(--accent-green)" : "var(--text-muted)",
        }} />
        <span className="text-sm text-secondary">
          {running ? "Proxy running on :8080" : "Stopped"}
        </span>
      </div>
      <div style={{ display: "flex", gap: "var(--space-2)", alignItems: "center" }}>
        {caMetadata && (
          <span className="text-xs text-muted" style={{ fontFamily: "var(--font-mono)" }}>
            CA: {new Date(caMetadata.created_at * 1000).toLocaleDateString()}
          </span>
        )}
        <button className="btn btn-sm btn-secondary" onClick={onDownloadCa} title="Copy CA cert to clipboard">
          📜 CA
        </button>
        <button
          className={`btn btn-sm ${running ? "btn-danger" : "btn-primary"}`}
          onClick={onStart}
          disabled={running}
        >
          {running ? "Running" : "Start"}
        </button>
      </div>
    </header>
  );
}
