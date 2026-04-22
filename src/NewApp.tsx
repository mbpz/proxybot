import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./index.css";

// ============================================================
// Types (replicated from App.tsx to avoid import issues)
// ============================================================
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

interface NetworkInfo {
  lan_ip: string;
  interface: string;
}

// ============================================================
// Utility functions
// ============================================================
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

function formatBody(body: string | undefined, headers: [string, string][]): string {
  if (!body) return "";
  const ct = headers.find(([n]) => n.toLowerCase() === "content-type");
  if (ct && ct[1].includes("application/json")) {
    try { return JSON.stringify(JSON.parse(body), null, 2); } catch { return body; }
  }
  return body;
}

function copyToClipboard(text: string) {
  navigator.clipboard.writeText(text).catch(() => {});
}

// ============================================================
// Badge component
// ============================================================
function MethodBadge({ method }: { method: string }) {
  const cls = {
    GET: "badge badge-get", POST: "badge badge-post", PUT: "badge badge-put",
    DELETE: "badge badge-delete", PATCH: "badge badge-patch",
  }[method.toUpperCase()] || "badge badge-info";
  return <span className={cls}>{method}</span>;
}

function AppBadge({ name }: { name?: string }) {
  if (!name) return <span className="badge badge-unknown">Unknown</span>;
  const lower = name.toLowerCase();
  if (lower.includes("wechat")) return <span className="badge badge-wechat">{name}</span>;
  if (lower.includes("douyin") || lower.includes("tiktok")) return <span className="badge badge-douyin">{name}</span>;
  if (lower.includes("alipay")) return <span className="badge badge-alipay">{name}</span>;
  return <span className="badge badge-unknown">{name}</span>;
}

// ============================================================
// Skeleton
// ============================================================
function SkeletonRows({ rows = 8 }: { rows?: number }) {
  return (
    <div>
      {Array.from({ length: rows }).map((_, i) => (
        <div key={i} className="skeleton-row">
          <div className="skeleton skeleton-cell" style={{ width: 56, height: 16 }} />
          <div className="skeleton skeleton-cell" style={{ flex: 1, height: 16 }} />
          <div className="skeleton skeleton-cell" style={{ width: 48, height: 16 }} />
          <div className="skeleton skeleton-cell" style={{ width: 64, height: 16 }} />
          <div className="skeleton skeleton-cell" style={{ width: 64, height: 16 }} />
          <div className="skeleton skeleton-cell" style={{ width: 80, height: 16 }} />
        </div>
      ))}
    </div>
  );
}

// ============================================================
// Error Banner
// ============================================================
function ErrorBanner({ message, onRetry }: { message: string; onRetry?: () => void }) {
  return (
    <div className="error-banner">
      <span className="error-banner-message">{message}</span>
      {onRetry && (
        <button className="btn btn-sm btn-secondary" onClick={onRetry}>Retry</button>
      )}
    </div>
  );
}

