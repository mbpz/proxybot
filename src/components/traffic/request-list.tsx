import { useState, useRef } from "react";

interface InterceptedRequest {
  id: string;
  timestamp: string;
  method: string;
  host: string;
  path: string;
  query_params?: string;
  status: number | null;
  latency_ms: number | null;
  scheme: string;
  req_headers: [string, string][];
  req_body?: string;
  resp_headers: [string, string][];
  resp_body?: string;
  resp_size?: number;
  app_name?: string;
  app_icon?: string;
  is_websocket: boolean;
  ws_frames?: WsFrame[];
}

interface WsFrame {
  direction: string;
  timestamp: string;
  payload: string;
  size: number;
}

type AppTab = "all" | "WeChat" | "Douyin" | "Alipay" | "Unknown";

interface Props {
  requests: InterceptedRequest[];
  onSelect: (req: InterceptedRequest) => void;
  selectedId: string | null;
  loading?: boolean;
  error?: string | null;
  onRetry?: () => void;
}

const APP_TABS: { label: string; value: AppTab }[] = [
  { label: "All", value: "all" },
  { label: "WeChat", value: "WeChat" },
  { label: "Douyin", value: "Douyin" },
  { label: "Alipay", value: "Alipay" },
  { label: "Unknown", value: "Unknown" },
];

