import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "../ui/Button";
import { Badge } from "../ui/Badge";
import { Tabs } from "../ui/Tabs";
import { ErrorBoundary } from "../ui/error-boundary";
import { SkeletonTable } from "../ui/skeleton";
import { Brain, BarChart3, Lock, Eye, Download } from "lucide-react";

// ============================================================
// Types
// ============================================================

interface AiStatsData {
  stats: AiStatRow[];
}

interface AiStatRow {
  provider: string;
  model: string;
  total_tokens: number;
  prompt_tokens: number;
  completion_tokens: number;
  cost_usd: number;
  requests: number;
}

interface InferredApi {
  id: number;
  session_id: string;
  name: string;
  method: string;
  path: string;
  params: string;
  auth_required: boolean;
  score: number | null;
  created_at: string;
}

interface AuthStateMachine {
  device_id: number | null;
  states: { name: string; is_initial: boolean; is_terminal: boolean }[];
  transitions: { from: string; to: string; label: string }[];
  mermaid_md: string;
  anomalies: { description: string; severity: string }[];
}

interface VisionAnalysis {
  id: number;
  session_id: string;
  filename: string;
  components: VisionComponent[];
  raw_response: string;
  score: number;
  created_at: string;
}

interface VisionComponent {
  component_type: string;
  text?: string;
  position: { x: number; y: number; width: number; height: number };
  children: VisionComponent[];
}

// ============================================================
// AI Panel
// ============================================================

export function AiPage() {
  const [activeTab, setActiveTab] = useState("token");

  const tabs = [
    { id: "token", label: "Token Usage" },
    { id: "inference", label: "API Inference" },
    { id: "auth", label: "Auth Flow" },
    { id: "vision", label: "Vision" },
  ];

  return (
    <div className="p-6">
      <h1 className="text-2xl font-bold mb-6 flex items-center gap-2">
        <Brain size={24} className="text-accent-purple" />
        AI Analysis
      </h1>

      <div className="panel">
        <Tabs tabs={tabs} activeTab={activeTab} onTabChange={setActiveTab} />
        <div className="panel-body">
          <ErrorBoundary>
            {activeTab === "token" && <TokenUsageTab />}
            {activeTab === "inference" && <ApiInferenceTab />}
            {activeTab === "auth" && <AuthFlowTab />}
            {activeTab === "vision" && <VisionTab />}
          </ErrorBoundary>
        </div>
      </div>
    </div>
  );
}

// ============================================================
// Token Usage Tab
// ============================================================

