import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "../ui/Button";
import { Power, Smartphone, Database } from "lucide-react";

export function GeneralTab() {
  const [keepRunning, setKeepRunning] = useState(false);
  const [dashboardRunning, setDashboardRunning] = useState(false);
  const [dashboardUrl, setDashboardUrl] = useState("");
  const [dbStats, setDbStats] = useState<{ http_requests_count: number; dns_queries_count: number; devices_count: number; app_tags_count: number } | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    load();
  }, []);

  async function load() {
    try {
      const [keep, running, url, stats] = await Promise.all([
        invoke<boolean>("get_keep_running"),
        invoke<boolean>("is_dashboard_running"),
        invoke<string>("get_dashboard_url").catch(() => ""),
        invoke<{ http_requests_count: number; dns_queries_count: number; devices_count: number; app_tags_count: number }>("get_db_stats").catch(() => null),
      ]);
      setKeepRunning(keep);
      setDashboardRunning(running);
      setDashboardUrl(url);
      setDbStats(stats);
    } catch (e) {
      console.error("Failed to load general settings:", e);
    } finally {
      setLoading(false);
    }
  }

  async function toggleKeepRunning() {
    try {
      await invoke("set_keep_running", { keep: !keepRunning });
      setKeepRunning(!keepRunning);
    } catch (e) {
      console.error("Failed to toggle keep running:", e);
    }
  }

  async function toggleDashboard() {
    try {
      if (dashboardRunning) {
        await invoke("stop_dashboard");
        setDashboardRunning(false);
        setDashboardUrl("");
      } else {
        const url = await invoke<string>("start_dashboard");
        setDashboardRunning(true);
        setDashboardUrl(url);
      }
    } catch (e) {
      console.error("Failed to toggle dashboard:", e);
    }
  }

  if (loading) {
    return <div className="p-4 text-text-muted">Loading...</div>;
  }

  return (
    <div className="space-y-4">
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
            onClick={toggleKeepRunning}
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
            onClick={toggleDashboard}
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
