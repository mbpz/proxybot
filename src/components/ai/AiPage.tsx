import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Badge } from "../ui/Badge";
import { Button } from "../ui/Button";
import { Tabs } from "../ui/Tabs";
import { ErrorBoundary } from "../ui/error-boundary";
import { SkeletonCard } from "../ui/skeleton";

interface Alert {
  id: number;
  severity: string;
  alert_type: string;
  details: string;
  acknowledged: boolean;
  created_at: string;
}

interface AuthState {
  states: string[];
  transitions: { from: string; to: string; label: string }[];
}

type AiTab = "alerts" | "state-machine";

export function AiPage() {
  const [activeTab, setActiveTab] = useState<AiTab>("alerts");
  const [alerts, setAlerts] = useState<Alert[]>([]);
  const [authState, setAuthState] = useState<AuthState | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    loadAlerts();
  }, []);

  async function loadAlerts() {
    try {
      setLoading(true);
      setError(null);
      const result = await invoke<Alert[]>("get_alerts");
      setAlerts(result);
    } catch (err) {
      console.error("Failed to load alerts:", err);
      setError(err instanceof Error ? err.message : "Failed to load alerts");
    } finally {
      setLoading(false);
    }
  }

  async function loadAuthStateMachine() {
    try {
      setLoading(true);
      const result = await invoke<AuthState>("get_auth_state_machine");
      setAuthState(result);
      setActiveTab("state-machine");
    } catch (err) {
      console.error("Failed to load state machine:", err);
    } finally {
      setLoading(false);
    }
  }

  async function acknowledgeAlert(id: number) {
    try {
      await invoke("acknowledge_alert", { id });
      await loadAlerts();
    } catch (err) {
      console.error("Failed to acknowledge alert:", err);
    }
  }

  const tabs = [
    { id: "alerts", label: "Alerts" },
    { id: "state-machine", label: "Auth Flow" },
  ];

  const alertCount = alerts.filter((a) => !a.acknowledged).length;

  return (
    <div>
      <div className="panel">
        {/* Header */}
        <div className="panel-header">
          <div className="flex items-center gap-3">
            <span className="panel-title">AI Analysis</span>
            {alertCount > 0 && <Badge variant="warning">{alertCount} new</Badge>}
          </div>
          <div className="flex gap-2">
            <Button variant="secondary" size="sm" onClick={loadAlerts}>
              Refresh
            </Button>
            <Button variant="secondary" size="sm" onClick={loadAuthStateMachine}>
              Auth Flow
            </Button>
          </div>
        </div>

        {/* Tabs */}
        <Tabs tabs={tabs} activeTab={activeTab} onTabChange={(id) => setActiveTab(id as AiTab)} />

        {/* Content */}
        <div style={{ maxHeight: 500, overflowY: "auto" }}>
          <ErrorBoundary>
            {loading ? (
              <div className="p-4">
                <SkeletonCard />
              </div>
            ) : activeTab === "alerts" ? (
              <AlertsList
                alerts={alerts}
                onAcknowledge={acknowledgeAlert}
                error={error}
                onRetry={loadAlerts}
              />
            ) : (
              <AuthStateMachineView authState={authState} />
            )}
          </ErrorBoundary>
        </div>
      </div>
    </div>
  );
}

function AlertsList({
  alerts,
  onAcknowledge,
  error,
  onRetry,
}: {
  alerts: Alert[];
  onAcknowledge: (id: number) => void;
  error: string | null;
  onRetry: () => void;
}) {
  if (error) {
    return (
      <div className="error-banner m-4">
        <span className="error-banner-message">{error}</span>
        <Button variant="secondary" size="sm" onClick={onRetry}>
          Retry
        </Button>
      </div>
    );
  }

  if (alerts.length === 0) {
    return (
      <div className="empty-state">
        <div className="empty-state-icon">✅</div>
        <div className="empty-state-title">No alerts</div>
        <div className="empty-state-description">
          Alerts are generated when anomalies are detected.
        </div>
      </div>
    );
  }

  return (
    <div>
      {alerts.map((alert) => (
        <div
          key={alert.id}
          className="flex gap-3 items-start px-4 py-3"
          style={{ borderBottom: "1px solid var(--border)" }}
        >
          <Badge
            variant={
              alert.severity === "Critical"
                ? "critical"
                : alert.severity === "Warning"
                ? "warning"
                : "info"
            }
          >
            {alert.severity}
          </Badge>
          <div className="flex-1">
            <div className="text-sm">{alert.alert_type}</div>
            <div className="text-xs text-muted">{alert.details}</div>
          </div>
          {!alert.acknowledged && (
            <button
              className="btn btn-sm btn-ghost"
              onClick={() => onAcknowledge(alert.id)}
            >
              Ack
            </button>
          )}
        </div>
      ))}
    </div>
  );
}

function AuthStateMachineView({
  authState,
}: {
  authState: AuthState | null;
}) {
  if (!authState) {
    return (
      <div className="empty-state">
        <div className="empty-state-icon">🔐</div>
        <div className="empty-state-title">No auth flow data</div>
        <div className="empty-state-description">
          Click "Auth Flow" to analyze authentication patterns.
        </div>
      </div>
    );
  }

  return (
    <div className="p-4">
      <div className="text-sm text-muted mb-4">
        {authState.states.length} states, {authState.transitions.length}{" "}
        transitions
      </div>
      <div
        style={{
          background: "var(--bg-tertiary)",
          borderRadius: "var(--radius-md)",
          padding: "var(--space-4)",
          fontFamily: "var(--font-mono)",
          fontSize: "var(--text-xs)",
          whiteSpace: "pre-wrap",
        }}
      >
        {authState.states.map((state, i) => (
          <div key={i} style={{ marginBottom: "var(--space-2)" }}>
            <span style={{ color: "var(--accent-blue)" }}>[{state}]</span>
            {authState.transitions
              .filter((t) => t.from === state)
              .map((t, j) => (
                <span key={j} style={{ marginLeft: "var(--space-4)" }}>
                  →{" "}
                  <span style={{ color: "var(--accent-green)" }}>{t.to}</span>{" "}
                  <span className="text-muted">({t.label})</span>
                </span>
              ))}
          </div>
        ))}
      </div>
    </div>
  );
}
