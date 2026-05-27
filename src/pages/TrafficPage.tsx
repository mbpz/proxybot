import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { InterceptedRequest, ReplayTarget, ReplayResult, AppTab } from "../types";
import { formatTimestamp, formatSize, formatBody, appBadgeClass } from "../utils";

interface TrafficPageProps {
  requests: InterceptedRequest[];
  onError: (msg: string) => void;
}

interface HarExport {
  log: {
    version: string;
    entries: unknown[];
  };
}

export function TrafficPage({ requests, onError }: TrafficPageProps) {
  const [selectedTab, setSelectedTab] = useState<AppTab>("all");
  const [selectedHost, setSelectedHost] = useState("all");
  const [keywordFilter, setKeywordFilter] = useState("");
  const [selectedRequest, setSelectedRequest] = useState<InterceptedRequest | null>(null);
  const [detailTab, setDetailTab] = useState<"headers" | "params" | "body" | "ws">("headers");
  const [sessionName, setSessionName] = useState("");
  const [showExportDialog, setShowExportDialog] = useState(false);
  const [exporting, setExporting] = useState(false);

  // Replay state
  const [replayTargets, setReplayTargets] = useState<ReplayTarget[]>([]);
  const [selectedReplayHost, setSelectedReplayHost] = useState("");
  const [replayDelay, setReplayDelay] = useState(100);
  const [replayResults, setReplayResults] = useState<ReplayResult[]>([]);
  const [replaying, setReplaying] = useState(false);

  useEffect(() => {
    invoke<ReplayTarget[]>("get_replay_targets")
      .then(setReplayTargets)
      .catch(console.error);
  }, []);

  const exportHar = async () => {
    try {
      setExporting(true);
      const name = sessionName.trim() || `session-${Date.now()}`;
      const har = await invoke<HarExport>("export_har", { sessionName: name });
      const harJson = JSON.stringify(har, null, 2);
      const path = await invoke<string>("save_har_file", { harJson, sessionName: name });
      onError(`HAR file saved to: ${path}`);
      setShowExportDialog(false);
      setSessionName("");
    } catch (e) {
      onError(String(e));
    } finally {
      setExporting(false);
    }
  };

  const startReplay = async () => {
    if (!selectedReplayHost) { onError("Please select a host to replay"); return; }
    try {
      setReplaying(true);
      setReplayResults([]);
      const results = await invoke<ReplayResult[]>("start_replay", {
        host: selectedReplayHost, delayMs: replayDelay,
      });
      setReplayResults(results);
    } catch (e) {
      onError(String(e));
    } finally {
      setReplaying(false);
    }
  };

  const filteredRequests = requests
    .filter((req) => {
      if (selectedTab !== "all") {
        if (selectedTab === "Unknown") return !req.app_name;
        return req.app_name === selectedTab;
      }
      return true;
    })
    .filter((req) => selectedHost === "all" || req.host === selectedHost)
    .filter((req) => {
      if (!keywordFilter) return true;
      const kw = keywordFilter.toLowerCase();
      return req.host.toLowerCase().includes(kw) || req.path.toLowerCase().includes(kw) || req.method.toLowerCase().includes(kw);
    });

  return (
    <div style={{ display: "grid", gridTemplateColumns: "1fr 400px", gap: "var(--space-4)", alignItems: "start" }}>
      {/* Request list panel */}
      <div className="panel">
        <div className="panel-header">
          <span className="panel-title">Traffic</span>
          <span className="text-sm text-muted">{requests.length} requests</span>
        </div>

        <div style={{ padding: "var(--space-3)", borderBottom: "1px solid var(--border)", display: "flex", gap: "var(--space-3)", flexWrap: "wrap", alignItems: "center" }}>
          <select value={selectedHost} onChange={(e) => setSelectedHost(e.target.value)} style={{ width: 180 }}>
            <option value="all">All Hosts</option>
            {[...new Set(requests.map((r) => r.host))].map((h) => (
              <option key={h} value={h}>{h}</option>
            ))}
          </select>
          <div className="tabs" style={{ borderBottom: "none", gap: 2 }}>
            {(["all", "WeChat", "Douyin", "Alipay", "Unknown"] as AppTab[]).map((tab) => (
              <button key={tab} className={`tab ${selectedTab === tab ? "active" : ""}`}
                onClick={() => setSelectedTab(tab)}
                style={{ fontSize: "var(--text-xs)", padding: "var(--space-1) var(--space-2)" }}>
                {tab === "all" ? "All" : tab}
              </button>
            ))}
          </div>
          <input type="text" placeholder="Filter by host or path..." value={keywordFilter}
            onChange={(e) => setKeywordFilter(e.target.value)} style={{ flex: 1, minWidth: 140 }} />
          <button className="btn btn-sm btn-secondary"
            onClick={() => { setSessionName(`session-${Date.now()}`); setShowExportDialog(true); }}>
            Export HAR
          </button>
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

        {filteredRequests.length === 0 ? (
          <div className="empty-state">
            <div className="empty-state-icon">📭</div>
            <div className="empty-state-title">No requests captured</div>
            <div className="empty-state-description">Start the proxy and make requests from your phone to see traffic here.</div>
          </div>
        ) : (
          <div style={{ maxHeight: 520, overflowY: "auto" }}>
            {filteredRequests.map((req) => (
              <div key={req.id}
                onClick={() => setSelectedRequest(selectedRequest?.id === req.id ? null : req)}
                style={{
                  display: "flex", padding: "var(--space-2) var(--space-3)",
                  borderBottom: "1px solid var(--border)", cursor: "pointer",
                  fontSize: "var(--text-sm)", fontFamily: "var(--font-mono)", alignItems: "center",
                  background: selectedRequest?.id === req.id ? "var(--bg-tertiary)" : "transparent",
                }}>
                <div style={{ width: 60 }}><span className={`badge badge-${req.method.toLowerCase()}`}>{req.method}</span></div>
                <div style={{ flex: 1, overflow: "hidden" }}>
                  <div className="truncate" style={{ fontSize: "var(--text-xs)" }}>{req.host}</div>
                  <div className="text-muted truncate" style={{ fontSize: 10 }}>{req.path}</div>
                </div>
                <div style={{ width: 56, textAlign: "center" }}>
                  {req.status && (
                    <span style={{
                      color: req.status < 300 ? "var(--accent-green)" : req.status < 400 ? "var(--accent-yellow)" : "var(--accent-red)",
                      fontWeight: 600, fontSize: "var(--text-xs)",
                    }}>{req.status}</span>
                  )}
                </div>
                <div style={{ width: 64, textAlign: "right", fontSize: "var(--text-xs)" }}>{req.latency_ms != null ? `${req.latency_ms}ms` : "—"}</div>
                <div style={{ width: 64, textAlign: "right", fontSize: "var(--text-xs)" }}>{req.resp_size != null ? formatSize(req.resp_size) : "—"}</div>
                <div style={{ width: 88, fontSize: "var(--text-xs)", color: "var(--text-secondary)" }}>{formatTimestamp(req.timestamp)}</div>
                <div style={{ width: 80 }}>
                  {req.app_name && <span className={`badge ${appBadgeClass(req.app_name)}`}>{req.app_name}</span>}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Right panel: detail + replay */}
      <div>
        {selectedRequest ? (
          <div className="panel">
            <div className="panel-header">
              <span className="panel-title">
                <span className={`badge badge-${selectedRequest.method.toLowerCase()}`} style={{ marginRight: 8 }}>{selectedRequest.method}</span>
                {selectedRequest.host}
                <span className="text-muted truncate" style={{ marginLeft: 8, maxWidth: 160 }}>{selectedRequest.path}</span>
              </span>
              <button className="btn btn-sm btn-ghost" onClick={() => setSelectedRequest(null)}>×</button>
            </div>
            <div className="tabs">
              {(["headers", "params", "body", ...(selectedRequest.is_websocket ? ["ws"] as const : [])] as const).map((t) => (
                <button key={t} className={`tab ${detailTab === t ? "active" : ""}`}
                  onClick={() => setDetailTab(t as typeof detailTab)}>
                  {t.charAt(0).toUpperCase() + t.slice(1)}
                </button>
              ))}
            </div>
            <div className="panel-body" style={{ maxHeight: 480, overflowY: "auto" }}>
              {detailTab === "headers" && (
                <div>
                  <div style={{ marginBottom: "var(--space-4)" }}>
                    <div className="card-title" style={{ marginBottom: "var(--space-2)" }}>Request Headers</div>
                    <table className="table"><tbody>
                      {selectedRequest.req_headers.map(([n, v]: [string, string]) => (
                        <tr key={n}><td style={{ fontWeight: 500, whiteSpace: "nowrap" }}>{n}</td>
                          <td className="mono" style={{ wordBreak: "break-all", fontSize: "var(--text-xs)" }}>{v}</td></tr>
                      ))}
                    </tbody></table>
                  </div>
                  <div>
                    <div className="card-title" style={{ marginBottom: "var(--space-2)" }}>Response Headers</div>
                    <table className="table"><tbody>
                      {selectedRequest.resp_headers.map(([n, v]: [string, string]) => (
                        <tr key={n}><td style={{ fontWeight: 500, whiteSpace: "nowrap" }}>{n}</td>
                          <td className="mono" style={{ wordBreak: "break-all", fontSize: "var(--text-xs)" }}>{v}</td></tr>
                      ))}
                    </tbody></table>
                  </div>
                </div>
              )}
              {detailTab === "params" && (
                <div>
                  {selectedRequest.query_params ? (
                    selectedRequest.query_params.split("&").map((p: string) => {
                      const [k, v] = p.split("=");
                      return (
                        <div key={k} style={{ display: "flex", gap: "var(--space-3)", padding: "var(--space-1) 0", borderBottom: "1px solid var(--border)" }}>
                          <span className="mono" style={{ color: "var(--accent-blue)", minWidth: 120 }}>{decodeURIComponent(k)}</span>
                          <span className="mono" style={{ fontSize: "var(--text-xs)" }}>{decodeURIComponent(v || "")}</span>
                        </div>
                      );
                    })
                  ) : (
                    <div className="empty-state" style={{ padding: "var(--space-6)" }}>
                      <div className="text-muted text-sm">No query parameters</div>
                    </div>
                  )}
                </div>
              )}
              {detailTab === "body" && (
                <div>
                  {selectedRequest.resp_body ? (
                    <pre className="mono" style={{ fontSize: "var(--text-xs)", whiteSpace: "pre-wrap", wordBreak: "break-all", background: "var(--bg-primary)", padding: "var(--space-3)", borderRadius: "var(--radius-md)" }}>
                      {formatBody(selectedRequest.resp_body, selectedRequest.resp_headers)}
                    </pre>
                  ) : (
                    <div className="empty-state" style={{ padding: "var(--space-6)" }}>
                      <div className="text-muted text-sm">No response body</div>
                    </div>
                  )}
                </div>
              )}
              {detailTab === "ws" && selectedRequest.ws_frames && (
                <div>
                  {selectedRequest.ws_frames.map((f: { direction: string; timestamp: string; size: number; payload: string }, i: number) => (
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

        {/* Replay section */}
        <div className="panel" style={{ marginTop: "var(--space-4)" }}>
          <div className="panel-header"><span className="panel-title">Replay</span></div>
          <div className="panel-body">
            <div style={{ display: "flex", gap: "var(--space-2)", marginBottom: "var(--space-3)", flexWrap: "wrap" }}>
              <select value={selectedReplayHost} onChange={(e) => setSelectedReplayHost(e.target.value)} style={{ flex: 1, minWidth: 120 }}>
                <option value="">Select host...</option>
                {replayTargets.map((t) => (
                  <option key={t.host} value={t.host}>{t.host} ({t.request_count})</option>
                ))}
              </select>
              <input type="number" min="0" max="5000" value={replayDelay}
                onChange={(e) => setReplayDelay(Number(e.target.value))} style={{ width: 80 }} title="Delay (ms)" />
              <button className="btn btn-sm btn-primary" onClick={startReplay} disabled={replaying || !selectedReplayHost}>
                {replaying ? "..." : "Replay"}
              </button>
            </div>
            {replayResults.length > 0 && (
              <div style={{ marginTop: "var(--space-3)", maxHeight: 200, overflowY: "auto" }}>
                {replayResults.slice(0, 5).map((r) => (
                  <div key={r.request_id} style={{ display: "flex", gap: "var(--space-2)", padding: "var(--space-1) 0", borderBottom: "1px solid var(--border)", fontSize: "var(--text-xs)" }}>
                    <span className={`badge ${r.error ? "badge-delete" : r.diff?.has_changes ? "badge-put" : "badge-get"}`}>
                      {r.error ? "Err" : r.mock_response?.status || "?"}
                    </span>
                    <span className="mono truncate" style={{ flex: 1 }}>{r.url}</span>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Export dialog */}
      {showExportDialog && (
        <div style={{ position: "fixed", top: 0, left: 0, right: 0, bottom: 0, background: "rgba(0,0,0,0.6)", display: "flex", alignItems: "center", justifyContent: "center", zIndex: 100 }}>
          <div className="panel" style={{ width: 400 }}>
            <div className="panel-header">
              <span className="panel-title">Export HAR</span>
              <button className="btn btn-sm btn-ghost" onClick={() => { setShowExportDialog(false); setSessionName(""); }}>×</button>
            </div>
            <div className="panel-body">
              <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-3)" }}>
                <div>
                  <label className="text-sm text-muted" style={{ display: "block", marginBottom: 4 }}>Session Name</label>
                  <input type="text" value={sessionName} onChange={(e) => setSessionName(e.target.value)} placeholder="session-1234567890" style={{ width: "100%" }} />
                </div>
                <div className="text-xs text-muted">Saved to ~/.proxybot/exports/</div>
                <div style={{ display: "flex", gap: "var(--space-2)", justifyContent: "flex-end" }}>
                  <button className="btn btn-sm btn-secondary" onClick={() => { setShowExportDialog(false); setSessionName(""); }}>Cancel</button>
                  <button className="btn btn-sm btn-primary" onClick={exportHar} disabled={exporting}>{exporting ? "..." : "Export"}</button>
                </div>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
