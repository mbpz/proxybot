import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { AppBadge } from "../ui/Badge";
import { Button } from "../ui/Button";
import { ErrorBoundary } from "../ui/error-boundary";
import { SkeletonTable } from "../ui/skeleton";

interface DnsEntry {
  domain: string;
  timestamp_ms: number;
  app_name?: string;
  app_icon?: string;
  action?: string;
  resolved_ips: string[];
}

interface DnsUpstream {
  upstream_type: "plainudp" | "doh";
  address: string;
}

export function DnsPage() {
  const [entries, setEntries] = useState<DnsEntry[]>([]);
  const [upstream, setUpstream] = useState<DnsUpstream | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadData = useCallback(async () => {
    setLoading(true);
    setError(null);

    const [logResult, upResult] = await Promise.allSettled([
      invoke<DnsEntry[]>("get_dns_log"),
      invoke<DnsUpstream>("get_dns_upstream"),
    ]);

    const errors: string[] = [];
    if (logResult.status === "fulfilled") {
      setEntries(logResult.value);
    } else {
      errors.push("DNS log: " + String(logResult.reason));
    }
    if (upResult.status === "fulfilled") {
      setUpstream(upResult.value);
    } else {
      errors.push("Upstream: " + String(upResult.reason));
    }

    if (errors.length > 0) setError(errors.join("; "));
    setLoading(false);
  }, []);

  useEffect(() => {
    loadData();

    const unlisten = listen<DnsEntry>("dns-query", (event) => {
      setEntries((prev) => [event.payload, ...prev].slice(0, 500));
    });

    return () => {
      unlisten.then((f) => f());
    };
  }, [loadData]);

  async function setUpstreamType(type: "plainudp" | "doh") {
    try {
      setError(null);
      // Re-use current address if the type already matches, otherwise use sensible defaults
      const currentType = upstream?.upstream_type;
      const currentAddr = upstream?.address;
      const address = currentType === type && currentAddr
        ? currentAddr
        : type === "doh"
          ? "https://1.1.1.1/dns-query"
          : "8.8.8.8:53";
      await invoke("set_dns_upstream", {
        upstream: { upstream_type: type, address },
      });
      setUpstream({ upstream_type: type, address });
    } catch (err) {
      setError(String(err));
    }
  }

  async function reloadLists() {
    try {
      setError(null);
      await invoke("reload_dns_lists");
    } catch (err) {
      setError(String(err));
    }
  }

  function formatTime(ms: number): string {
    const d = new Date(ms);
    return d.toLocaleTimeString("en-US", { hour12: false }) +
      "." + String(d.getMilliseconds()).padStart(3, "0");
  }

  return (
    <div>
      <div className="panel">
        {/* Header */}
        <div className="panel-header">
          <div className="flex items-center gap-3">
            <span className="panel-title">DNS Queries</span>
            <span className="text-sm text-muted">
              {entries.length} entries
            </span>
            {upstream && (
              <span
                className="badge"
                style={{
                  background: upstream.upstream_type === "doh"
                    ? "rgba(62,207,142,0.15)"
                    : "rgba(77,157,224,0.15)",
                  color: upstream.upstream_type === "doh"
                    ? "var(--accent-green)"
                    : "var(--accent-blue)",
                }}
              >
                {upstream.upstream_type === "doh" ? "DoH" : "UDP"}
              </span>
            )}
          </div>
          <div className="flex gap-2">
            <Button
              variant={upstream?.upstream_type === "doh" ? "primary" : "secondary"}
              size="sm"
              onClick={() => setUpstreamType("doh")}
            >
              DoH
            </Button>
            <Button
              variant={upstream?.upstream_type === "plainudp" ? "primary" : "secondary"}
              size="sm"
              onClick={() => setUpstreamType("plainudp")}
            >
              UDP
            </Button>
            <Button variant="secondary" size="sm" onClick={reloadLists}>
              Reload Lists
            </Button>
            <Button variant="secondary" size="sm" onClick={loadData}>
              Refresh
            </Button>
          </div>
        </div>

        {/* Error banner */}
        {error && (
          <div className="error-banner" style={{ margin: "0 var(--space-4) var(--space-2)" }}>
            <span className="error-banner-message">{error}</span>
            <Button variant="secondary" size="sm" onClick={loadData}>
              Retry
            </Button>
          </div>
        )}

        {/* Content */}
        <div style={{ maxHeight: 500, overflowY: "auto" }}>
          <ErrorBoundary>
            {loading ? (
              <SkeletonTable rows={8} />
            ) : entries.length === 0 ? (
              <div className="empty-state">
                <div className="empty-state-icon">🌐</div>
                <div className="empty-state-title">No DNS queries</div>
                <div className="empty-state-description">
                  DNS queries from connected devices will appear here.
                  Start the proxy and configure your phone gateway to your Mac IP.
                </div>
              </div>
            ) : (
              <table className="table">
                <thead>
                  <tr>
                    <th style={{ width: 100 }}>Time</th>
                    <th>Domain</th>
                    <th style={{ width: 60 }}>Type</th>
                    <th style={{ width: 140 }}>Response IPs</th>
                    <th style={{ width: 80 }}>Action</th>
                    <th style={{ width: 80 }}>App</th>
                  </tr>
                </thead>
                <tbody>
                  {entries.map((entry, i) => (
                    <tr key={`${entry.domain}-${entry.timestamp_ms}-${i}`}>
                      <td className="mono text-xs" style={{ color: "var(--text-muted)" }}>
                        {formatTime(entry.timestamp_ms)}
                      </td>
                      <td className="mono text-sm">{entry.domain}</td>
                      <td className="text-xs" style={{ color: "var(--text-muted)" }}>A</td>
                      <td>
                        <div className="flex flex-wrap gap-1">
                          {entry.resolved_ips.length > 0
                            ? entry.resolved_ips.map((ip) => (
                                <span
                                  key={ip}
                                  className="mono text-xs"
                                  style={{
                                    color: "var(--accent-blue)",
                                    background: "rgba(77,157,224,0.1)",
                                    padding: "1px 4px",
                                    borderRadius: "var(--radius-sm)",
                                  }}
                                >
                                  {ip}
                                </span>
                              ))
                            : (
                              <span className="text-xs" style={{ color: "var(--text-muted)" }}>
                                —
                              </span>
                            )}
                        </div>
                      </td>
                      <td>
                        {entry.action ? (
                          <span
                            className="badge"
                            style={{
                              background:
                                entry.action === "DIRECT" ? "rgba(62,207,142,0.15)"
                                : entry.action === "REJECT" ? "rgba(231,111,81,0.15)"
                                : "rgba(77,157,224,0.15)",
                              color:
                                entry.action === "DIRECT" ? "var(--accent-green)"
                                : entry.action === "REJECT" ? "var(--accent-red)"
                                : "var(--accent-blue)",
                            }}
                          >
                            {entry.action}
                          </span>
                        ) : (
                          <span className="text-xs" style={{ color: "var(--text-muted)" }}>—</span>
                        )}
                      </td>
                      <td>
                        <AppBadge app={entry.app_name || null} />
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </ErrorBoundary>
        </div>
      </div>
    </div>
  );
}
