import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { MethodBadge } from "../ui/Badge";
import { Button } from "../ui/Button";
import { ErrorBoundary } from "../ui/error-boundary";
import { SkeletonTable } from "../ui/skeleton";

interface ReplayTarget {
  host: string;
  request_count: number;
  path_count: number;
}

interface ReplayResult {
  request_id: number;
  method: string;
  url: string;
  recorded_response: {
    status: number;
    headers: [string, string][];
    body?: string;
  };
  mock_response?: {
    status: number;
    headers: [string, string][];
    body?: string;
  };
  diff?: DiffResult;
  delay_ms: number;
  error?: string;
}

interface DiffResult {
  header_diffs: HeaderDiff[];
  body_diff?: {
    recorded?: string;
    mock?: string;
    line_diffs: LineDiff[];
  };
  has_changes: boolean;
}

interface HeaderDiff {
  header: string;
  recorded?: string;
  mock?: string;
  diff_type: "Added" | "Removed" | "Modified" | "Unchanged";
}

interface LineDiff {
  line_number_recorded?: number;
  line_number_mock?: number;
  recorded_text?: string;
  mock_text?: string;
  diff_type: "Added" | "Removed" | "Modified" | "Unchanged";
}

export function ReplayPage() {
  const [targets, setTargets] = useState<ReplayTarget[]>([]);
  const [selectedHost, setSelectedHost] = useState<string>("");
  const [delayMs, setDelayMs] = useState(100);
  const [results, setResults] = useState<ReplayResult[]>([]);
  const [selectedResult, setSelectedResult] = useState<ReplayResult | null>(null);
  const [isRunning, setIsRunning] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    loadTargets();
  }, []);

  async function loadTargets() {
    try {
      setLoading(true);
      setError(null);
      const result = await invoke<ReplayTarget[]>("get_replay_targets");
      setTargets(result);
      if (result.length > 0 && !selectedHost) {
        setSelectedHost(result[0].host);
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }

  async function handleStartReplay() {
    if (!selectedHost) return;
    setIsRunning(true);
    setResults([]);
    setError(null);
    try {
      const result = await invoke<ReplayResult[]>("start_replay", {
        host: selectedHost,
        delay_ms: delayMs,
      });
      setResults(result);
    } catch (err) {
      setError(String(err));
    } finally {
      setIsRunning(false);
    }
  }

  function formatBody(body: string | undefined): string {
    if (!body) return "";
    try {
      return JSON.stringify(JSON.parse(body), null, 2);
    } catch {
      return body;
    }
  }

  function diffTypeColor(type: string): string {
    switch (type) {
      case "Added": return "var(--accent-green)";
      case "Removed": return "var(--accent-red)";
      case "Modified": return "var(--accent-yellow)";
      default: return "var(--text-muted)";
    }
  }

  const hosts = targets.map((t) => ({ value: t.host, label: `${t.host} (${t.request_count} reqs)` }));

  return (
    <div>
      {/* Controls */}
      <div className="panel" style={{ marginBottom: "var(--space-4)" }}>
        <div className="panel-header">
          <span className="panel-title">Replay</span>
          <Button variant="secondary" size="sm" onClick={loadTargets}>
            Refresh
          </Button>
        </div>
        <div className="panel-body">
          <div className="flex items-end gap-3" style={{ flexWrap: "wrap" }}>
            <div className="flex flex-col gap-1" style={{ minWidth: 240 }}>
              <label className="text-xs text-secondary font-mono">Target Host</label>
              <select
                value={selectedHost}
                onChange={(e) => setSelectedHost(e.target.value)}
                disabled={isRunning}
              >
                <option value="">Select a host...</option>
                {hosts.map((h) => (
                  <option key={h.value} value={h.value}>{h.label}</option>
                ))}
              </select>
            </div>
            <div className="flex flex-col gap-1" style={{ width: 120 }}>
              <label className="text-xs text-secondary font-mono">Delay (ms)</label>
              <input
                type="number"
                value={delayMs}
                onChange={(e) => setDelayMs(Number(e.target.value))}
                disabled={isRunning}
                min={0}
                max={5000}
              />
            </div>
            <Button
              variant="primary"
              onClick={handleStartReplay}
              disabled={isRunning || !selectedHost || targets.length === 0}
            >
              {isRunning ? "Replaying..." : "Start Replay"}
            </Button>
          </div>

          {/* Progress */}
          {isRunning && (
            <div style={{ marginTop: "var(--space-3)" }}>
              <div
                className="skeleton"
                style={{
                  height: 4,
                  borderRadius: "var(--radius-sm)",
                  background: "var(--accent-blue)",
                }}
              />
              <div className="text-xs text-muted" style={{ marginTop: "var(--space-1)" }}>
                Replaying requests...
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Error banner */}
      {error && (
        <div className="error-banner" style={{ marginBottom: "var(--space-4)" }}>
          <span className="error-banner-message">{error}</span>
          <Button variant="secondary" size="sm" onClick={loadTargets}>
            Retry
          </Button>
        </div>
      )}

      {/* Targets list */}
      <div className="panel" style={{ marginBottom: "var(--space-4)" }}>
        <div className="panel-header">
          <span className="panel-title">Targets ({targets.length})</span>
        </div>
        <div style={{ maxHeight: 200, overflowY: "auto" }}>
          <ErrorBoundary>
            {loading ? (
              <SkeletonTable rows={4} />
            ) : targets.length === 0 ? (
              <div className="empty-state">
                <div className="empty-state-icon">🔄</div>
                <div className="empty-state-title">No replay targets</div>
                <div className="empty-state-description">
                  Capture traffic first, then select a host above to replay.
                </div>
              </div>
            ) : (
              <table className="table">
                <thead>
                  <tr>
                    <th>Host</th>
                    <th style={{ width: 100 }}>Requests</th>
                    <th style={{ width: 100 }}>Unique Paths</th>
                  </tr>
                </thead>
                <tbody>
                  {targets.map((t) => (
                    <tr
                      key={t.host}
                      onClick={() => setSelectedHost(t.host)}
                      style={{
                        cursor: "pointer",
                        background: selectedHost === t.host ? "var(--bg-tertiary)" : undefined,
                      }}
                    >
                      <td className="mono text-sm">{t.host}</td>
                      <td className="text-sm">{t.request_count}</td>
                      <td className="text-sm">{t.path_count}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </ErrorBoundary>
        </div>
      </div>

      {/* Results */}
      {results.length > 0 && (
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "var(--space-4)", alignItems: "start" }}>
          {/* Results table */}
          <div className="panel">
            <div className="panel-header">
              <span className="panel-title">Results ({results.length})</span>
            </div>
            <div style={{ maxHeight: 400, overflowY: "auto" }}>
              <table className="table">
                <thead>
                  <tr>
                    <th style={{ width: 60 }}>Method</th>
                    <th>URL</th>
                    <th style={{ width: 60 }}>Status</th>
                    <th style={{ width: 50 }}>Diff</th>
                  </tr>
                </thead>
                <tbody>
                  {results.map((r) => (
                    <tr
                      key={r.request_id}
                      onClick={() => setSelectedResult(r)}
                      style={{
                        cursor: "pointer",
                        background: selectedResult?.request_id === r.request_id
                          ? "var(--bg-tertiary)"
                          : undefined,
                      }}
                    >
                      <td><MethodBadge method={r.method} /></td>
                      <td className="mono text-xs truncate" style={{ maxWidth: 200 }}>
                        {r.url}
                      </td>
                      <td>
                        <span
                          className="text-sm"
                          style={{
                            color: r.error ? "var(--accent-red)"
                              : r.mock_response && r.mock_response.status < 400 ? "var(--accent-green)"
                              : "var(--accent-yellow)",
                            fontWeight: 600,
                          }}
                        >
                          {r.error ? "ERR" : r.mock_response?.status ?? "—"}
                        </span>
                      </td>
                      <td>
                        {r.diff?.has_changes ? (
                          <span
                            className="badge badge-warning"
                            style={{ fontSize: 10 }}
                          >
                            Δ
                          </span>
                        ) : r.error ? (
                          <span style={{ color: "var(--accent-red)" }}>✕</span>
                        ) : (
                          <span style={{ color: "var(--accent-green)" }}>✓</span>
                        )}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>

          {/* Diff detail */}
          <div className="panel">
            <div className="panel-header">
              <span className="panel-title">
                {selectedResult
                  ? `Diff: ${selectedResult.method} ${selectedResult.url}`
                  : "Select a result"}
              </span>
            </div>
            <div className="panel-body" style={{ maxHeight: 400, overflowY: "auto" }}>
              {!selectedResult ? (
                <div className="text-sm text-muted">Click a result row to view diff</div>
              ) : selectedResult.error ? (
                <div className="error-banner">
                  <span className="error-banner-message">{selectedResult.error}</span>
                </div>
              ) : selectedResult.diff ? (
                <div>
                  {/* Header diffs */}
                  <div className="card-title" style={{ marginBottom: "var(--space-2)" }}>
                    Headers
                  </div>
                  <table className="table" style={{ marginBottom: "var(--space-4)" }}>
                    <thead>
                      <tr>
                        <th>Header</th>
                        <th>Recorded</th>
                        <th>Mock</th>
                        <th style={{ width: 60 }}>Δ</th>
                      </tr>
                    </thead>
                    <tbody>
                      {selectedResult.diff.header_diffs.map((hd) => (
                        <tr key={hd.header}>
                          <td className="mono text-xs">{hd.header}</td>
                          <td className="mono text-xs" style={{ maxWidth: 120 }}>
                            <div className="truncate">{hd.recorded || "—"}</div>
                          </td>
                          <td className="mono text-xs" style={{ maxWidth: 120 }}>
                            <div className="truncate">{hd.mock || "—"}</div>
                          </td>
                          <td>
                            <span
                              className="badge"
                              style={{
                                background: `${diffTypeColor(hd.diff_type)}22`,
                                color: diffTypeColor(hd.diff_type),
                              }}
                            >
                              {hd.diff_type === "Unchanged" ? "=" : hd.diff_type}
                            </span>
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>

                  {/* Body diff */}
                  {selectedResult.diff.body_diff && (
                    <div>
                      <div className="card-title" style={{ marginBottom: "var(--space-2)" }}>
                        Body
                      </div>
                      <div style={{
                        display: "grid",
                        gridTemplateColumns: "1fr 1fr",
                        gap: "var(--space-2)",
                        fontSize: "var(--text-xs)",
                      }}>
                        <div>
                          <div className="text-xs text-muted" style={{ marginBottom: 4 }}>Recorded</div>
                          <pre style={{
                            background: "var(--bg-primary)",
                            padding: "var(--space-2)",
                            borderRadius: "var(--radius-md)",
                            whiteSpace: "pre-wrap",
                            wordBreak: "break-all",
                            maxHeight: 200,
                            overflowY: "auto",
                          }}>
                            {formatBody(selectedResult.diff.body_diff.recorded) || "(empty)"}
                          </pre>
                        </div>
                        <div>
                          <div className="text-xs text-muted" style={{ marginBottom: 4 }}>Mock</div>
                          <pre style={{
                            background: "var(--bg-primary)",
                            padding: "var(--space-2)",
                            borderRadius: "var(--radius-md)",
                            whiteSpace: "pre-wrap",
                            wordBreak: "break-all",
                            maxHeight: 200,
                            overflowY: "auto",
                          }}>
                            {formatBody(selectedResult.diff.body_diff.mock) || "(empty)"}
                          </pre>
                        </div>
                      </div>

                      {/* Line-by-line diff */}
                      {selectedResult.diff.body_diff.line_diffs.length > 0 && (
                        <div style={{ marginTop: "var(--space-3)" }}>
                          <div className="text-xs text-muted" style={{ marginBottom: 4 }}>
                            Line Diff
                          </div>
                          <div style={{
                            background: "var(--bg-primary)",
                            borderRadius: "var(--radius-md)",
                            padding: "var(--space-2)",
                            fontFamily: "var(--font-mono)",
                            fontSize: "var(--text-xs)",
                            maxHeight: 200,
                            overflowY: "auto",
                          }}>
                            {selectedResult.diff.body_diff.line_diffs.map((ld, i) => (
                              <div
                                key={i}
                                style={{
                                  padding: "1px var(--space-2)",
                                  background:
                                    ld.diff_type === "Added" ? "rgba(62,207,142,0.1)"
                                    : ld.diff_type === "Removed" ? "rgba(231,111,81,0.1)"
                                    : ld.diff_type === "Modified" ? "rgba(244,211,94,0.1)"
                                    : "transparent",
                                  borderLeft: ld.diff_type !== "Unchanged"
                                    ? `3px solid ${diffTypeColor(ld.diff_type)}`
                                    : "3px solid transparent",
                                }}
                              >
                                <span style={{ color: "var(--text-muted)", marginRight: 8 }}>
                                  {ld.diff_type === "Added" ? "+" : ld.diff_type === "Removed" ? "-" : " "}
                                </span>
                                {ld.recorded_text || ld.mock_text || ""}
                              </div>
                            ))}
                          </div>
                        </div>
                      )}
                    </div>
                  )}

                  {!selectedResult.diff.has_changes && (
                    <div className="text-sm" style={{ color: "var(--accent-green)" }}>
                      ✓ No differences — mock matches recorded response exactly
                    </div>
                  )}
                </div>
              ) : (
                <div className="text-sm text-muted">No diff data available</div>
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
