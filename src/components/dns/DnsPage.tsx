import { useState, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { AppBadge } from "../ui/Badge";
import { ErrorBoundary } from "../ui/error-boundary";
import { SkeletonTable } from "../ui/skeleton";
import { safeInvokeOr } from "../../utils/safeInvoke";

interface DnsEntry {
  domain: string;
  timestamp_ms: number;
  app_name?: string;
  app_icon?: string;
  response_ips?: string[];
  query_type?: string;
}

export function DnsPage() {
  const [queries, setQueries] = useState<DnsEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [dnsUpstream, setDnsUpstream] = useState<string>("");

  useEffect(() => {
    loadDnsLog();
    loadDnsUpstream();

    // Subscribe to real-time DNS queries
    const unlisten = listen<DnsEntry>("dns-query", (event) => {
      setQueries((prev) => [event.payload, ...prev].slice(0, 500));
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  async function loadDnsLog() {
    try {
      setLoading(true);
      setError(null);
      const result = await safeInvokeOr<DnsEntry[]>("get_dns_log", []);
      setQueries(result);
    } catch (err) {
      console.error("Failed to load DNS log:", err);
    } finally {
      setLoading(false);
    }
  }

  async function loadDnsUpstream() {
    const upstream = await safeInvokeOr<string>("get_dns_upstream", "");
    setDnsUpstream(upstream);
  }

  function formatTime(timestamp_ms: number): string {
    const d = new Date(timestamp_ms);
    return d.toLocaleTimeString("en-US", {
      hour12: false,
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });
  }

  return (
    <div className="panel">
      {/* Header */}
      <div className="panel-header">
        <div className="flex items-center gap-3">
          <span className="panel-title">DNS Queries</span>
          {dnsUpstream && (
            <span
              className="text-xs font-mono"
              style={{
                background: "var(--bg-elevated)",
                padding: "var(--space-1) var(--space-2)",
                borderRadius: "var(--radius-sm)",
                color: "var(--text-muted)",
              }}
            >
              {dnsUpstream}
            </span>
          )}
        </div>
        <span className="text-sm text-muted">{queries.length} entries</span>
      </div>

      {/* Error banner */}
      {error && (
        <div className="error-banner mx-4 mt-2">
          <span className="error-banner-message">{error}</span>
          <button className="btn btn-sm btn-secondary" onClick={loadDnsLog}>
            Retry
          </button>
        </div>
      )}

      {/* Content */}
      <div style={{ maxHeight: 600, overflowY: "auto" }}>
        <ErrorBoundary>
          {loading ? (
            <SkeletonTable rows={8} />
          ) : queries.length === 0 ? (
            <div className="empty-state">
              <div className="empty-state-icon">🌐</div>
              <div className="empty-state-title">No DNS queries</div>
              <div className="empty-state-description">
                Enable transparent proxy to start capturing DNS queries
              </div>
            </div>
          ) : (
            <table className="table">
              <thead>
                <tr>
                  <th style={{ width: 120 }}>App</th>
                  <th style={{ width: 100 }}>Time</th>
                  <th>Domain</th>
                  <th style={{ width: 80 }}>Type</th>
                  <th>Response IPs</th>
                </tr>
              </thead>
              <tbody>
                {queries.map((q, idx) => (
                  <tr key={`${q.timestamp_ms}-${idx}`}>
                    <td>
                      <AppBadge app={q.app_name || null} />
                    </td>
                    <td className="mono text-xs" style={{ color: "var(--text-muted)" }}>
                      {formatTime(q.timestamp_ms)}
                    </td>
                    <td className="mono text-sm">{q.domain}</td>
                    <td>
                      <span
                        className="text-xs font-mono"
                        style={{
                          background: "var(--bg-elevated)",
                          padding: "2px 6px",
                          borderRadius: "var(--radius-sm)",
                          color: "var(--text-secondary)",
                        }}
                      >
                        {q.query_type || "A"}
                      </span>
                    </td>
                    <td className="mono text-xs" style={{ color: "var(--text-muted)" }}>
                      {q.response_ips?.join(", ") || "-"}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </ErrorBoundary>
      </div>
    </div>
  );
}