// ============================================================
// Main App
// ============================================================
export default function NewApp() {
  const [running, setRunning] = useState(false);
  const [requests, setRequests] = useState<InterceptedRequest[]>([]);
  const [networkInfo, setNetworkInfo] = useState<NetworkInfo | null>(null);
  const [pfEnabled, setPfEnabled] = useState(false);
  const [tunEnabled, setTunEnabled] = useState(false);
  const [selectedRequest, setSelectedRequest] = useState<InterceptedRequest | null>(null);
  const [detailTab, setDetailTab] = useState<"headers" | "params" | "body" | "ws">("headers");
  const [activeTab, setActiveTab] = useState<"traffic" | "dns" | "rules" | "devices" | "ai">("traffic");
  const [appFilter, setAppFilter] = useState<string>("all");
  const [hostFilter, setHostFilter] = useState<string>("all");
  const [keyword, setKeyword] = useState("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [pfLoading, setPfLoading] = useState(false);
  const [tunLoading, setTunLoading] = useState(false);

  useEffect(() => {
    Promise.all([
      invoke<NetworkInfo>("get_network_info").then(setNetworkInfo).catch(() => {}),
      invoke<boolean>("is_pf_enabled").then(setPfEnabled).catch(() => {}),
      invoke<boolean>("is_tun_enabled").then(setTunEnabled).catch(() => {}),
    ]).finally(() => setLoading(false));

    const unlisten1 = listen<InterceptedRequest>("intercepted-request", (ev) => {
      setRequests((prev) => [ev.payload, ...prev].slice(0, 500));
    });
    const unlisten2 = listen<InterceptedRequest>("dns-query", () => {});

    return () => {
      unlisten1.then((f) => f());
      unlisten2.then((f) => f());
    };
  }, []);

  const startProxy = async () => {
    try {
      setError(null);
      await invoke<string>("start_proxy");
      setRunning(true);
    } catch (e) {
      setError(String(e));
    }
  };

  const enablePf = async () => {
    if (!networkInfo) return;
    try {
      setPfLoading(true);
      setError(null);
      const already = await invoke<boolean>("is_pf_enabled");
      if (already) { setPfEnabled(true); return; }
      await invoke<string>("setup_pf");
      setPfEnabled(true);
    } catch (e) {
      setError(String(e));
    } finally {
      setPfLoading(false);
    }
  };

  const disablePf = async () => {
    try {
      setPfLoading(true);
      await invoke<void>("teardown_pf");
      setPfEnabled(false);
    } catch (e) {
      setError(String(e));
    } finally {
      setPfLoading(false);
    }
  };

  const enableTun = async () => {
    try {
      setTunLoading(true);
      setError(null);
      await invoke<string>("setup_tun");
      setTunEnabled(true);
    } catch (e) {
      setError(String(e));
    } finally {
      setTunLoading(false);
    }
  };

  const downloadCa = async () => {
    try {
      const pem = await invoke<string>("get_ca_cert_pem");
      await navigator.clipboard.writeText(pem);
      alert("CA cert copied to clipboard — save as .pem");
    } catch (e) {
      setError(String(e));
    }
  };

  const hosts = Array.from(new Set(requests.map((r) => r.host)));

  const filtered = requests.filter((r) => {
    if (appFilter !== "all" && r.app_name !== appFilter) return false;
    if (hostFilter !== "all" && r.host !== hostFilter) return false;
    if (keyword) {
      const kw = keyword.toLowerCase();
      if (!r.host.toLowerCase().includes(kw) && !r.path.toLowerCase().includes(kw)) return false;
    }
    return true;
  });

  const APP_TABS = ["all", "WeChat", "Douyin", "Alipay", "Unknown"];

  return (
    <div style={{ minHeight: "100vh", background: "var(--bg-primary)" }}>
      {/* Header */}
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
        <div style={{ display: "flex", gap: "var(--space-2)" }}>
          <button className="btn btn-sm btn-secondary" onClick={downloadCa} title="Download CA cert">
            📜 CA
          </button>
          <button
            className={`btn btn-sm ${running ? "btn-danger" : "btn-primary"}`}
            onClick={startProxy}
            disabled={running}
          >
            {running ? "Running" : "Start"}
          </button>
        </div>
      </header>

      {/* Tab bar */}
      <div className="tabs" style={{ padding: "0 var(--space-4)", background: "var(--bg-secondary)", borderBottom: "1px solid var(--border)" }}>
        {(["traffic", "dns", "rules", "devices", "ai"] as const).map((tab) => (
          <button
            key={tab}
            className={`tab ${activeTab === tab ? "active" : ""}`}
            onClick={() => setActiveTab(tab)}
          >
            {tab.charAt(0).toUpperCase() + tab.slice(1)}
          </button>
        ))}
      </div>

      {/* Main content */}
      <div style={{ padding: "var(--space-4)", display: "flex", gap: "var(--space-4)", flexDirection: "column" }}>
        {error && <ErrorBanner message={error} />}

        {activeTab === "traffic" && (
          <div style={{ display: "grid", gridTemplateColumns: "1fr 380px", gap: "var(--space-4)", alignItems: "start" }}>
            {/* Request list */}
            <div className="panel">
              <div className="panel-header">
                <span className="panel-title">Traffic</span>
                <span className="text-sm text-muted">{filtered.length}</span>
              </div>

              {/* Filters */}
              <div style={{ padding: "var(--space-3)", borderBottom: "1px solid var(--border)", display: "flex", gap: "var(--space-3)", flexWrap: "wrap", alignItems: "center" }}>
                <select value={hostFilter} onChange={(e) => setHostFilter(e.target.value)} style={{ width: 160 }}>
                  <option value="all">All Hosts</option>
                  {hosts.map((h) => <option key={h} value={h}>{h}</option>)}
                </select>
                <div className="tabs" style={{ borderBottom: "none", gap: 2 }}>
                  {APP_TABS.map((tab) => (
                    <button
                      key={tab}
                      className={`tab ${appFilter === tab ? "active" : ""}`}
                      onClick={() => setAppFilter(tab)}
                      style={{ fontSize: "var(--text-xs)", padding: "var(--space-1) var(--space-2)" }}
                    >
                      {tab === "all" ? "All" : tab}
                    </button>
                  ))}
                </div>
                <input
                  type="text"
                  placeholder="Filter..."
                  value={keyword}
                  onChange={(e) => setKeyword(e.target.value)}
                  style={{ flex: 1, minWidth: 120 }}
                />
              </div>

              {/* Table header */}
              <div style={{
                display: "flex", padding: "var(--space-2) var(--space-3)",
                background: "var(--bg-tertiary)", fontSize: "var(--text-xs)", fontWeight: 600,
                color: "var(--text-secondary)", textTransform: "uppercase" as const,
                letterSpacing: "0.5px", borderBottom: "1px solid var(--border)",
              }}>
                <div style={{ width: 60 }}>Method</div>
                <div style={{ flex: 1 }}>URL</div>
                <div style={{ width: 56, textAlign: "center" }}>Status</div>
                <div style={{ width: 64, textAlign: "right" }}>Latency</div>
                <div style={{ width: 64, textAlign: "right" }}>Size</div>
                <div style={{ width: 88 }}>Time</div>
                <div style={{ width: 80 }}>App</div>
              </div>

              {/* Rows */}
              {loading ? (
                <SkeletonRows rows={10} />
              ) : filtered.length === 0 ? (
                <div className="empty-state">
                  <div className="empty-state-icon">📭</div>
                  <div className="empty-state-title">No requests captured</div>
                  <div className="empty-state-description">Start proxy and make requests from your phone</div>
                </div>
              ) : (
                <div style={{ maxHeight: 520, overflowY: "auto" }}>
                  {filtered.map((req) => (
                    <div
                      key={req.id}
                      onClick={() => setSelectedRequest(req.id === selectedRequest?.id ? null : req)}
                      style={{
                        display: "flex", padding: "var(--space-2) var(--space-3)",
                        borderBottom: "1px solid var(--border)", cursor: "pointer",
                        fontSize: "var(--text-sm)", fontFamily: "var(--font-mono)",
                        alignItems: "center",
                        background: selectedRequest?.id === req.id ? "var(--bg-tertiary)" : "transparent",
                      }}
                    >
                      <div style={{ width: 60 }}><MethodBadge method={req.method} /></div>
                      <div style={{ flex: 1, overflow: "hidden" }}>
                        <div className="truncate" style={{ fontSize: "var(--text-xs)" }}>{req.host}</div>
                        <div className="text-muted truncate" style={{ fontSize: 10 }}>{req.path}</div>
                      </div>
                      <div style={{ width: 56, textAlign: "center" }}>
                        {req.status && (
                          <span style={{
                            color: req.status < 300 ? "var(--accent-green)"
                              : req.status < 400 ? "var(--accent-yellow)"
                              : "var(--accent-red)", fontWeight: 600, fontSize: "var(--text-xs)",
                          }}>{req.status}</span>
                        )}
                      </div>
                      <div style={{ width: 64, textAlign: "right", fontSize: "var(--text-xs)" }}>
                        {req.latency_ms != null ? `${req.latency_ms}ms` : "—"}
                      </div>
                      <div style={{ width: 64, textAlign: "right", fontSize: "var(--text-xs)" }}>
                        {req.resp_size != null ? formatSize(req.resp_size) : "—"}
                      </div>
                      <div style={{ width: 88, fontSize: "var(--text-xs)", color: "var(--text-secondary)" }}>
                        {formatTime(req.timestamp)}
                      </div>
                      <div style={{ width: 80 }}>
                        <AppBadge name={req.app_name} />
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>

            {/* Detail panel */}
            <div>
              {selectedRequest ? (
                <div className="panel">
                  <div className="panel-header">
                    <span className="panel-title">
                      <MethodBadge method={selectedRequest.method} />
                      <span style={{ marginLeft: 8 }}>{selectedRequest.host}</span>
                    </span>
                    <button className="btn btn-sm btn-ghost" onClick={() => setSelectedRequest(null)}>×</button>
                  </div>
                  <div className="tabs">
                    {(["headers", "params", "body", ...(selectedRequest.is_websocket ? ["ws"] as const : [])] as const).map((t) => (
                      <button key={t} className={`tab ${detailTab === t ? "active" : ""}`} onClick={() => setDetailTab(t as typeof detailTab)}>
                        {t.charAt(0).toUpperCase() + t.slice(1)}
                      </button>
                    ))}
                  </div>
                  <div className="panel-body" style={{ maxHeight: 480, overflowY: "auto" }}>
                    {detailTab === "headers" && (
                      <div>
                        <div style={{ marginBottom: "var(--space-4)" }}>
                          <div className="card-title" style={{ marginBottom: "var(--space-2)" }}>Request</div>
                          <table className="table"><tbody>
                            {selectedRequest.req_headers.map(([n, v]) => (
                              <tr key={n}><td style={{ fontWeight: 500, whiteSpace: "nowrap" }}>{n}</td><td className="mono" style={{ wordBreak: "break-all" }}>{v}</td></tr>
                            ))}
                          </tbody></table>
                        </div>
                        <div>
                          <div className="card-title" style={{ marginBottom: "var(--space-2)" }}>Response</div>
                          <table className="table"><tbody>
                            {selectedRequest.resp_headers.map(([n, v]) => (
                              <tr key={n}><td style={{ fontWeight: 500, whiteSpace: "nowrap" }}>{n}</td><td className="mono" style={{ wordBreak: "break-all" }}>{v}</td></tr>
                            ))}
                          </tbody></table>
                        </div>
                      </div>
                    )}
                    {detailTab === "params" && (
                      <div>
                        {selectedRequest.query_params ? (
                          selectedRequest.query_params.split("&").map((p) => {
                            const [k, v] = p.split("=");
                            return (
                              <div key={k} style={{ display: "flex", gap: "var(--space-3)", padding: "var(--space-1) 0", borderBottom: "1px solid var(--border)" }}>
                                <span className="mono" style={{ color: "var(--accent-blue)", minWidth: 120 }}>{decodeURIComponent(k)}</span>
                                <span className="mono">{decodeURIComponent(v || "")}</span>
                              </div>
                            );
                          })
                        ) : (
                          <div className="text-muted text-sm">No query parameters</div>
                        )}
                      </div>
                    )}
                    {detailTab === "body" && (
                      <div>
                        {selectedRequest.resp_body ? (
                          <div style={{ position: "relative" }}>
                            <button className="btn btn-sm btn-ghost" style={{ position: "absolute", top: 0, right: 0 }} onClick={() => copyToClipboard(formatBody(selectedRequest.resp_body, selectedRequest.resp_headers))}>📋 Copy</button>
                            <pre className="mono" style={{ fontSize: "var(--text-xs)", whiteSpace: "pre-wrap", wordBreak: "break-all", background: "var(--bg-primary)", padding: "var(--space-3)", borderRadius: "var(--radius-md)" }}>
                              {formatBody(selectedRequest.resp_body, selectedRequest.resp_headers)}
                            </pre>
                          </div>
                        ) : (
                          <div className="text-muted text-sm">No body</div>
                        )}
                      </div>
                    )}
                    {detailTab === "ws" && selectedRequest.ws_frames && (
                      <div>
                        {selectedRequest.ws_frames.map((f, i) => (
                          <div key={i} style={{ display: "flex", gap: "var(--space-3)", padding: "var(--space-2)", borderBottom: "1px solid var(--border)", alignItems: "flex-start" }}>
                            <span className={`badge ${f.direction === "←" || f.direction === "IN" ? "badge-get" : "badge-post"}`}>{f.direction}</span>
                            <div style={{ flex: 1 }}>
                              <div className="text-xs text-muted" style={{ marginBottom: 2 }}>{f.timestamp} · {f.size}B</div>
                              <pre className="mono" style={{ fontSize: "var(--text-xs)", whiteSpace: "pre-wrap", wordBreak: "break-all", margin: 0 }}>{f.payload}</pre>
                            </div>
                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                </div>
              ) : (
                <div className="panel">
                  <div className="panel-header"><span className="panel-title">Request Detail</span></div>
                  <div className="panel-body">
                    <div className="empty-state">
                      <div className="empty-state-icon">👆</div>
                      <div className="empty-state-title">Select a request</div>
                      <div className="empty-state-description">Click a row to inspect details</div>
                    </div>
                  </div>
                </div>
              )}
            </div>
          </div>
        )}

        {activeTab === "dns" && (
          <div className="panel">
            <div className="panel-header"><span className="panel-title">DNS Queries</span></div>
            <div className="panel-body">
              <div className="empty-state">
                <div className="empty-state-icon">🌐</div>
                <div className="empty-state-title">DNS log</div>
                <div className="empty-state-description">Configure transparent proxy to start capturing DNS queries</div>
              </div>
            </div>
          </div>
        )}

        {activeTab === "rules" && (
          <div className="panel">
            <div className="panel-header"><span className="panel-title">Rules</span></div>
            <div className="panel-body">
              <div className="empty-state">
                <div className="empty-state-icon">📋</div>
                <div className="empty-state-title">Rule editor</div>
                <div className="empty-state-description">Manage routing rules for traffic direction</div>
              </div>
            </div>
          </div>
        )}

        {activeTab === "devices" && (
          <div className="panel">
            <div className="panel-header"><span className="panel-title">Devices</span></div>
            <div className="panel-body">
              <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(280px, 1fr))", gap: "var(--space-3)" }}>
                <div className="card" style={{ display: "flex", flexDirection: "column", gap: "var(--space-2)" }}>
                  <div style={{ fontWeight: 600 }}>Phone — 192.168.1.100</div>
                  <div className="text-sm text-muted">MAC: 00:11:22:33:44:55</div>
                  <div style={{ display: "flex", gap: "var(--space-4)", fontSize: "var(--text-xs)" }}>
                    <span>↑ 2.4 MB</span>
                    <span>↓ 1.8 MB</span>
                    <span>Last seen: 2 min ago</span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        )}

        {activeTab === "ai" && (
          <div className="panel">
            <div className="panel-header"><span className="panel-title">AI Analysis</span></div>
            <div className="panel-body">
              <div className="empty-state">
                <div className="empty-state-icon">🤖</div>
                <div className="empty-state-title">AI Analysis</div>
                <div className="empty-state-description">Auth flows, anomaly detection, and API inference appear here</div>
              </div>
            </div>
          </div>
        )}
      </div>

      {/* Proxy control footer */}
      <div style={{
        position: "fixed", bottom: 0, left: 0, right: 0,
        padding: "var(--space-3) var(--space-4)",
        background: "var(--bg-secondary)", borderTop: "1px solid var(--border)",
        display: "flex", alignItems: "center", justifyContent: "space-between",
        fontSize: "var(--text-sm)",
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
            <button className="btn btn-sm btn-secondary" onClick={enablePf} disabled={pfLoading || !networkInfo}>
              {pfLoading ? "Enabling..." : "Enable pf"}
            </button>
          ) : (
            <button className="btn btn-sm btn-secondary" onClick={disablePf} disabled={pfLoading}>
              {pfLoading ? "Disabling..." : "Disable pf"}
            </button>
          )}
          {!tunEnabled && (
            <button className="btn btn-sm btn-secondary" onClick={enableTun} disabled={tunLoading}>
              {tunLoading ? "Enabling..." : "TUN Mode"}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
