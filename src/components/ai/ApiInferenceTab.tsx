// ============================================================
// API Inference Tab
// ============================================================

import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "../ui/Button";
import { Badge } from "../ui/Badge";
import { SpecGenPanel } from "./SpecGenPanel";
import type { InferredApi, TrafficRecord } from "./types";

export function ApiInferenceTab() {
  const [sessionId, setSessionId] = useState("");
  const [apis, setApis] = useState<InferredApi[]>([]);
  const [openapiSpec, setOpenapiSpec] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function loadInferred() {
    if (!sessionId) {
      setError("Session ID is required");
      return;
    }
    try {
      setLoading(true);
      setError(null);
      const result = await invoke<InferredApi[]>("get_inferred_apis", {
        session_id: sessionId || null,
      });
      setApis(result);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }

  async function triggerInference() {
    if (!sessionId) return;
    try {
      setLoading(true);
      setError(null);
      await invoke("infer_api_semantics", { session_id: sessionId, device_id: null });
      loadInferred();
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }

  async function evaluateInference() {
    if (!sessionId) return;
    try {
      setLoading(true);
      setError(null);
      const result = await invoke<{ valid: boolean; errors: string[]; score: number }>(
        "evaluate_inference", { session_id: sessionId }
      );
      setError(result.valid ? null : result.errors.join("; "));
      alert(`Score: ${(result.score * 100).toFixed(0)}% — ${result.valid ? "Valid" : "Issues found"}`);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }

  async function exportYaml() {
    if (!sessionId) return;
    try {
      const yaml = await invoke<string>("generate_openapi_yaml", { session_id: sessionId });
      await navigator.clipboard.writeText(yaml);
      alert("OpenAPI YAML copied to clipboard");
    } catch (err) {
      setError(String(err));
    }
  }

  async function loadOpenApi() {
    if (!sessionId) return;
    try {
      setLoading(true);
      setError(null);
      const spec = await invoke<string>("get_openapi_spec", { session_id: sessionId });
      setOpenapiSpec(spec);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }

  function methodVariant(m: string): "get" | "post" | "put" | "delete" | "patch" | "info" {
    const v = m.toLowerCase();
    return ["get", "post", "put", "delete", "patch"].includes(v)
      ? (v as "get" | "post" | "put" | "delete" | "patch")
      : "info";
  }

  return (
    <div>
      <div className="flex items-end gap-3 mb-4 flex-wrap">
        <div className="flex flex-col gap-1" style={{ minWidth: 200 }}>
          <label className="text-xs text-text-muted font-mono">Session ID</label>
          <input
            type="text"
            value={sessionId}
            onChange={(e) => setSessionId(e.target.value)}
            placeholder="session_001"
          />
        </div>
        <Button variant="primary" size="sm" onClick={triggerInference} disabled={loading || !sessionId}>
          Infer APIs
        </Button>
        <Button variant="secondary" size="sm" onClick={loadInferred} disabled={loading}>
          Load APIs
        </Button>
        <Button variant="secondary" size="sm" onClick={evaluateInference} disabled={loading || !sessionId}>
          Evaluate
        </Button>
        <Button variant="secondary" size="sm" onClick={loadOpenApi} disabled={loading || !sessionId}>
          JSON Spec
        </Button>
        <Button variant="secondary" size="sm" onClick={exportYaml} disabled={loading || !sessionId}>
          YAML Export
        </Button>
        <Button variant="secondary" size="sm" onClick={async () => {
          if (!sessionId) return;
          try { await invoke("store_inference_result", { session_id: sessionId, inference: { interfaces: [], modules: [], valid: true, errors: [], score: 1.0 } }); alert("Stored"); }
          catch (err) { setError(String(err)); }
        }} disabled={loading || !sessionId}>
          Store
        </Button>
      </div>

      {error && <div className="error-banner mb-4"><span className="error-banner-message">{error}</span></div>}

      {apis.length > 0 && (
        <div style={{ maxHeight: 300, overflowY: "auto", marginBottom: "var(--space-4)" }}>
          <table className="table">
            <thead>
              <tr>
                <th style={{ width: 60 }}>Method</th>
                <th>Path</th>
                <th>Name</th>
                <th style={{ width: 60 }}>Auth</th>
                <th style={{ width: 50 }}>Score</th>
              </tr>
            </thead>
            <tbody>
              {apis.map((api) => (
                <tr key={api.id}>
                  <td><Badge variant={methodVariant(api.method)}>{api.method}</Badge></td>
                  <td className="mono text-xs">{api.path}</td>
                  <td className="text-sm">{api.name}</td>
                  <td>{api.auth_required ? "🔒" : "—"}</td>
                  <td className="text-xs" style={{
                    color: (api.score ?? 0) >= 0.8 ? "var(--accent-green)" : "var(--accent-yellow)",
                  }}>
                    {api.score != null ? (api.score * 100).toFixed(0) + "%" : "—"}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {openapiSpec && (
        <details>
          <summary className="text-sm cursor-pointer" style={{ color: "var(--accent-blue)" }}>
            OpenAPI 3.1 Spec
          </summary>
          <pre style={{
            background: "var(--bg-primary)",
            padding: "var(--space-3)",
            borderRadius: "var(--radius-md)",
            fontSize: "var(--text-xs)",
            fontFamily: "var(--font-mono)",
            maxHeight: 300,
            overflowY: "auto",
            whiteSpace: "pre-wrap",
            wordBreak: "break-all",
            marginTop: "var(--space-2)",
          }}>
            {openapiSpec}
          </pre>
        </details>
      )}

      {!loading && apis.length === 0 && !openapiSpec && (
        <div className="empty-state">
          <div className="empty-state-icon">🔍</div>
          <div className="empty-state-title">API Inference</div>
          <div className="empty-state-description">
            Enter a session ID and click "Load APIs" to see inferred API endpoints.
          </div>
        </div>
      )}

      <SpecGenPanel
        sessionId={sessionId}
        trafficRecords={[] as TrafficRecord[]}
        onError={setError}
      />
    </div>
  );
}
