// ============================================================
// Auth Flow Tab
// ============================================================

import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "../ui/Button";
import { Badge } from "../ui/Badge";
import { SkeletonTable } from "../ui/skeleton";
import { Lock, Download } from "lucide-react";
import type { AuthStateMachine } from "./types";

export function AuthFlowTab() {
  const [machine, setMachine] = useState<AuthStateMachine | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function load() {
    try {
      setLoading(true);
      setError(null);
      const result = await invoke<AuthStateMachine>("get_auth_state_machine", {
        device_id: null,
      });
      setMachine(result);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => { load(); }, []);

  return (
    <div>
      <div className="flex items-center justify-between mb-4">
        <span className="text-sm text-text-muted">
          {machine ? `${machine.states.length} states, ${machine.transitions.length} transitions` : ""}
        </span>
        <Button variant="secondary" size="sm" onClick={load}>Refresh</Button>
      </div>

      {error && <div className="error-banner mb-4"><span className="error-banner-message">{error}</span></div>}

      {loading ? (
        <SkeletonTable rows={3} />
      ) : machine ? (
        <div>
          {/* Mermaid diagram */}
          {machine.mermaid_md && (
            <div className="card mb-4">
              <div className="card-header">
                <span className="card-title">State Machine</span>
                <Button variant="ghost" size="sm" onClick={() => {
                  navigator.clipboard.writeText(machine.mermaid_md);
                }}>
                  <Download size={14} /> Copy Mermaid
                </Button>
              </div>
              <pre style={{
                background: "var(--bg-primary)",
                padding: "var(--space-3)",
                borderRadius: "var(--radius-md)",
                fontSize: "var(--text-xs)",
                fontFamily: "var(--font-mono)",
                maxHeight: 300,
                overflowY: "auto",
                whiteSpace: "pre-wrap",
              }}>
                {machine.mermaid_md}
              </pre>
            </div>
          )}

          {/* Transitions table */}
          <div className="card mb-4">
            <div className="card-header"><span className="card-title">Transitions</span></div>
            <table className="table">
              <thead>
                <tr><th>From</th><th>To</th><th>Label</th></tr>
              </thead>
              <tbody>
                {machine.transitions.map((t, i) => (
                  <tr key={i}>
                    <td><Badge variant="info">{t.from_state}</Badge></td>
                    <td><Badge variant="info">{t.to_state}</Badge></td>
                    <td className="text-sm">{t.method} {t.path}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          {/* Anomalies */}
          {machine.anomalies.length > 0 && (
            <div className="card">
              <div className="card-header"><span className="card-title">Anomalies</span></div>
              {machine.anomalies.map((a, i) => (
                <div key={i} className="flex items-start gap-2 py-2" style={{ borderBottom: "1px solid var(--border)" }}>
                  <Badge variant={a.severity === "Critical" ? "critical" : "warning"}>
                    {a.severity}
                  </Badge>
                  <span className="text-sm">{a.description}</span>
                </div>
              ))}
            </div>
          )}
        </div>
      ) : (
        <div className="empty-state">
          <Lock size={48} className="empty-state-icon" />
          <div className="empty-state-title">No auth flow data</div>
          <div className="empty-state-description">
            Capture traffic with login sequences to generate auth state machine.
          </div>
        </div>
      )}
    </div>
  );
}
