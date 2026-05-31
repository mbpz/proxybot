import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { AlertTriangle, Bell, BellOff } from "lucide-react";

interface AlertItem {
  id: number;
  device_id: number | null;
  severity: "Info" | "Warning" | "Critical";
  alert_type: string;
  details: string;
  created_at: string;
  acknowledged: boolean;
}

function formatTimeAgo(dateStr: string): string {
  const date = new Date(dateStr);
  const now = Date.now();
  const diffMs = now - date.getTime();
  const diffMins = Math.floor(diffMs / (1000 * 60));

  if (diffMins < 1) return "just now";
  if (diffMins < 60) return `${diffMins}m ago`;
  const diffHours = Math.floor(diffMins / 60);
  if (diffHours < 24) return `${diffHours}h ago`;
  const diffDays = Math.floor(diffHours / 24);
  return `${diffDays}d ago`;
}

function getSeverityColor(severity: "Info" | "Warning" | "Critical"): string {
  switch (severity) {
    case "Critical":
      return "var(--accent-red)";
    case "Warning":
      return "var(--accent-yellow)";
    case "Info":
    default:
      return "var(--accent-blue)";
  }
}

interface AlertItemProps {
  alert: AlertItem;
}

function AlertItemComponent({ alert }: AlertItemProps) {
  const color = getSeverityColor(alert.severity);

  return (
    <div
      className="
        flex items-start gap-3
        bg-surface-secondary border border-border rounded-lg
        p-3 hover:border-accent-cyan/40 transition-all duration-200
      "
    >
      {/* Status dot */}
      <div
        className="w-2.5 h-2.5 rounded-full mt-1.5 flex-shrink-0"
        style={{
          background: color,
          boxShadow: `0 0 8px ${color}`,
        }}
      />

      {/* Content */}
      <div className="flex-1 min-w-0">
        <div className="text-sm text-primary leading-snug">
          {alert.details}
        </div>
        <div className="text-xs text-muted mt-1">
          {formatTimeAgo(alert.created_at)}
        </div>
      </div>
    </div>
  );
}

export function AlertsPage() {
  const [alerts, setAlerts] = useState<AlertItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [alertsEnabled, setAlertsEnabled] = useState(true);

  useEffect(() => {
    loadAlerts();
  }, []);

  async function loadAlerts() {
    try {
      setLoading(true);
      setError(null);
      const result = await invoke<AlertItem[]>("get_alerts");
      setAlerts(result);
    } catch (err) {
      console.error("Failed to load alerts:", err);
      setError(err instanceof Error ? err.message : "Failed to load alerts");
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="panel">
      {/* Header */}
      <div className="panel-header">
        <div className="flex items-center gap-2">
          <Bell size={18} className="text-text-secondary" />
          <span className="panel-title">Alerts</span>
          {alerts.length > 0 && (
            <span className="text-sm text-muted">
              {alerts.length} active
            </span>
          )}
        </div>

        {/* Toggle All button */}
        <button
          onClick={() => setAlertsEnabled(!alertsEnabled)}
          className={`
            flex items-center gap-1.5 px-2 py-1 rounded
            text-xs font-medium transition-all duration-200
            ${alertsEnabled
              ? "bg-accent-red/20 border border-accent-red/50 text-accent-red"
              : "bg-bg-tertiary border border-border text-text-muted"
            }
          `}
          style={{ minWidth: 52 }}
        >
          {alertsEnabled ? (
            <>
              <AlertTriangle size={12} />
             <span>ON</span>
            </>
          ) : (
            <>
              <BellOff size={12} />
              <span>OFF</span>
            </>
          )}
        </button>
      </div>

      {/* Error banner */}
      {error && (
        <div className="error-banner mx-4 mt-2">
          <span className="error-banner-message">{error}</span>
          <button className="btn btn-sm btn-secondary" onClick={loadAlerts}>
            Retry
          </button>
        </div>
      )}

      {/* Alerts List */}
      <div className="p-3" style={{ maxHeight: 400, overflowY: "auto" }}>
        {loading ? (
          <div className="space-y-3">
            <div className="skeleton skeleton-row" style={{ height: 60 }} />
            <div className="skeleton skeleton-row" style={{ height: 60 }} />
          </div>
        ) : alerts.length === 0 ? (
          <div className="empty-state">
            <div className="empty-state-icon">
              <Bell size={32} />
            </div>
            <div className="empty-state-title">No alerts</div>
            <div className="empty-state-description">
              System alerts will appear here when detected.
            </div>
          </div>
        ) : (
          <div className="space-y-2">
            {alerts.map((alert) => (
              <AlertItemComponent key={alert.id} alert={alert} />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}