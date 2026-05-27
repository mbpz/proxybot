import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { AiTokenGauge } from "../components/ai/AiTokenGauge";
import { AiUsageTable } from "../components/ai/AiUsageTable";
import type { Alert, AuthStateMachine, VisionAnalysis, ComponentTree } from "../types";

export function AiPage() {
  const [alerts, setAlerts] = useState<Alert[]>([]);
  const [alertCount, setAlertCount] = useState(0);
  const [authStateMachine, setAuthStateMachine] = useState<AuthStateMachine | null>(null);
  const [showStateMachinePanel, setShowStateMachinePanel] = useState(false);

  // Vision state
  const [visionSessionId, setVisionSessionId] = useState("default");
  const [visionAnalyses, setVisionAnalyses] = useState<VisionAnalysis[]>([]);
  const [visionAnalyzing, setVisionAnalyzing] = useState(false);
  const [fusedComponentTree, setFusedComponentTree] = useState<ComponentTree | null>(null);

  // Scaffold state
  const [scaffoldSessionId, setScaffoldSessionId] = useState("default");
  const [scaffoldProjectName, setScaffoldProjectName] = useState("proxybot_frontend");
  const [scaffoldGenerating, setScaffoldGenerating] = useState(false);
  const [scaffoldResult, setScaffoldResult] = useState<any>(null);

  // Deploy state
  const [deploySessionId, setDeploySessionId] = useState("default");
  const [deployProjectName, setDeployProjectName] = useState("proxybot_deployment");
  const [deployGenerating, setDeployGenerating] = useState(false);
  const [deployResult, setDeployResult] = useState<any>(null);

  useEffect(() => {
    invoke<number>("get_alert_count").then(setAlertCount).catch(console.error);
  }, []);

  useEffect(() => {
    invoke<VisionAnalysis[]>("get_vision_analyses", { sessionId: visionSessionId })
      .then(setVisionAnalyses)
      .catch(console.error);
  }, [visionSessionId]);

  const loadAlerts = async () => {
    try {
      setAlerts(await invoke<Alert[]>("get_alerts_cmd", { deviceId: null, severity: null, limit: 50 }));
    } catch (e) { console.error("Failed to load alerts:", e); }
  };

  const acknowledgeAlert = async (alertId: number) => {
    try {
      await invoke("acknowledge_alert_cmd", { alertId });
      await loadAlerts();
      setAlertCount(await invoke<number>("get_alert_count"));
    } catch (e) { console.error("Failed to acknowledge alert:", e); }
  };

  const loadAuthStateMachine = async () => {
    try {
      const machine = await invoke<AuthStateMachine>("get_auth_state_machine", { deviceId: null });
      setAuthStateMachine(machine);
      setShowStateMachinePanel(true);
    } catch (e) { console.error("Failed to load auth state machine:", e); }
  };

  const analyzeScreenshot = async (file: File) => {
    try {
      setVisionAnalyzing(true);
      const arrayBuffer = await file.arrayBuffer();
      const base64 = btoa(new Uint8Array(arrayBuffer).reduce((data, byte) => data + String.fromCharCode(byte), ""));
      const result = await invoke<VisionAnalysis>("analyze_screenshot_base64", {
        sessionId: visionSessionId, imageDataBase64: base64, filename: file.name,
      });
      setVisionAnalyses(prev => [result, ...prev]);
    } catch (e) { alert(String(e)); } finally { setVisionAnalyzing(false); }
  };

  const fuseVisionWithApi = async () => {
    try {
      setFusedComponentTree(await invoke<ComponentTree>("fuse_vision_with_api", { sessionId: visionSessionId }));
    } catch (e) { alert(String(e)); }
  };

  const deleteVisionAnalysis = async (id: number) => {
    try {
      await invoke("delete_vision_analysis", { id });
      setVisionAnalyses(prev => prev.filter(a => a.id !== id));
    } catch (e) { alert(String(e)); }
  };

  const generateScaffold = async () => {
    try {
      setScaffoldGenerating(true); setScaffoldResult(null);
      setScaffoldResult(await invoke<any>("generate_scaffold_project", { sessionId: scaffoldSessionId, projectName: scaffoldProjectName }));
    } catch (e) { alert(String(e)); } finally { setScaffoldGenerating(false); }
  };

  const writeScaffold = async () => {
    try {
      setScaffoldGenerating(true);
      const path = await invoke<string>("write_scaffold_project", { sessionId: scaffoldSessionId, projectName: scaffoldProjectName, outputDir: null });
      alert(`Scaffold project written to:\n${path}`);
    } catch (e) { alert(String(e)); } finally { setScaffoldGenerating(false); }
  };

  const evaluateScaffold = async () => {
    if (!scaffoldResult?.base_path) { alert("Generate a scaffold first."); return; }
    try {
      setScaffoldGenerating(true);
      await invoke<any>("evaluate_scaffold_project", { projectPath: scaffoldResult.base_path, sessionId: scaffoldSessionId });
    } catch (e) { alert(String(e)); } finally { setScaffoldGenerating(false); }
  };

  const generateScaffoldWithVision = async () => {
    if (!fusedComponentTree) { alert("Run 'Fuse with Traffic' first."); return; }
    try {
      setScaffoldGenerating(true);
      setScaffoldResult(await invoke<any>("generate_scaffold_with_vision", {
        sessionId: scaffoldSessionId, name: scaffoldProjectName, visionJson: JSON.stringify(fusedComponentTree),
      }));
    } catch (e) { alert(String(e)); } finally { setScaffoldGenerating(false); }
  };

  const writeScaffoldWithVision = async () => {
    if (!fusedComponentTree) { alert("Run 'Fuse with Traffic' first."); return; }
    try {
      setScaffoldGenerating(true);
      const result = await invoke<any>("generate_scaffold_with_vision", {
        sessionId: scaffoldSessionId, name: scaffoldProjectName, visionJson: JSON.stringify(fusedComponentTree),
      });
      setScaffoldResult(result);
      const path = await invoke<string>("write_scaffold_project_with_vision", { project: result, outputDir: null });
      alert(`Vision-enhanced scaffold written to:\n${path}`);
    } catch (e) { alert(String(e)); } finally { setScaffoldGenerating(false); }
  };

  const generateDeployment = async () => {
    try {
      setDeployGenerating(true);
      setDeployResult(await invoke<any>("generate_deployment_bundle", { sessionId: deploySessionId, projectName: deployProjectName }));
    } catch (e) { alert(String(e)); } finally { setDeployGenerating(false); }
  };

  const writeDeployment = async () => {
    try {
      setDeployGenerating(true);
      const result = await invoke<any>("write_deployment_bundle", { sessionId: deploySessionId, projectName: deployProjectName, outputDir: null });
      setDeployResult(result);
      alert(`Deployment bundle written to:\n${result.bundle_path}\n\nTo run:\n  cd ${result.bundle_path}\n  docker compose up --build`);
    } catch (e) { alert(String(e)); } finally { setDeployGenerating(false); }
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-4)" }}>
      {/* Alerts */}
      <div className="panel">
        <div className="panel-header">
          <span className="panel-title">Alerts</span>
          {alertCount > 0 && <span className="badge badge-warning">{alertCount}</span>}
          <div style={{ display: "flex", gap: "var(--space-2)" }}>
            <button className="btn btn-sm btn-secondary" onClick={loadAlerts}>Refresh</button>
            <button className="btn btn-sm btn-secondary" onClick={loadAuthStateMachine}>State Machine</button>
          </div>
        </div>
        {alerts.length === 0 ? (
          <div className="panel-body">
            <div className="empty-state">
              <div className="empty-state-icon">✅</div>
              <div className="empty-state-title">No alerts</div>
              <div className="empty-state-description">Alerts are generated when anomalies are detected.</div>
            </div>
          </div>
        ) : (
          <div style={{ maxHeight: 200, overflowY: "auto" }}>
            {alerts.map((alert) => (
              <div key={alert.id} style={{ padding: "var(--space-3)", borderBottom: "1px solid var(--border)", display: "flex", gap: "var(--space-3)", alignItems: "flex-start" }}>
                <span className={`badge ${alert.severity === "Critical" ? "badge-critical" : alert.severity === "Warning" ? "badge-warning" : "badge-info"}`}>{alert.severity}</span>
                <div style={{ flex: 1 }}>
                  <div className="text-sm">{alert.alert_type}</div>
                  <div className="text-xs text-muted">{alert.details}</div>
                </div>
                {!alert.acknowledged && <button className="btn btn-sm btn-ghost" onClick={() => acknowledgeAlert(alert.id)}>Ack</button>}
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Vision */}
      <div className="panel">
        <div className="panel-header"><span className="panel-title">Vision Screenshot Analyzer</span></div>
        <div className="panel-body">
          <div style={{ display: "flex", gap: "var(--space-3)", marginBottom: "var(--space-4)", flexWrap: "wrap", alignItems: "center" }}>
            <input type="text" value={visionSessionId} onChange={(e) => setVisionSessionId(e.target.value)} placeholder="Session ID" style={{ width: 140 }} />
            <div style={{ display: "flex", gap: "var(--space-2)", alignItems: "center" }}>
              <input type="file" accept="image/*" onChange={(e) => e.target.files?.[0] && analyzeScreenshot(e.target.files[0])} disabled={visionAnalyzing} id="screenshot-upload" style={{ display: "none" }} />
              <label htmlFor="screenshot-upload" className="btn btn-sm btn-secondary" style={{ cursor: "pointer" }}>
                {visionAnalyzing ? "Analyzing..." : "Upload Screenshot"}
              </label>
            </div>
            <button className="btn btn-sm btn-secondary" onClick={fuseVisionWithApi} disabled={visionAnalyses.length === 0}>Fuse with Traffic</button>
            {fusedComponentTree && (
              <span className="badge badge-success" style={{ fontSize: "0.7rem" }}>
                {fusedComponentTree.components.length} components · {fusedComponentTree.suggested_routes.length} routes
              </span>
            )}
          </div>
          {visionAnalyses.length > 0 && (
            <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-2)" }}>
              {visionAnalyses.map((a) => (
                <div key={a.id} style={{ display: "flex", gap: "var(--space-3)", padding: "var(--space-2)", borderBottom: "1px solid var(--border)", alignItems: "center" }}>
                  <span className="text-sm truncate" style={{ flex: 1 }}>{a.filename}</span>
                  <span className="text-xs text-muted">{a.components.length} components</span>
                  <button className="btn btn-sm btn-ghost" onClick={() => deleteVisionAnalysis(a.id)}>×</button>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>

      {/* AI Token Tracking */}
      <div className="panel">
        <div className="panel-header">
          <span className="panel-title">AI Token Usage</span>
          <span className="text-xs text-muted">OpenAI / Anthropic / Azure / Google / Cohere / Groq</span>
        </div>
        <div className="panel-body" style={{ padding: "var(--space-3)" }}><AiTokenGauge /></div>
      </div>

      {/* AI Usage Table */}
      <div className="panel">
        <div className="panel-header"><span className="panel-title">AI Usage Details</span></div>
        <div className="panel-body" style={{ padding: "var(--space-3)", maxHeight: 300, overflowY: "auto" }}><AiUsageTable /></div>
      </div>

      {/* Scaffold */}
      <div className="panel">
        <div className="panel-header"><span className="panel-title">Scaffold Generator</span></div>
        <div className="panel-body">
          <div style={{ display: "flex", gap: "var(--space-3)", marginBottom: "var(--space-3)", flexWrap: "wrap" }}>
            <input type="text" value={scaffoldSessionId} onChange={(e) => setScaffoldSessionId(e.target.value)} placeholder="Session ID" style={{ width: 140 }} />
            <input type="text" value={scaffoldProjectName} onChange={(e) => setScaffoldProjectName(e.target.value)} placeholder="Project name" style={{ width: 180 }} />
            <button className="btn btn-sm btn-primary" onClick={generateScaffold} disabled={scaffoldGenerating}>{scaffoldGenerating ? "..." : "Generate"}</button>
            <button className="btn btn-sm btn-secondary" onClick={writeScaffold} disabled={scaffoldGenerating}>Write</button>
            <button className="btn btn-sm btn-secondary" onClick={evaluateScaffold} disabled={scaffoldGenerating || !scaffoldResult}>Eval</button>
            <button className="btn btn-sm btn-primary" onClick={generateScaffoldWithVision} disabled={scaffoldGenerating || !fusedComponentTree}>{scaffoldGenerating ? "..." : "Vision Scaffold"}</button>
            <button className="btn btn-sm btn-secondary" onClick={writeScaffoldWithVision} disabled={scaffoldGenerating || !fusedComponentTree}>Write Vision</button>
          </div>
          {scaffoldResult && <div className="text-xs text-muted">Files: {Object.keys(scaffoldResult.files || {}).length} — {scaffoldResult.components?.length || 0} components</div>}
        </div>
      </div>

      {/* Deploy */}
      <div className="panel">
        <div className="panel-header"><span className="panel-title">Docker Deployment</span></div>
        <div className="panel-body">
          <div style={{ display: "flex", gap: "var(--space-3)", marginBottom: "var(--space-3)", flexWrap: "wrap" }}>
            <input type="text" value={deploySessionId} onChange={(e) => setDeploySessionId(e.target.value)} placeholder="Session ID" style={{ width: 140 }} />
            <input type="text" value={deployProjectName} onChange={(e) => setDeployProjectName(e.target.value)} placeholder="Project name" style={{ width: 180 }} />
            <button className="btn btn-sm btn-primary" onClick={generateDeployment} disabled={deployGenerating}>{deployGenerating ? "..." : "Generate"}</button>
            <button className="btn btn-sm btn-secondary" onClick={writeDeployment} disabled={deployGenerating}>Write</button>
          </div>
          {deployResult && <div className="text-xs text-muted">Bundle: {deployResult.bundle_path}</div>}
        </div>
      </div>

      {/* State machine modal */}
      {showStateMachinePanel && authStateMachine && (
        <div style={{ position: "fixed", top: 0, left: 0, right: 0, bottom: 0, background: "rgba(0,0,0,0.6)", display: "flex", alignItems: "center", justifyContent: "center", zIndex: 100 }}>
          <div className="panel" style={{ width: 700, maxHeight: "80vh", display: "flex", flexDirection: "column" }}>
            <div className="panel-header">
              <span className="panel-title">Auth Flow State Machine</span>
              <button className="btn btn-sm btn-ghost" onClick={() => setShowStateMachinePanel(false)}>×</button>
            </div>
            <div className="panel-body" style={{ overflowY: "auto", flex: 1 }}>
              {authStateMachine.anomalies.length > 0 && (
                <div style={{ marginBottom: "var(--space-4)" }}>
                  <div className="card-title" style={{ marginBottom: "var(--space-2)" }}>Anomalies ({authStateMachine.anomalies.length})</div>
                  {authStateMachine.anomalies.map((a, i) => (
                    <div key={i} style={{ display: "flex", gap: "var(--space-2)", padding: "var(--space-1) 0", fontSize: "var(--text-xs)" }}>
                      <span className={`badge ${a.severity === "Critical" ? "badge-critical" : a.severity === "Warning" ? "badge-warning" : "badge-info"}`}>{a.severity}</span>
                      <span>{a.description}</span>
                    </div>
                  ))}
                </div>
              )}
              <pre className="mono" style={{ fontSize: "var(--text-xs)", whiteSpace: "pre-wrap" }}>{authStateMachine.mermaid_md}</pre>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