function formatTime(ts: string): string {
  try {
    const parts = ts.split(".");
    const secs = parseInt(parts[0]);
    const ms = parts[1] || "000";
    const date = new Date(secs * 1000);
    return date.toLocaleTimeString() + "." + ms.slice(0, 3);
  } catch {
    return ts;
  }
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes}B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)}KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)}MB`;
}

function getMethodClass(method: string): string {
  switch (method.toUpperCase()) {
    case "GET":
      return "badge badge-get";
    case "POST":
      return "badge badge-post";
    case "PUT":
      return "badge badge-put";
    case "DELETE":
      return "badge badge-delete";
    case "PATCH":
      return "badge badge-patch";
    default:
      return "badge badge-info";
  }
}

function getAppBadgeClass(appName?: string): string {
  if (!appName) return "badge badge-unknown";
  const lower = appName.toLowerCase();
  if (lower.includes("wechat")) return "badge badge-wechat";
  if (lower.includes("douyin") || lower.includes("tiktok")) return "badge badge-douyin";
  if (lower.includes("alipay")) return "badge badge-alipay";
  return "badge badge-unknown";
}

export function RequestList({ requests, onSelect, selectedId, loading, error, onRetry }: Props) {
  const [hostFilter, setHostFilter] = useState<string>("all");
  const [appFilter, setAppFilter] = useState<AppTab>("all");
  const [keyword, setKeyword] = useState("");
  const listRef = useRef<HTMLDivElement>(null);

  // Extract unique hosts for filter dropdown
  const hosts = Array.from(new Set(requests.map((r) => r.host)));

  // Filter requests
  const filtered = requests.filter((req) => {
    if (hostFilter !== "all" && req.host !== hostFilter) return false;
    if (appFilter !== "all" && req.app_name !== appFilter) return false;
    if (keyword) {
      const kw = keyword.toLowerCase();
      if (!req.host.toLowerCase().includes(kw) && !req.path.toLowerCase().includes(kw)) {
        return false;
      }
    }
    return true;
  });

  if (loading) {
    return (
      <div className="panel">
        <div className="panel-header">
          <span className="panel-title">Traffic</span>
        </div>
        <div className="panel-body" style={{ padding: 0 }}>
          <SkeletonTable rows={8} />
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="panel">
        <div className="panel-header">
          <span className="panel-title">Traffic</span>
        </div>
        <div className="panel-body">
          <div className="error-banner">
            <span className="error-banner-message">{error}</span>
            {onRetry && (
              <button className="btn btn-sm btn-secondary" onClick={onRetry}>
                Retry
              </button>
            )}
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="panel">
      <div className="panel-header">
        <span className="panel-title">Traffic</span>
        <span className="text-sm text-muted">{filtered.length} requests</span>
      </div>

      {/* Filters */}
      <div className="filters" style={{ padding: "var(--space-3)", borderBottom: "1px solid var(--border)", display: "flex", gap: "var(--space-3)", flexWrap: "wrap" }}>
        <select value={hostFilter} onChange={(e) => setHostFilter(e.target.value)} style={{ width: 180 }}>
          <option value="all">All Hosts</option>
          {hosts.map((h) => (
            <option key={h} value={h}>{h}</option>
          ))}
        </select>

        <div className="tabs" style={{ borderBottom: "none", gap: "var(--space-1)" }}>
          {APP_TABS.map((tab) => (
            <button
              key={tab.value}
              className={`tab ${appFilter === tab.value ? "active" : ""}`}
              onClick={() => setAppFilter(tab.value)}
            >
              {tab.label}
            </button>
          ))}
        </div>

        <input
          type="text"
          placeholder="Filter by host or path..."
          value={keyword}
          onChange={(e) => setKeyword(e.target.value)}
          style={{ flex: 1, minWidth: 160 }}
        />
      </div>

      {/* Table header */}
      <div style={{ display: "flex", padding: "var(--space-2) var(--space-3)", background: "var(--bg-tertiary)", fontSize: "var(--text-xs)", fontWeight: 600, color: "var(--text-secondary)", textTransform: "uppercase", letterSpacing: "0.5px", borderBottom: "1px solid var(--border)" }}>
        <div style={{ width: 60 }}>Method</div>
        <div style={{ flex: 1 }}>URL</div>
        <div style={{ width: 60, textAlign: "center" }}>Status</div>
        <div style={{ width: 70, textAlign: "right" }}>Latency</div>
        <div style={{ width: 70, textAlign: "right" }}>Size</div>
        <div style={{ width: 90 }}>Time</div>
        <div style={{ width: 80 }}>App</div>
      </div>

      {/* Request list */}
      <div ref={listRef} style={{ maxHeight: 500, overflowY: "auto" }}>
        {filtered.length === 0 ? (
          <div className="empty-state">
            <div className="empty-state-icon">📭</div>
            <div className="empty-state-title">No requests captured yet</div>
            <div className="empty-state-description">
              Start the proxy and make requests from your phone to see traffic here.
            </div>
          </div>
        ) : (
          filtered.map((req) => (
            <div
              key={req.id}
              className={`request-row ${selectedId === req.id ? "selected" : ""}`}
              onClick={() => onSelect(req)}
              style={{
                display: "flex",
                padding: "var(--space-2) var(--space-3)",
                borderBottom: "1px solid var(--border)",
                cursor: "pointer",
                fontSize: "var(--text-sm)",
                fontFamily: "var(--font-mono)",
                alignItems: "center",
                background: selectedId === req.id ? "var(--bg-tertiary)" : "transparent",
              }}
            >
              <div style={{ width: 60 }}>
                <span className={getMethodClass(req.method)}>{req.method}</span>
              </div>
              <div style={{ flex: 1, overflow: "hidden" }}>
                <span className="truncate" style={{ display: "block" }}>{req.host}</span>
                <span className="text-muted truncate" style={{ fontSize: "var(--text-xs)" }}>{req.path}</span>
              </div>
              <div style={{ width: 60, textAlign: "center" }}>
                {req.status && (
                  <span style={{
                    color: req.status < 300 ? "var(--accent-green)" : req.status < 400 ? "var(--accent-yellow)" : "var(--accent-red)",
                    fontWeight: 600,
                  }}>
                    {req.status}
                  </span>
                )}
              </div>
              <div style={{ width: 70, textAlign: "right", fontSize: "var(--text-xs)" }}>
                {req.latency_ms != null ? `${req.latency_ms}ms` : "—"}
              </div>
              <div style={{ width: 70, textAlign: "right", fontSize: "var(--text-xs)" }}>
                {req.resp_size != null ? formatSize(req.resp_size) : "—"}
              </div>
              <div style={{ width: 90, fontSize: "var(--text-xs)", color: "var(--text-secondary)" }}>
                {formatTime(req.timestamp)}
              </div>
              <div style={{ width: 80 }}>
                {req.app_name && (
                  <span className={getAppBadgeClass(req.app_name)}>{req.app_name}</span>
                )}
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  );
}

// Inline SkeletonTable to avoid circular import
import { SkeletonTable } from "../ui/skeleton";
