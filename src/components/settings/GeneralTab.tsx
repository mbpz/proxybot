import { useState, useEffect } from "react";
import { desktop, type DesktopContract } from "../../desktop/contract";
import type { DbStats } from "../../generated/desktop-contract";
import { Button } from "../ui/Button";
import { Power, Smartphone, Database } from "lucide-react";

interface GeneralTabProps {
  contract?: DesktopContract;
}

type PendingAction = "keep-running" | "dashboard" | null;

export function GeneralTab({ contract = desktop }: GeneralTabProps) {
  const [keepRunning, setKeepRunning] = useState(false);
  const [dashboardRunning, setDashboardRunning] = useState(false);
  const [dashboardUrl, setDashboardUrl] = useState("");
  const [dbStats, setDbStats] = useState<DbStats | null>(null);
  const [loading, setLoading] = useState(true);
  const [pendingAction, setPendingAction] = useState<PendingAction>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void load();
  }, [contract]);

  async function load() {
    setLoading(true);
    setError(null);
    try {
      const [keep, running, url, stats] = await Promise.all([
        contract.call("get_keep_running", {}),
        contract.call("is_dashboard_running", {}),
        contract.call("get_dashboard_url", {}),
        contract.call("get_db_stats", {}),
      ]);
      setKeepRunning(keep);
      setDashboardRunning(running);
      setDashboardUrl(url);
      setDbStats(stats);
    } catch (cause) {
      setError(errorMessage("Could not load general settings", cause));
    } finally {
      setLoading(false);
    }
  }

  async function toggleKeepRunning() {
    const next = !keepRunning;
    setPendingAction("keep-running");
    setError(null);
    try {
      await contract.call("set_keep_running", { keep: next });
      setKeepRunning(next);
    } catch (cause) {
      setError(errorMessage("Could not update Keep Running", cause));
    } finally {
      setPendingAction(null);
    }
  }

  async function toggleDashboard() {
    setPendingAction("dashboard");
    setError(null);
    try {
      if (dashboardRunning) {
        await contract.call("stop_dashboard", {});
        setDashboardRunning(false);
        setDashboardUrl("");
      } else {
        const url = await contract.call("start_dashboard", {});
        setDashboardRunning(true);
        setDashboardUrl(url);
      }
    } catch (cause) {
      setError(errorMessage("Could not update Mobile Dashboard", cause));
    } finally {
      setPendingAction(null);
    }
  }

  if (loading) {
    return <div className="p-4 text-text-muted">Loading...</div>;
  }

  return (
    <div className="space-y-4">
      {error && (
        <div className="error-banner" role="alert">
          <span className="error-banner-message">{error}</span>
          <Button variant="secondary" size="sm" onClick={() => void load()}>
            Retry
          </Button>
        </div>
      )}

      {/* Keep Running */}
      <div className="card">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <Power size={20} className="text-text-secondary" />
            <div>
              <div className="font-medium text-text-primary">Keep Running</div>
              <div className="text-sm text-text-muted">
                Continue running in background when window is closed
              </div>
            </div>
          </div>
          <button
            type="button"
            aria-label="Keep running after window closes"
            aria-pressed={keepRunning}
            disabled={pendingAction !== null}
            onClick={() => void toggleKeepRunning()}
            className={`relative w-12 h-6 rounded-full transition-colors ${
              keepRunning ? "bg-accent-green" : "bg-surface-tertiary"
            }`}
          >
            <span
              className={`absolute top-0.5 w-5 h-5 rounded-full bg-white transition-transform shadow-sm ${
                keepRunning ? "translate-x-6" : "translate-x-0.5"
              }`}
            />
          </button>
        </div>
      </div>

      {/* Mobile Dashboard */}
      <div className="card">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <Smartphone size={20} className="text-text-secondary" />
            <div>
              <div className="font-medium text-text-primary">Mobile Dashboard</div>
              <div className="text-sm text-text-muted">
                Start a web dashboard accessible from your phone
              </div>
              {dashboardRunning && dashboardUrl && (
                <div className="text-xs font-mono text-accent-blue mt-1">
                  {dashboardUrl}
                </div>
              )}
            </div>
          </div>
          <Button
            variant={dashboardRunning ? "danger" : "primary"}
            size="sm"
            disabled={pendingAction !== null}
            onClick={() => void toggleDashboard()}
          >
            {dashboardRunning ? "Stop" : "Start"}
          </Button>
        </div>
      </div>

      {/* DB Statistics */}
      {dbStats && (
        <div className="card">
          <div className="flex items-center gap-3 mb-4">
            <Database size={20} className="text-text-secondary" />
            <div className="font-medium text-text-primary">Database</div>
          </div>
          <div className="grid grid-cols-2 gap-4">
            <div className="text-center p-3 rounded-md" style={{ background: "var(--bg-primary)" }}>
              <div className="text-2xl font-bold text-accent-blue">{dbStats.http_requests_count.toLocaleString()}</div>
              <div className="text-xs text-text-muted mt-1">HTTP Requests</div>
            </div>
            <div className="text-center p-3 rounded-md" style={{ background: "var(--bg-primary)" }}>
              <div className="text-2xl font-bold text-accent-green">{dbStats.dns_queries_count.toLocaleString()}</div>
              <div className="text-xs text-text-muted mt-1">DNS Queries</div>
            </div>
            <div className="text-center p-3 rounded-md" style={{ background: "var(--bg-primary)" }}>
              <div className="text-2xl font-bold text-accent-yellow">{dbStats.devices_count.toLocaleString()}</div>
              <div className="text-xs text-text-muted mt-1">Devices</div>
            </div>
            <div className="text-center p-3 rounded-md" style={{ background: "var(--bg-primary)" }}>
              <div className="text-2xl font-bold text-accent-purple">{dbStats.app_tags_count.toLocaleString()}</div>
              <div className="text-xs text-text-muted mt-1">App Tags</div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

function errorMessage(context: string, cause: unknown): string {
  const detail = cause instanceof Error ? cause.message : String(cause);
  return `${context}: ${detail}`;
}
