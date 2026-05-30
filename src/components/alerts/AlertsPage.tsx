import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Badge } from "../ui/Badge";
import { Button } from "../ui/Button";
import { Tabs } from "../ui/Tabs";
import { ErrorBoundary } from "../ui/error-boundary";
import { SkeletonTable } from "../ui/skeleton";

interface Alert {
  id: number;
  device_id: number | null;
  severity: "Info" | "Warning" | "Critical";
  alert_type: string;
  details: string;
  created_at: string;
  acknowledged: boolean;
}

interface BaselineEntry {
  value: string;
  count: number;
  first_seen: string;
  last_seen: string;
}

interface TrafficBaseline {
  device_id: number | null;
  domains: BaselineEntry[];
  ips: BaselineEntry[];
}

export function AlertsPage() {
  const [alerts, setAlerts] = useState<Alert[]>([]);
  const [baseline, setBaseline] = useState<TrafficBaseline | null>(null);
  const [unackedCount, setUnackedCount] = useState(0);
  const [severityFilter, setSeverityFilter] = useState<string>("all");
  const [activeTab, setActiveTab] = useState("alerts");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadAlerts = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const [result, count] = await Promise.all([
        invoke<Alert[]>("get_alerts", {
          severity: severityFilter === "all" ? null : severityFilter.toLowerCase(),
          limit: 100,
        }),
        invoke<number>("get_alert_count"),
      ]);
      setAlerts(result);
      setUnackedCount(count);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, [severityFilter]);

  const loadBaseline = useCallback(async () => {
    try {
      const result = await invoke<TrafficBaseline>("get_traffic_baseline", { device_id: null });
      setBaseline(result);
    } catch (err) {
      console.error("Failed to load baseline:", err);
    }
  }, []);

  // Alerts reload when severity filter changes
  useEffect(() => {
    loadAlerts();
  }, [loadAlerts]);

  // Baseline loads once on mount
  useEffect(() => {
    loadBaseline();
  }, [loadBaseline]);

  async function acknowledgeAlert(id: number) {
    try {
      await invoke("acknowledge_alert", { alert_id: id });
      loadAlerts();
    } catch (err) {
      setError(String(err));
    }
  }

  function severityBadge(severity: string) {
    switch (severity) {
      case "Critical": return <Badge variant="critical">CRITICAL</Badge>;
      case "Warning": return <Badge variant="warning">WARNING</Badge>;
      default: return <Badge variant="info">INFO</Badge>;
    }
  }

  function formatTime(dateStr: string): string {
    const d = new Date(dateStr);
    return d.toLocaleString("en-US", {
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
      hour12: false,
    });
  }

  const tabs = [
    { id: "alerts", label: `Alerts${unackedCount > 0 ? ` (${unackedCount})` : ""}` },
    { id: "baseline", label: "Baseline" },
  ];

  return (
    <div>
      <div className="panel">
        {/* Header */}
        <div className="panel-header">
          <div className="flex items-center gap-3">
            <span className="panel-title">Alerts</span>
            {unackedCount > 0 && (
              <span
                style={{
                  background: "var(--accent-red)",
                  color: "#fff",
                  borderRadius: "50%",
                  width: 20,
                  height: 20,
                  display: "inline-flex",
                  alignItems: "center",
                  justifyContent: "center",
                  fontSize: "var(--text-xs)",
                  fontWeight: 700,
                }}
              >
                {unackedCount}
              </span>
            )}
          </div>
          <Button variant="secondary" size="sm" onClick={() => { loadAlerts(); loadBaseline(); }}>
            Refresh
          </Button>
        </div>

        {/* Error banner */}
        {error && (
          <div className="error-banner" style={{ margin: "0 var(--space-4) var(--space-2)" }}>
            <span className="error-banner-message">{error}</span>
            <Button variant="secondary" size="sm" onClick={loadAlerts}>
              Retry
            </Button>
          </div>
        )}

        {/* Tabs */}
        <Tabs tabs={tabs} activeTab={activeTab} onTabChange={setActiveTab} />

        {/* Content */}
        <div style={{ maxHeight: 500, overflowY: "auto" }}>
          <ErrorBoundary>
            {activeTab === "alerts" && (
              <>
                {/* Severity filter */}
                <div className="flex gap-2" style={{ padding: "var(--space-3) var(--space-4)" }}>
                  {["all", "Info", "Warning", "Critical"].map((sev) => (
                    <button
                      key={sev}
                      className={`btn btn-sm ${severityFilter === sev ? "btn-primary" : "btn-secondary"}`}
                      onClick={() => setSeverityFilter(sev)}
                    >
                      {sev === "all" ? "All" : sev}
                    </button>
                  ))}
                </div>

                {loading ? (
                  <SkeletonTable rows={5} />
                ) : alerts.length === 0 ? (
                  <div className="empty-state">
                    <div className="empty-state-icon">✅</div>
                    <div className="empty-state-title">No alerts</div>
                    <div className="empty-state-description">
                      Anomaly detection will generate alerts when unusual traffic patterns are detected.
                    </div>
                  </div>
                ) : (
                  <table className="table">
                    <thead>
                      <tr>
                        <th style={{ width: 80 }}>Severity</th>
                        <th style={{ width: 100 }}>Type</th>
                        <th>Details</th>
                        <th style={{ width: 120 }}>Time</th>
                        <th style={{ width: 60 }}>Status</th>
                        <th style={{ width: 60 }}>Action</th>
                      </tr>
                    </thead>
                    <tbody>
                      {alerts.map((alert) => (
                        <tr
                          key={alert.id}
                          style={{
                            opacity: alert.acknowledged ? 0.5 : 1,
                            background: !alert.acknowledged && alert.severity === "Critical"
                              ? "rgba(231,111,81,0.05)"
                              : undefined,
                          }}
                        >
                          <td>{severityBadge(alert.severity)}</td>
                          <td className="text-xs" style={{ color: "var(--text-secondary)" }}>
                            {alert.alert_type}
                          </td>
                          <td className="text-sm">{alert.details}</td>
                          <td className="mono text-xs" style={{ color: "var(--text-muted)" }}>
                            {formatTime(alert.created_at)}
                          </td>
                          <td>
                            <span
                              className="text-xs"
                              style={{
                                color: alert.acknowledged ? "var(--accent-green)" : "var(--accent-yellow)",
                              }}
                            >
                              {alert.acknowledged ? "ACK'd" : "New"}
                            </span>
                          </td>
                          <td>
                            {!alert.acknowledged && (
                              <Button
                                variant="ghost"
                                size="sm"
                                onClick={() => acknowledgeAlert(alert.id)}
                              >
                                ACK
                              </Button>
                            )}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                )}
              </>
            )}

            {activeTab === "baseline" && (
              <div>
                {!baseline ? (
                  <SkeletonTable rows={5} />
                ) : (
                  <div style={{ padding: "var(--space-4)" }}>
                    <div className="card" style={{ marginBottom: "var(--space-4)" }}>
                      <div className="card-header">
                        <span className="card-title">Domains ({baseline.domains.length})</span>
                      </div>
                      <div style={{ maxHeight: 200, overflowY: "auto" }}>
                        {baseline.domains.length === 0 ? (
                          <div className="text-sm text-muted" style={{ padding: "var(--space-3)" }}>
                            No domain baseline yet. Start capturing traffic to build one.
                          </div>
                        ) : (
                          <table className="table">
                            <thead>
                              <tr>
                                <th>Domain</th>
                                <th style={{ width: 60 }}>Count</th>
                                <th style={{ width: 120 }}>Last Seen</th>
                              </tr>
                            </thead>
                            <tbody>
                              {baseline.domains.map((d) => (
                                <tr key={d.value}>
                                  <td className="mono text-sm">{d.value}</td>
                                  <td className="text-xs">{d.count}</td>
                                  <td className="mono text-xs" style={{ color: "var(--text-muted)" }}>
                                    {formatTime(d.last_seen)}
                                  </td>
                                </tr>
                              ))}
                            </tbody>
                          </table>
                        )}
                      </div>
                    </div>

                    <div className="card">
                      <div className="card-header">
                        <span className="card-title">IPs ({baseline.ips.length})</span>
                      </div>
                      <div style={{ maxHeight: 200, overflowY: "auto" }}>
                        {baseline.ips.length === 0 ? (
                          <div className="text-sm text-muted" style={{ padding: "var(--space-3)" }}>
                            No IP baseline yet.
                          </div>
                        ) : (
                          <table className="table">
                            <thead>
                              <tr>
                                <th>IP Address</th>
                                <th style={{ width: 60 }}>Count</th>
                                <th style={{ width: 120 }}>Last Seen</th>
                              </tr>
                            </thead>
                            <tbody>
                              {baseline.ips.map((ip) => (
                                <tr key={ip.value}>
                                  <td className="mono text-sm">{ip.value}</td>
                                  <td className="text-xs">{ip.count}</td>
                                  <td className="mono text-xs" style={{ color: "var(--text-muted)" }}>
                                    {formatTime(ip.last_seen)}
                                  </td>
                                </tr>
                              ))}
                            </tbody>
                          </table>
                        )}
                      </div>
                    </div>
                  </div>
                )}
              </div>
            )}
          </ErrorBoundary>
        </div>
      </div>
    </div>
  );
}