function TokenUsageTab() {
  const [data, setData] = useState<AiStatRow[]>([]);
  const [contextWindows, setContextWindows] = useState<Record<string, number>>({});
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    load();
  }, []);

  async function load() {
    try {
      setLoading(true);
      setError(null);
      const [statsResult, cwResult] = await Promise.allSettled([
        invoke<AiStatsData>("get_ai_stats"),
        invoke<Record<string, number>>("get_ai_context_windows"),
      ]);

      if (statsResult.status === "fulfilled" && statsResult.value?.stats) {
        setData(statsResult.value.stats);
      }
      if (cwResult.status === "fulfilled") {
        setContextWindows(cwResult.value);
      }

      const errors: string[] = [];
      if (statsResult.status === "rejected") errors.push("Stats: " + String(statsResult.reason));
      if (cwResult.status === "rejected") errors.push("Context: " + String(cwResult.reason));
      if (errors.length) setError(errors.join("; "));
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }

  function formatTokens(n: number): string {
    if (n >= 1_000_000) return (n / 1_000_000).toFixed(1) + "M";
    if (n >= 1_000) return (n / 1_000).toFixed(1) + "K";
    return String(n);
  }

  function formatCost(n: number): string {
    return "$" + n.toFixed(4);
  }

  return (
    <div>
      <div className="flex items-center justify-between mb-4">
        <span className="text-sm text-text-muted">
          {data.length} models tracked
        </span>
        <Button variant="secondary" size="sm" onClick={load}>Refresh</Button>
      </div>

      {error && (
        <div className="error-banner mb-4">
          <span className="error-banner-message">{error}</span>
        </div>
      )}

      {loading ? (
        <SkeletonTable rows={4} />
      ) : data.length === 0 ? (
        <div className="empty-state">
          <BarChart3 size={48} className="empty-state-icon" />
          <div className="empty-state-title">No LLM token data</div>
          <div className="empty-state-description">
            Token usage is tracked when AI providers (OpenAI, Anthropic, etc.)
            are detected in intercepted traffic.
          </div>
        </div>
      ) : (
        <table className="table">
          <thead>
            <tr>
              <th>Provider</th>
              <th>Model</th>
              <th>Total Tokens</th>
              <th>Prompt</th>
              <th>Completion</th>
              <th>Cost</th>
              <th>Requests</th>
              <th>Context Window</th>
            </tr>
          </thead>
          <tbody>
            {data.map((row) => (
              <tr key={`${row.provider}-${row.model}`}>
                <td className="text-sm font-medium">{row.provider}</td>
                <td className="mono text-xs">{row.model}</td>
                <td className="mono text-sm">{formatTokens(row.total_tokens)}</td>
                <td className="text-xs text-text-muted">{formatTokens(row.prompt_tokens)}</td>
                <td className="text-xs text-text-muted">{formatTokens(row.completion_tokens)}</td>
                <td className="mono text-sm" style={{ color: "var(--accent-green)" }}>
                  {formatCost(row.cost_usd)}
                </td>
                <td className="text-sm">{row.requests}</td>
                <td className="text-xs text-text-muted">
                  {contextWindows[row.model]
                    ? formatTokens(contextWindows[row.model])
                    : "—"}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}

// ============================================================
// API Inference Tab
// ============================================================

function ApiInferenceTab() {
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
    </div>
  );
}

// ============================================================
// Auth Flow Tab
// ============================================================

function AuthFlowTab() {
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
                    <td><Badge variant="info">{t.from}</Badge></td>
                    <td><Badge variant="info">{t.to}</Badge></td>
                    <td className="text-sm">{t.label}</td>
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
                  <Badge variant={a.severity === "critical" ? "critical" : "warning"}>
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

// ============================================================
// Vision Tab
// ============================================================

function VisionTab() {
  const [sessionId, setSessionId] = useState("");
  const [analyses, setAnalyses] = useState<VisionAnalysis[]>([]);
  const [uploading, setUploading] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [fuseResult, setFuseResult] = useState<string | null>(null);

  async function loadAnalyses() {
    if (!sessionId) return;
    try {
      setLoading(true);
      setError(null);
      const result = await invoke<VisionAnalysis[]>("get_vision_analyses", {
        session_id: sessionId,
      });
      setAnalyses(result);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }

  async function handleUpload(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    if (!file || !sessionId) return;

    try {
      setUploading(true);
      setError(null);
      const reader = new FileReader();
      const base64 = await new Promise<string>((resolve, reject) => {
        reader.onload = () => {
          const result = reader.result as string;
          resolve(result.split(",")[1]); // strip data:image/...;base64,
        };
        reader.onerror = reject;
        reader.readAsDataURL(file);
      });

      await invoke("analyze_screenshot_base64", {
        session_id: sessionId,
        image_data_base64: base64,
        filename: file.name,
      });
      loadAnalyses();
    } catch (err) {
      setError(String(err));
    } finally {
      setUploading(false);
    }
  }

  async function handleFuse() {
    if (!sessionId) return;
    try {
      setLoading(true);
      const result = await invoke<string>("fuse_vision_with_api", {
        session_id: sessionId,
      });
      setFuseResult(result);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }

  async function handleDelete(id: number) {
    try {
      await invoke("delete_vision_analysis", { id });
      loadAnalyses();
    } catch (err) {
      setError(String(err));
    }
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
        <label className="btn btn-primary cursor-pointer">
          <Eye size={16} />
          {uploading ? "Uploading..." : "Upload Screenshot"}
          <input
            type="file"
            accept="image/*"
            onChange={handleUpload}
            className="hidden"
            disabled={uploading || !sessionId}
          />
        </label>
        <Button variant="secondary" size="sm" onClick={async () => {
          const path = prompt("Enter screenshot file path:");
          if (path && sessionId) {
            try { setUploading(true); setError(null); await invoke("analyze_screenshot", { session_id: sessionId, image_path: path }); loadAnalyses(); }
            catch (err) { setError(String(err)); }
            finally { setUploading(false); }
          }
        }} disabled={!sessionId}>
          From Path
        </Button>
        <Button variant="secondary" size="sm" onClick={loadAnalyses} disabled={!sessionId}>
          Load Analyses
        </Button>
        <Button variant="secondary" size="sm" onClick={handleFuse} disabled={!sessionId}>
          Fuse with API
        </Button>
      </div>

      {error && <div className="error-banner mb-4"><span className="error-banner-message">{error}</span></div>}

      {fuseResult && (
        <div className="card mb-4" style={{ borderColor: "var(--accent-purple)" }}>
          <div className="card-header">
            <span className="card-title">Fused Component Tree</span>
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
            {fuseResult}
          </pre>
        </div>
      )}

      {loading ? (
        <SkeletonTable rows={3} />
      ) : analyses.length > 0 ? (
        <div style={{ maxHeight: 400, overflowY: "auto" }}>
          {analyses.map((a) => (
            <div key={a.id} className="card mb-3">
              <div className="flex items-center justify-between mb-2">
                <div>
                  <span className="text-sm font-medium">{a.filename}</span>
                  <span className="text-xs text-text-muted ml-3">
                    Score: {(a.score * 100).toFixed(0)}%
                  </span>
                </div>
                <Button variant="ghost" size="sm" onClick={() => handleDelete(a.id)}>✕</Button>
              </div>
              <div className="flex flex-wrap gap-1">
                {a.components.slice(0, 10).map((c, i) => (
                  <span
                    key={i}
                    className="badge"
                    style={{
                      background: "rgba(155,93,229,0.15)",
                      color: "var(--accent-purple)",
                    }}
                  >
                    {c.component_type}
                  </span>
                ))}
                {a.components.length > 10 && (
                  <span className="text-xs text-text-muted">+{a.components.length - 10} more</span>
                )}
              </div>
            </div>
          ))}
        </div>
      ) : (
        <div className="empty-state">
          <Eye size={48} className="empty-state-icon" />
          <div className="empty-state-title">No vision analyses</div>
          <div className="empty-state-description">
            Upload a screenshot of a mobile app to analyze its UI components.
          </div>
        </div>
      )}
    </div>
  );
}
