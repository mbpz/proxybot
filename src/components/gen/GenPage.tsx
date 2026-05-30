import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "../ui/Button";
import { Tabs } from "../ui/Tabs";
import { MethodBadge } from "../ui/Badge";
import { ErrorBoundary } from "../ui/error-boundary";

interface MockEndpoint {
  method: string;
  path: string;
  name: string;
  fixtures: { variant_id: string; status: number; body?: string }[];
  conditionals: { condition_field: string; condition_value: string; response_variant_id: string }[];
}

interface MockProject {
  name: string;
  base_path: string;
  endpoints: MockEndpoint[];
  openapi_spec: string;
}

interface DeploymentBundle {
  name: string;
  base_path: string;
  docker_compose_content: string;
  readme_content: string;
  ci_template_content: string;
}

export function GenPage() {
  const [activeTab, setActiveTab] = useState("mock");
  const [sessionId, setSessionId] = useState("");
  const [projectName, setProjectName] = useState("proxybot_gen");
  const [outputDir, setOutputDir] = useState("");

  // Mock state
  const [mockProject, setMockProject] = useState<MockProject | null>(null);
  const [mockLoading, setMockLoading] = useState(false);
  const [mockError, setMockError] = useState<string | null>(null);
  const [mockWriteResult, setMockWriteResult] = useState<string | null>(null);

  // Mock server state
  const [mockServerRunning, setMockServerRunning] = useState(false);
  const [mockServerUrl, setMockServerUrl] = useState<string | null>(null);

  // Deploy state
  const [deployBundle, setDeployBundle] = useState<DeploymentBundle | null>(null);
  const [deployLoading, setDeployLoading] = useState(false);
  const [deployError, setDeployError] = useState<string | null>(null);
  const [deployWriteResult, setDeployWriteResult] = useState<string | null>(null);

  async function loadMockEndpoints() {
    if (!sessionId) return;
    try {
      setMockError(null);
      const endpoints = await invoke<MockEndpoint[]>("get_mock_endpoints", { session_id: sessionId });
      setMockProject({ name: projectName, base_path: "", endpoints, openapi_spec: "" });
    } catch (err) {
      setMockError(String(err));
    }
  }

  async function generateMock() {
    if (!sessionId) {
      setMockError("Session ID is required");
      return;
    }
    setMockLoading(true);
    setMockError(null);
    setMockWriteResult(null);
    try {
      const result = await invoke<MockProject>("generate_mock_project", {
        session_id: sessionId,
        project_name: projectName || null,
      });
      setMockProject(result);
    } catch (err) {
      setMockError(String(err));
    } finally {
      setMockLoading(false);
    }
  }

  async function writeMock(): Promise<string | null> {
    if (!sessionId) return null;
    setMockLoading(true);
    setMockError(null);
    try {
      const result = await invoke<string>("write_mock_project", {
        session_id: sessionId,
        project_name: projectName || null,
        output_dir: outputDir || null,
      });
      setMockWriteResult(result);
      return result;
    } catch (err) {
      setMockError(String(err));
      return null;
    } finally {
      setMockLoading(false);
    }
  }

  async function toggleMockServer() {
    try {
      setMockError(null);
      if (mockServerRunning) {
        setMockServerRunning(false);
        setMockServerUrl(null);
      } else {
        // Ensure project is written first
        let projectPath = mockWriteResult;
        if (!projectPath) {
          projectPath = await writeMock();
        }
        if (projectPath) {
          const msg = await invoke<string>("start_mock_server", {
            project_path: projectPath,
            port: null,
          });
          setMockServerRunning(true);
          setMockServerUrl(msg);
        }
      }
    } catch (err) {
      setMockError(String(err));
    }
  }

  async function generateDeploy() {
    if (!sessionId) {
      setDeployError("Session ID is required");
      return;
    }
    setDeployLoading(true);
    setDeployError(null);
    setDeployWriteResult(null);
    try {
      const result = await invoke<DeploymentBundle>("generate_deployment_bundle", {
        session_id: sessionId,
        project_name: projectName || null,
      });
      setDeployBundle(result);
    } catch (err) {
      setDeployError(String(err));
    } finally {
      setDeployLoading(false);
    }
  }

  async function writeDeploy() {
    if (!sessionId) return;
    setDeployLoading(true);
    setDeployError(null);
    try {
      const result = await invoke<{ success: boolean; bundle_path: string; message: string }>(
        "write_deployment_bundle",
        {
          session_id: sessionId,
          project_name: projectName || null,
          output_dir: outputDir || null,
        }
      );
      setDeployWriteResult(result.message);
    } catch (err) {
      setDeployError(String(err));
    } finally {
      setDeployLoading(false);
    }
  }

  const tabs = [
    { id: "mock", label: "Mock API" },
    { id: "scaffold", label: "Scaffold" },
    { id: "deploy", label: "Deploy" },
  ];

  return (
    <div>
      <div className="panel">
        <div className="panel-header">
          <span className="panel-title">Generate</span>
        </div>

        {/* Common inputs */}
        <div className="panel-body">
          <div className="flex items-end gap-3" style={{ flexWrap: "wrap", marginBottom: "var(--space-4)" }}>
            <div className="flex flex-col gap-1" style={{ minWidth: 200 }}>
              <label className="text-xs text-secondary font-mono">Session ID</label>
              <input
                type="text"
                value={sessionId}
                onChange={(e) => setSessionId(e.target.value)}
                placeholder="e.g. session_001"
              />
            </div>
            <div className="flex flex-col gap-1" style={{ minWidth: 160 }}>
              <label className="text-xs text-secondary font-mono">Project Name</label>
              <input
                type="text"
                value={projectName}
                onChange={(e) => setProjectName(e.target.value)}
                placeholder="proxybot_gen"
              />
            </div>
            <div className="flex flex-col gap-1" style={{ minWidth: 240 }}>
              <label className="text-xs text-secondary font-mono">Output Dir (optional)</label>
              <input
                type="text"
                value={outputDir}
                onChange={(e) => setOutputDir(e.target.value)}
                placeholder="Default: ~/.proxybot/..."
              />
            </div>
          </div>
        </div>

        {/* Tabs */}
        <Tabs tabs={tabs} activeTab={activeTab} onTabChange={setActiveTab} />

        <div className="panel-body">
          <ErrorBoundary>
            {/* ================ Mock API ================ */}
            {activeTab === "mock" && (
              <div>
                <div className="flex gap-2" style={{ marginBottom: "var(--space-4)" }}>
                  <Button
                    variant="primary"
                    onClick={generateMock}
                    disabled={mockLoading || !sessionId}
                  >
                    {mockLoading ? "Generating..." : "Generate Mock API"}
                  </Button>
                  <Button variant="secondary" onClick={loadMockEndpoints} disabled={!sessionId}>
                    Load Endpoints
                  </Button>
                  {mockProject && (
                    <>
                      <Button variant="secondary" onClick={() => writeMock()} disabled={mockLoading}>
                        Write to Disk
                      </Button>
                      <Button
                        variant={mockServerRunning ? "danger" : "secondary"}
                        onClick={toggleMockServer}
                        disabled={mockLoading}
                      >
                        {mockServerRunning ? "Stop Server" : "Start Server"}
                      </Button>
                    </>
                  )}
                </div>

                {mockError && (
                  <div className="error-banner" style={{ marginBottom: "var(--space-4)" }}>
                    <span className="error-banner-message">{mockError}</span>
                  </div>
                )}

                {mockWriteResult && (
                  <div
                    style={{
                      padding: "var(--space-3)",
                      background: "rgba(62,207,142,0.1)",
                      border: "1px solid var(--accent-green)",
                      borderRadius: "var(--radius-md)",
                      color: "var(--accent-green)",
                      marginBottom: "var(--space-4)",
                      fontSize: "var(--text-sm)",
                    }}
                  >
                    ✓ {mockWriteResult}
                  </div>
                )}

                {mockServerUrl && (
                  <div style={{
                    padding: "var(--space-3)", background: "rgba(77,157,224,0.1)",
                    border: "1px solid var(--accent-blue)", borderRadius: "var(--radius-md)",
                    color: "var(--accent-blue)", marginBottom: "var(--space-4)", fontSize: "var(--text-sm)",
                  }}>
                    🚀 {mockServerUrl}
                  </div>
                )}

                {mockProject ? (
                  <div>
                    <div className="card-title" style={{ marginBottom: "var(--space-3)" }}>
                      Endpoints ({mockProject.endpoints.length})
                    </div>
                    <div style={{ maxHeight: 300, overflowY: "auto" }}>
                      <table className="table">
                        <thead>
                          <tr>
                            <th style={{ width: 70 }}>Method</th>
                            <th>Path</th>
                            <th>Name</th>
                            <th style={{ width: 80 }}>Fixtures</th>
                          </tr>
                        </thead>
                        <tbody>
                          {mockProject.endpoints.map((ep) => (
                            <tr key={`${ep.method}-${ep.path}`}>
                              <td><MethodBadge method={ep.method} /></td>
                              <td className="mono text-xs">{ep.path}</td>
                              <td className="text-sm">{ep.name}</td>
                              <td className="text-xs" style={{ color: "var(--text-muted)" }}>
                                {ep.fixtures.length}
                              </td>
                            </tr>
                          ))}
                        </tbody>
                      </table>
                    </div>

                    {/* OpenAPI spec preview */}
                    <details style={{ marginTop: "var(--space-4)" }}>
                      <summary className="text-sm" style={{ cursor: "pointer", color: "var(--accent-blue)" }}>
                        OpenAPI Spec
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
                        {mockProject.openapi_spec}
                      </pre>
                    </details>
                  </div>
                ) : !mockLoading && (
                  <div className="empty-state">
                    <div className="empty-state-icon">🧪</div>
                    <div className="empty-state-title">Generate Mock API</div>
                    <div className="empty-state-description">
                      Enter a session ID and click Generate to create a FastAPI mock server
                      from recorded traffic.
                    </div>
                  </div>
                )}
              </div>
            )}

            {/* ================ Scaffold ================ */}
            {activeTab === "scaffold" && <ScaffoldTab sessionId={sessionId} projectName={projectName} outputDir={outputDir} />}

            {/* ================ Deploy ================ */}
            {activeTab === "deploy" && (
              <div>
                <div className="flex gap-2" style={{ marginBottom: "var(--space-4)" }}>
                  <Button
                    variant="primary"
                    onClick={generateDeploy}
                    disabled={deployLoading || !sessionId}
                  >
                    {deployLoading ? "Generating..." : "Generate Deployment Bundle"}
                  </Button>
                  {deployBundle && (
                    <Button variant="secondary" onClick={writeDeploy} disabled={deployLoading}>
                      Write to Disk + Git Init
                    </Button>
                  )}
                </div>

                {deployError && (
                  <div className="error-banner" style={{ marginBottom: "var(--space-4)" }}>
                    <span className="error-banner-message">{deployError}</span>
                  </div>
                )}

                {deployWriteResult && (
                  <div
                    style={{
                      padding: "var(--space-3)",
                      background: "rgba(62,207,142,0.1)",
                      border: "1px solid var(--accent-green)",
                      borderRadius: "var(--radius-md)",
                      color: "var(--accent-green)",
                      marginBottom: "var(--space-4)",
                      fontSize: "var(--text-sm)",
                      whiteSpace: "pre-wrap",
                    }}
                  >
                    {deployWriteResult}
                  </div>
                )}

                {deployBundle ? (
                  <div>
                    {/* Docker Compose preview */}
                    <div className="card-title" style={{ marginBottom: "var(--space-2)" }}>
                      docker-compose.yml
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
                      wordBreak: "break-all",
                    }}>
                      {deployBundle.docker_compose_content}
                    </pre>

                    {/* CI template preview */}
                    <details style={{ marginTop: "var(--space-4)" }}>
                      <summary className="text-sm" style={{ cursor: "pointer", color: "var(--accent-blue)" }}>
                        GitHub Actions CI
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
                        {deployBundle.ci_template_content}
                      </pre>
                    </details>
                  </div>
                ) : !deployLoading && (
                  <div className="empty-state">
                    <div className="empty-state-icon">🐳</div>
                    <div className="empty-state-title">Deployment Bundle</div>
                    <div className="empty-state-description">
                      Generate a complete Docker Compose deployment with mock API,
                      frontend scaffold, PostgreSQL, and GitHub Actions CI.
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

// ============================================================
// Scaffold Tab (separate component to avoid cluttering GenPage)
// ============================================================

interface ScaffoldProject {
  name: string;
  files: { path: string; content: string }[];
}

function ScaffoldTab({ sessionId, projectName, outputDir }: {
  sessionId: string;
  projectName: string;
  outputDir: string;
}) {
  const [project, setProject] = useState<ScaffoldProject | null>(null);
  const [evalScore, setEvalScore] = useState<number | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [writeResult, setWriteResult] = useState<string | null>(null);

  async function generate() {
    if (!sessionId) { setError("Session ID is required"); return; }
    try {
      setLoading(true);
      setError(null);
      setWriteResult(null);
      const result = await invoke<ScaffoldProject>("generate_scaffold_project", {
        session_id: sessionId,
        name: projectName || null,
      });
      setProject(result);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }

  async function write() {
    if (!sessionId) return;
    try {
      setLoading(true);
      setError(null);
      const result = await invoke<string>("write_scaffold_project", {
        session_id: sessionId,
        name: projectName || null,
        dir: outputDir || null,
      });
      setWriteResult(result);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }

  async function evaluate() {
    if (!sessionId) return;
    try {
      setLoading(true);
      const defaultPath = `~/.proxybot/scaffolds/${projectName || "proxybot_gen"}`;
      const result = await invoke<[boolean, number, string[]]>("evaluate_scaffold_project", {
        session_id: sessionId,
        path: outputDir || defaultPath,
      });
      const [valid, score, _errors] = result;
      setEvalScore(valid ? score : score * 0.5);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }

  return (
    <div>
      <div className="flex gap-2 mb-4 flex-wrap">
        <Button variant="primary" onClick={generate} disabled={loading || !sessionId}>
          {loading ? "Generating..." : "Generate Scaffold"}
        </Button>
        <Button variant="secondary" onClick={async () => {
          if (!sessionId) return;
          try { setLoading(true); setError(null);
            const r = await invoke<ScaffoldProject>("generate_scaffold_with_vision", { session_id: sessionId, name: projectName || null });
            setProject(r);
          } catch (err) { setError(String(err)); }
          finally { setLoading(false); }
        }} disabled={loading || !sessionId}>
          +Vision
        </Button>
        {project && (
          <>
            <Button variant="secondary" onClick={write} disabled={loading}>
              Write to Disk
            </Button>
            <Button variant="secondary" onClick={async () => {
              if (!project) return;
              try { setLoading(true);
                const r = await invoke<string>("write_scaffold_project_with_vision", { project, output_dir: outputDir || null });
                setWriteResult(r);
              } catch (err) { setError(String(err)); }
              finally { setLoading(false); }
            }} disabled={loading}>
              Write+Vision
            </Button>
            <Button variant="secondary" onClick={evaluate} disabled={loading}>
              Evaluate
            </Button>
          </>
        )}
      </div>

      {error && <div className="error-banner mb-4"><span className="error-banner-message">{error}</span></div>}

      {writeResult && (
        <div style={{
          padding: "var(--space-3)", background: "rgba(62,207,142,0.1)",
          border: "1px solid var(--accent-green)", borderRadius: "var(--radius-md)",
          color: "var(--accent-green)", marginBottom: "var(--space-4)", fontSize: "var(--text-sm)",
        }}>
          ✓ {writeResult}
        </div>
      )}

      {evalScore != null && (
        <div className="card mb-4" style={{ borderColor: evalScore >= 0.8 ? "var(--accent-green)" : "var(--accent-yellow)" }}>
          <span className="text-sm">Scaffold Score: </span>
          <span className="text-lg font-bold" style={{ color: evalScore >= 0.8 ? "var(--accent-green)" : "var(--accent-yellow)" }}>
            {(evalScore * 100).toFixed(0)}%
          </span>
        </div>
      )}

      {project ? (
        <div>
          <div className="card-title mb-3">Files ({project.files?.length || 0})</div>
          <div style={{ maxHeight: 400, overflowY: "auto" }}>
            <table className="table">
              <thead><tr><th>File</th><th style={{ width: 80 }}>Size</th></tr></thead>
              <tbody>
                {project.files?.map((f) => (
                  <tr key={f.path}>
                    <td className="mono text-xs">{f.path}</td>
                    <td className="text-xs text-text-muted">{f.content.length} B</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      ) : !loading && (
        <div className="empty-state">
          <div className="empty-state-icon">🏗️</div>
          <div className="empty-state-title">Frontend Scaffold</div>
          <div className="empty-state-description">
            Generate a React + TypeScript frontend scaffold from inferred APIs.
            Run API inference first, then click Generate.
          </div>
        </div>
      )}
    </div>
  );
}
