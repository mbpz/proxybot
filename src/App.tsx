import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./index.css";

interface InterceptedRequest {
  id: string;
  timestamp: string;
  method: string;
  host: string;
  path: string;
  query_params?: string;
  status: number | null;
  latency_ms: number | null;
  scheme: string;
  req_headers: [string, string][];
  req_body?: string;
  resp_headers: [string, string][];
  resp_body?: string;
  resp_size?: number;
  app_name?: string;
  app_icon?: string;
  is_websocket: boolean;
  ws_frames?: WsFrame[];
}

interface WsFrame {
  direction: string;
  timestamp: string;
  payload: string;
  size: number;
}

type AppTab = "all" | "WeChat" | "Douyin" | "Alipay" | "Unknown";

interface NetworkInfo {
  lan_ip: string;
  interface: string;
}

interface DnsEntry {
  domain: string;
  timestamp_ms: number;
  app_name?: string;
  app_icon?: string;
}

interface CaMetadata {
  created_at: number;
  serial: string;
}

type RulePattern = "DOMAIN" | "DOMAIN-SUFFIX" | "DOMAIN-KEYWORD" | "IP-CIDR" | "GEOIP" | "RULE-SET";
type RuleAction = "DIRECT" | "PROXY" | "REJECT";

interface Rule {
  pattern: RulePattern;
  value: string;
  action: RuleAction;
}

interface DeviceInfo {
  id: number;
  mac_address: string;
  name: string;
  created_at: string;
  last_seen_at: string;
  upload_bytes: number;
  download_bytes: number;
  rule_override: string | null;
}

interface ReplayTarget {
  host: string;
  request_count: number;
  path_count: number;
}

interface ReplayResult {
  request_id: number;
  method: string;
  url: string;
  recorded_response: RecordedResponse;
  mock_response: MockResponse | null;
  diff: DiffResult | null;
  delay_ms: number;
  error: string | null;
}

interface RecordedResponse {
  status: number;
  headers: [string, string][];
  body: string | null;
}

interface MockResponse {
  status: number;
  headers: [string, string][];
  body: string | null;
}

interface DiffResult {
  header_diffs: HeaderDiff[];
  body_diff: BodyDiff | null;
  has_changes: boolean;
}

interface HeaderDiff {
  header: string;
  recorded: string | null;
  mock: string | null;
  diff_type: "Added" | "Removed" | "Modified" | "Unchanged";
}

interface BodyDiff {
  recorded: string | null;
  mock: string | null;
  recorded_lines: string[];
  mock_lines: string[];
  line_diffs: LineDiff[];
}

interface LineDiff {
  line_number_recorded: number | null;
  line_number_mock: number | null;
  recorded_text: string | null;
  mock_text: string | null;
  diff_type: "Added" | "Removed" | "Modified" | "Unchanged";
}

interface Alert {
  id: number;
  device_id: number | null;
  severity: "Info" | "Warning" | "Critical";
  alert_type: string;
  details: string;
  created_at: string;
  acknowledged: boolean;
}

interface AuthState {
  id: string;
  label: string;
  state_type: "Initial" | "Login" | "Authenticated" | "Resource" | "Logout" | "Error";
}

interface AuthTransition {
  from_state: string;
  to_state: string;
  request_id: number;
  method: string;
  path: string;
  token_type: string | null;
  is_anomalous: boolean;
  anomaly_reason: string | null;
}

interface AuthStateMachine {
  device_id: number | null;
  states: AuthState[];
  transitions: AuthTransition[];
  mermaid_md: string;
  anomalies: Anomaly[];
}

interface Anomaly {
  request_id: number;
  anomaly_type: string;
  description: string;
  severity: "Info" | "Warning" | "Critical";
}

function App() {
  const [running, setRunning] = useState(false);
  const [requests, setRequests] = useState<InterceptedRequest[]>([]);
  const [error, setError] = useState("");
  const [networkInfo, setNetworkInfo] = useState<NetworkInfo | null>(null);
  const [pfEnabled, setPfEnabled] = useState(false);
  const [pfLoading, setPfLoading] = useState(false);
  const [tunEnabled, setTunEnabled] = useState(false);
  const [tunLoading, setTunLoading] = useState(false);
  const [dnsQueries, setDnsQueries] = useState<DnsEntry[]>([]);
  const [selectedTab, setSelectedTab] = useState<AppTab>("all");
  const [caMetadata, setCaMetadata] = useState<CaMetadata | null>(null);
  const [selectedHost, setSelectedHost] = useState<string>("all");
  const [keywordFilter, setKeywordFilter] = useState("");
  const [selectedRequest, setSelectedRequest] = useState<InterceptedRequest | null>(null);
  const [rules, setRules] = useState<Rule[]>([]);
  const [ruleFiles, setRuleFiles] = useState<string[]>([]);
  const [selectedRuleFile, setSelectedRuleFile] = useState<string>("rules.yaml");
  const [showRuleEditor, setShowRuleEditor] = useState(false);
  const [editingRule, setEditingRule] = useState<Rule | null>(null);
  const [devices, setDevices] = useState<DeviceInfo[]>([]);
  const [selectedDevice, setSelectedDevice] = useState<DeviceInfo | null>(null);
  const [sessionName, setSessionName] = useState<string>("");
  const [showExportDialog, setShowExportDialog] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [replayTargets, setReplayTargets] = useState<ReplayTarget[]>([]);
  const [selectedReplayHost, setSelectedReplayHost] = useState<string>("");
  const [replayDelay, setReplayDelay] = useState<number>(100);
  const [replayResults, setReplayResults] = useState<ReplayResult[]>([]);
  const [replaying, setReplaying] = useState(false);
  const [authStateMachine, setAuthStateMachine] = useState<AuthStateMachine | null>(null);
  const [alerts, setAlerts] = useState<Alert[]>([]);
  const [alertCount, setAlertCount] = useState(0);
  const [showStateMachinePanel, setShowStateMachinePanel] = useState(false);
  const [scaffoldSessionId, setScaffoldSessionId] = useState<string>("default");
  const [scaffoldProjectName, setScaffoldProjectName] = useState<string>("proxybot_frontend");
  const [scaffoldGenerating, setScaffoldGenerating] = useState(false);
  const [scaffoldResult, setScaffoldResult] = useState<any>(null);
  const [visionSessionId, setVisionSessionId] = useState<string>("default");
  const [visionAnalyses, setVisionAnalyses] = useState<VisionAnalysis[]>([]);
  const [visionAnalyzing, setVisionAnalyzing] = useState(false);
  const [selectedVisionAnalysis, setSelectedVisionAnalysis] = useState<VisionAnalysis | null>(null);
  const [fusedComponentTree, setFusedComponentTree] = useState<ComponentTree | null>(null);
  const [deploySessionId, setDeploySessionId] = useState<string>("default");
  const [deployProjectName, setDeployProjectName] = useState<string>("proxybot_deployment");
  const [deployGenerating, setDeployGenerating] = useState(false);
  const [deployResult, setDeployResult] = useState<any>(null);

  // UI state: which main tab is active
  const [activeTopTab, setActiveTopTab] = useState<"traffic" | "dns" | "rules" | "devices" | "ai">("traffic");
  const [detailTab, setDetailTab] = useState<"headers" | "params" | "body" | "ws">("headers");

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
    text: string | null;
    position: VisionPosition;
    children: VisionComponent[];
  }

  interface VisionPosition {
    x: number;
    y: number;
    width: number;
    height: number;
  }

  interface ComponentTree {
    components: VisionComponent[];
    layout_json: string;
    suggested_routes: string[];
  }

  useEffect(() => {
    invoke<number>("get_alert_count").then(setAlertCount).catch(console.error);
    invoke<CaMetadata | null>("get_ca_metadata")
      .then(setCaMetadata)
      .catch(console.error);
    invoke<NetworkInfo>("get_network_info")
      .then(setNetworkInfo)
      .catch((e) => console.error("Failed to get network info:", e));
    invoke<boolean>("is_pf_enabled")
      .then((enabled) => setPfEnabled(enabled))
      .catch((e) => console.error("Failed to get pf status:", e));
    invoke<boolean>("is_tun_enabled")
      .then((enabled) => setTunEnabled(enabled))
      .catch((e) => console.error("Failed to get TUN status:", e));

    const unlisten = listen<InterceptedRequest>("intercepted-request", (event) => {
      setRequests((prev) => [event.payload, ...prev].slice(0, 100));
    });

    const unlistenDns = listen<DnsEntry>("dns-query", (event) => {
      setDnsQueries((prev) => [event.payload, ...prev].slice(0, 50));
    });

    // Load initial DNS log
    invoke<DnsEntry[]>("get_dns_log")
      .then(setDnsQueries)
      .catch((e) => console.error("Failed to get DNS log:", e));

    // Load devices
    invoke<DeviceInfo[]>("get_devices")
      .then(setDevices)
      .catch((e) => console.error("Failed to get devices:", e));

    return () => {
      unlisten.then((fn) => fn());
      unlistenDns.then((fn) => fn());
    };
  }, []);

  useEffect(() => {
    loadRuleFiles();
    loadRules();
  }, []);

  const loadRuleFiles = async () => {
    try {
      const files = await invoke<string[]>("list_rule_files");
      setRuleFiles(files.length > 0 ? files : ["rules.yaml"]);
      if (files.length > 0 && !files.includes(selectedRuleFile)) {
        setSelectedRuleFile(files[0]);
      }
    } catch (e) {
      console.error("Failed to load rule files:", e);
      setRuleFiles(["rules.yaml"]);
    }
  };

  const loadRules = async () => {
    try {
      const loadedRules = await invoke<Rule[]>("get_rules");
      setRules(loadedRules);
    } catch (e) {
      console.error("Failed to load rules:", e);
    }
  };

  const startProxy = async () => {
    try {
      setError("");
      const result = await invoke<string>("start_proxy");
      console.log(result);
      setRunning(true);
    } catch (e) {
      setError(String(e));
    }
  };

  const enableTransparentProxy = async () => {
    if (!networkInfo) return;
    try {
      setPfLoading(true);
      setError("");

      // Check if pf is already enabled — if so, skip setup to avoid redundant admin prompt
      const alreadyEnabled = await invoke<boolean>("is_pf_enabled");
      if (alreadyEnabled) {
        setPfEnabled(true);
        return;
      }

      // get_network_info populates ProxyState with interface + local_ip
      await invoke<NetworkInfo>("get_network_info");
      // setup_pf reads from ProxyState — no args needed
      const result = await invoke<string>("setup_pf");
      console.log(result);
      setPfEnabled(true);
    } catch (e) {
      setError(String(e));
    } finally {
      setPfLoading(false);
    }
  };

  const disableTransparentProxy = async () => {
    try {
      setPfLoading(true);
      setError("");
      await invoke<void>("teardown_pf");
      setPfEnabled(false);
    } catch (e) {
      setError(String(e));
    } finally {
      setPfLoading(false);
    }
  };

  const enableTunMode = async () => {
    try {
      setTunLoading(true);
      setError("");
      const result = await invoke<string>("setup_tun");
      console.log(result);
      setTunEnabled(true);
    } catch (e) {
      setError(String(e));
    } finally {
      setTunLoading(false);
    }
  };

  const downloadCaCert = async () => {
    try {
      const caPem = await invoke<string>("get_ca_cert_pem");
      await navigator.clipboard.writeText(caPem);
      alert("CA certificate copied to clipboard. Paste it to a file with .pem extension.");
    } catch (e) {
      setError(String(e));
    }
  };

  const saveRule = async (rule: Rule) => {
    try {
      await invoke("save_rule", { rule, filename: selectedRuleFile });
      await loadRules();
      setShowRuleEditor(false);
      setEditingRule(null);
    } catch (e) {
      setError(String(e));
    }
  };

  const deleteRule = async (rule: Rule) => {
    try {
      await invoke("delete_rule", { rule, filename: selectedRuleFile });
      await loadRules();
    } catch (e) {
      setError(String(e));
    }
  };

  const moveRule = async (index: number, direction: "up" | "down") => {
    const newRules = [...rules];
    const targetIndex = direction === "up" ? index - 1 : index + 1;
    if (targetIndex < 0 || targetIndex >= newRules.length) return;
    [newRules[index], newRules[targetIndex]] = [newRules[targetIndex], newRules[index]];
    try {
      await invoke("reorder_rules", { rules: newRules, filename: selectedRuleFile });
      await loadRules();
    } catch (e) {
      setError(String(e));
    }
  };

  const formatTimestamp = (ts: string) => {
    try {
      const parts = ts.split(".");
      const secs = parseInt(parts[0]);
      const ms = parts[1] || "000";
      const date = new Date(secs * 1000);
      return date.toLocaleTimeString() + "." + ms.slice(0, 3);
    } catch {
      return ts;
    }
  };

  const formatSize = (bytes: number): string => {
    if (bytes < 1024) return `${bytes}B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)}KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)}MB`;
  };

  const formatBody = (body: string | undefined, headers: [string, string][]): string => {
    if (!body) return "";
    const contentType = headers.find(([name]) => name.toLowerCase() === "content-type");
    if (contentType && contentType[1].includes("application/json")) {
      try {
        return JSON.stringify(JSON.parse(body), null, 2);
      } catch {
        return body;
      }
    }
    return body;
  };

  const loadDevices = async () => {
    try {
      const deviceList = await invoke<DeviceInfo[]>("get_devices");
      setDevices(deviceList);
    } catch (e) {
      console.error("Failed to load devices:", e);
    }
  };

  const updateDeviceRuleOverride = async (macAddress: string, ruleOverride: string | null) => {
    try {
      await invoke("set_device_rule_override", { macAddress, ruleOverride });
      await loadDevices();
    } catch (e) {
      setError(String(e));
    }
  };

  const exportHar = async () => {
    try {
      setExporting(true);
      const name = sessionName.trim() || `session-${Date.now()}`;
      const har = await invoke<any>("export_har", { sessionName: name });
      const harJson = JSON.stringify(har, null, 2);
      const path = await invoke<string>("save_har_file", { harJson, sessionName: name });
      alert(`HAR file saved to:\n${path}`);
      setShowExportDialog(false);
      setSessionName("");
    } catch (e) {
      setError(String(e));
    } finally {
      setExporting(false);
    }
  };

  const loadReplayTargets = async () => {
    try {
      const targets = await invoke<ReplayTarget[]>("get_replay_targets");
      setReplayTargets(targets);
    } catch (e) {
      console.error("Failed to load replay targets:", e);
    }
  };

  const startReplay = async () => {
    if (!selectedReplayHost) {
      alert("Please select a host to replay");
      return;
    }
    try {
      setReplaying(true);
      setReplayResults([]);
      const results = await invoke<ReplayResult[]>("start_replay", {
        host: selectedReplayHost,
        delayMs: replayDelay,
      });
      setReplayResults(results);
    } catch (e) {
      setError(String(e));
    } finally {
      setReplaying(false);
    }
  };

  useEffect(() => {
    loadReplayTargets();
  }, []);

  const loadAuthStateMachine = async () => {
    try {
      const machine = await invoke<AuthStateMachine>("get_auth_state_machine", {
        deviceId: selectedDevice?.id || null,
      });
      setAuthStateMachine(machine);
      setShowStateMachinePanel(true);
    } catch (e) {
      console.error("Failed to load auth state machine:", e);
    }
  };

  const loadAlerts = async () => {
    try {
      const alertList = await invoke<Alert[]>("get_alerts_cmd", {
        deviceId: null,
        severity: null,
        limit: 50,
      });
      setAlerts(alertList);
    } catch (e) {
      console.error("Failed to load alerts:", e);
    }
  };

  const acknowledgeAlert = async (alertId: number) => {
    try {
      await invoke("acknowledge_alert_cmd", { alertId });
      await loadAlerts();
      const count = await invoke<number>("get_alert_count");
      setAlertCount(count);
    } catch (e) {
      console.error("Failed to acknowledge alert:", e);
    }
  };

  const generateScaffold = async () => {
    try {
      setScaffoldGenerating(true);
      setError("");
      setScaffoldResult(null);
      const result = await invoke<any>("generate_scaffold_project", {
        sessionId: scaffoldSessionId,
        projectName: scaffoldProjectName,
      });
      setScaffoldResult(result);
    } catch (e) {
      setError(String(e));
    } finally {
      setScaffoldGenerating(false);
    }
  };

  const writeScaffold = async () => {
    try {
      setScaffoldGenerating(true);
      setError("");
      const path = await invoke<string>("write_scaffold_project", {
        sessionId: scaffoldSessionId,
        projectName: scaffoldProjectName,
        outputDir: null,
      });
      alert(`Scaffold project written to:\n${path}`);
    } catch (e) {
      setError(String(e));
    } finally {
      setScaffoldGenerating(false);
    }
  };

  const generateScaffoldWithVision = async () => {
    if (!fusedComponentTree) {
      setError("No fused component tree. Run 'Fuse with Traffic' first.");
      return;
    }
    try {
      setScaffoldGenerating(true);
      setError("");
      const result = await invoke<any>("generate_scaffold_with_vision", {
        sessionId: scaffoldSessionId,
        name: scaffoldProjectName,
        visionJson: JSON.stringify(fusedComponentTree),
      });
      setScaffoldResult(result);
    } catch (e) {
      setError(String(e));
    } finally {
      setScaffoldGenerating(false);
    }
  };

  const writeScaffoldWithVision = async () => {
    if (!fusedComponentTree) {
      setError("No fused component tree. Run 'Fuse with Traffic' first.");
      return;
    }
    try {
      setScaffoldGenerating(true);
      setError("");
      // Generate with vision
      const result = await invoke<any>("generate_scaffold_with_vision", {
        sessionId: scaffoldSessionId,
        name: scaffoldProjectName,
        visionJson: JSON.stringify(fusedComponentTree),
      });
      setScaffoldResult(result);
      // Write the pre-generated project to disk
      const path = await invoke<string>("write_scaffold_project_with_vision", {
        project: result,
        outputDir: null,
      });
      alert(`Vision-enhanced scaffold written to:\n${path}`);
    } catch (e) {
      setError(String(e));
    } finally {
      setScaffoldGenerating(false);
    }
  };

  const evaluateScaffold = async () => {
    if (!scaffoldResult?.base_path) {
      setError("Generate a scaffold first before evaluating.");
      return;
    }
    try {
      setScaffoldGenerating(true);
      setError("");
      await invoke<any>("evaluate_scaffold_project", {
        projectPath: scaffoldResult.base_path,
        sessionId: scaffoldSessionId,
      });
    } catch (e) {
      setError(String(e));
    } finally {
      setScaffoldGenerating(false);
    }
  };

  const analyzeScreenshot = async (file: File) => {
    try {
      setVisionAnalyzing(true);
      setError("");
      const arrayBuffer = await file.arrayBuffer();
      const base64 = btoa(
        new Uint8Array(arrayBuffer).reduce((data, byte) => data + String.fromCharCode(byte), "")
      );
      const result = await invoke<VisionAnalysis>("analyze_screenshot_base64", {
        sessionId: visionSessionId,
        imageDataBase64: base64,
        filename: file.name,
      });
      setVisionAnalyses(prev => [result, ...prev]);
      setSelectedVisionAnalysis(result);
    } catch (e) {
      setError(String(e));
    } finally {
      setVisionAnalyzing(false);
    }
  };

  const handleScreenshotUpload = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file) {
      analyzeScreenshot(file);
    }
  };

  const loadVisionAnalyses = async () => {
    try {
      const analyses = await invoke<VisionAnalysis[]>("get_vision_analyses", {
        sessionId: visionSessionId,
      });
      setVisionAnalyses(analyses);
    } catch (e) {
      console.error("Failed to load vision analyses:", e);
    }
  };

  const deleteVisionAnalysis = async (id: number) => {
    try {
      await invoke("delete_vision_analysis", { id });
      setVisionAnalyses(prev => prev.filter(a => a.id !== id));
      if (selectedVisionAnalysis?.id === id) {
        setSelectedVisionAnalysis(null);
      }
    } catch (e) {
      setError(String(e));
    }
  };

  const fuseVisionWithApi = async () => {
    try {
      setError("");
      const result = await invoke<ComponentTree>("fuse_vision_with_api", {
        sessionId: visionSessionId,
      });
      setFusedComponentTree(result);
    } catch (e) {
      setError(String(e));
    }
  };

  useEffect(() => {
    loadVisionAnalyses();
  }, [visionSessionId]);

  const generateDeployment = async () => {
    try {
      setDeployGenerating(true);
      setError("");
      const result = await invoke<any>("generate_deployment_bundle", {
        sessionId: deploySessionId,
        projectName: deployProjectName,
      });
      setDeployResult(result);
    } catch (e) {
      setError(String(e));
    } finally {
      setDeployGenerating(false);
    }
  };

  const writeDeployment = async () => {
    try {
      setDeployGenerating(true);
      setError("");
      const result = await invoke<any>("write_deployment_bundle", {
        sessionId: deploySessionId,
        projectName: deployProjectName,
        outputDir: null,
      });
      setDeployResult(result);
      alert(`Deployment bundle written to:\n${result.bundle_path}\n\nTo run:\n  cd ${result.bundle_path}\n  docker compose up --build`);
    } catch (e) {
      setError(String(e));
    } finally {
      setDeployGenerating(false);
    }
  };

  const formatBytes = (bytes: number): string => {
    if (bytes < 1024) return `${bytes}B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)}KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)}MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)}GB`;
  };

  return (
    <div style={{ minHeight: "100vh", background: "var(--bg-primary)" }}>
      {/* Header */}
      <header style={{
        display: "flex", alignItems: "center", justifyContent: "space-between",
        padding: "var(--space-3) var(--space-4)",
        background: "var(--bg-secondary)", borderBottom: "1px solid var(--border)",
      }}>
        <div style={{ display: "flex", alignItems: "center", gap: "var(--space-3)" }}>
          <h1 style={{ fontSize: "var(--text-lg)", fontWeight: 700, margin: 0 }}>ProxyBot</h1>
          <span style={{
            width: 8, height: 8, borderRadius: "50%",
            background: running ? "var(--accent-green)" : "var(--text-muted)",
          }} />
          <span className="text-sm text-secondary">
            {running ? "Proxy running on :8080" : "Stopped"}
          </span>
        </div>
        <div style={{ display: "flex", gap: "var(--space-2)", alignItems: "center" }}>
          {caMetadata && (
            <span className="text-xs text-muted" style={{ fontFamily: "var(--font-mono)" }}>
              CA: {new Date(caMetadata.created_at * 1000).toLocaleDateString()}
            </span>
          )}
          <button className="btn btn-sm btn-secondary" onClick={downloadCaCert} title="Copy CA cert to clipboard">
            📜 CA
          </button>
          <button
            className={`btn btn-sm ${running ? "btn-danger" : "btn-primary"}`}
            onClick={startProxy}
            disabled={running}
          >
            {running ? "Running" : "Start"}
          </button>
        </div>
      </header>

      {/* Top tab bar */}
      <div className="tabs" style={{ padding: "0 var(--space-4)", background: "var(--bg-secondary)", borderBottom: "1px solid var(--border)" }}>
        {(["traffic", "dns", "rules", "devices", "ai"] as const).map((tab) => (
          <button
            key={tab}
            className={`tab ${activeTopTab === tab ? "active" : ""}`}
            onClick={() => setActiveTopTab(tab)}
          >
            {tab.charAt(0).toUpperCase() + tab.slice(1)}
          </button>
        ))}
      </div>

      {/* Error banner */}
      {error && (
        <div style={{ padding: "var(--space-4) var(--space-4) 0" }}>
          <div className="error-banner">
            <span className="error-banner-message">{error}</span>
          </div>
        </div>
      )}

      {/* Main content */}
      <div style={{ padding: "var(--space-4)", display: "flex", gap: "var(--space-4)", flexDirection: "column", paddingBottom: 80 }}>

        {/* ── TRAFFIC TAB ── */}
        {activeTopTab === "traffic" && (
          <div style={{ display: "grid", gridTemplateColumns: "1fr 400px", gap: "var(--space-4)", alignItems: "start" }}>
            {/* Request list panel */}
            <div className="panel">
              <div className="panel-header">
                <span className="panel-title">Traffic</span>
                <span className="text-sm text-muted">{requests.length} requests</span>
              </div>

              {/* Filters */}
              <div style={{ padding: "var(--space-3)", borderBottom: "1px solid var(--border)", display: "flex", gap: "var(--space-3)", flexWrap: "wrap", alignItems: "center" }}>
                <select
                  value={selectedHost}
                  onChange={(e) => setSelectedHost(e.target.value)}
                  style={{ width: 180 }}
                >
                  <option value="all">All Hosts</option>
                  {[...new Set(requests.map((r) => r.host))].map((h) => (
                    <option key={h} value={h}>{h}</option>
                  ))}
                </select>
                <div className="tabs" style={{ borderBottom: "none", gap: 2 }}>
                  {(["all", "WeChat", "Douyin", "Alipay", "Unknown"] as AppTab[]).map((tab) => (
                    <button
                      key={tab}
                      className={`tab ${selectedTab === tab ? "active" : ""}`}
                      onClick={() => setSelectedTab(tab)}
                      style={{ fontSize: "var(--text-xs)", padding: "var(--space-1) var(--space-2)" }}
                    >
                      {tab === "all" ? "All" : tab}
                    </button>
                  ))}
                </div>
                <input
                  type="text"
                  placeholder="Filter by host or path..."
                  value={keywordFilter}
                  onChange={(e) => setKeywordFilter(e.target.value)}
                  style={{ flex: 1, minWidth: 140 }}
                />
                <button
                  className="btn btn-sm btn-secondary"
                  onClick={() => { setSessionName(`session-${Date.now()}`); setShowExportDialog(true); }}
                >
                  Export HAR
                </button>
              </div>

              {/* Table header */}
              <div style={{
                display: "flex", padding: "var(--space-2) var(--space-3)",
                background: "var(--bg-tertiary)", fontSize: "var(--text-xs)", fontWeight: 600,
                color: "var(--text-secondary)", textTransform: "uppercase" as const,
                letterSpacing: "0.5px", borderBottom: "1px solid var(--border)",
              }}>
                <div style={{ width: 60 }}>Method</div>
                <div style={{ flex: 1 }}>URL</div>
                <div style={{ width: 56, textAlign: "center" }}>Status</div>
                <div style={{ width: 64, textAlign: "right" }}>Latency</div>
                <div style={{ width: 64, textAlign: "right" }}>Size</div>
                <div style={{ width: 88 }}>Time</div>
                <div style={{ width: 80 }}>App</div>
              </div>

              {/* Rows */}
              {requests.length === 0 ? (
                <div className="empty-state">
                  <div className="empty-state-icon">📭</div>
                  <div className="empty-state-title">No requests captured</div>
                  <div className="empty-state-description">Start the proxy and make requests from your phone to see traffic here.</div>
                </div>
              ) : (
                <div style={{ maxHeight: 520, overflowY: "auto" }}>
                  {requests
                    .filter((req) => {
                      if (selectedTab !== "all") {
                        if (selectedTab === "Unknown") return !req.app_name;
                        return req.app_name === selectedTab;
                      }
                      return true;
                    })
                    .filter((req) => selectedHost === "all" || req.host === selectedHost)
                    .filter((req) => {
                      if (!keywordFilter) return true;
                      const kw = keywordFilter.toLowerCase();
                      return (
                        req.host.toLowerCase().includes(kw) ||
                        req.path.toLowerCase().includes(kw) ||
                        req.method.toLowerCase().includes(kw)
                      );
                    })
                    .map((req) => (
                      <div
                        key={req.id}
                        onClick={() => setSelectedRequest(selectedRequest?.id === req.id ? null : req)}
                        style={{
                          display: "flex", padding: "var(--space-2) var(--space-3)",
                          borderBottom: "1px solid var(--border)", cursor: "pointer",
                          fontSize: "var(--text-sm)", fontFamily: "var(--font-mono)",
                          alignItems: "center",
                          background: selectedRequest?.id === req.id ? "var(--bg-tertiary)" : "transparent",
                        }}
                      >
                        <div style={{ width: 60 }}>
                          <span className={`badge badge-${req.method.toLowerCase()}`}>{req.method}</span>
                        </div>
                        <div style={{ flex: 1, overflow: "hidden" }}>
                          <div className="truncate" style={{ fontSize: "var(--text-xs)" }}>{req.host}</div>
                          <div className="text-muted truncate" style={{ fontSize: 10 }}>{req.path}</div>
                        </div>
                        <div style={{ width: 56, textAlign: "center" }}>
                          {req.status && (
                            <span style={{
                              color: req.status < 300 ? "var(--accent-green)"
                                : req.status < 400 ? "var(--accent-yellow)"
                                : "var(--accent-red)", fontWeight: 600, fontSize: "var(--text-xs)",
                            }}>{req.status}</span>
                          )}
                        </div>
                        <div style={{ width: 64, textAlign: "right", fontSize: "var(--text-xs)" }}>
                          {req.latency_ms != null ? `${req.latency_ms}ms` : "—"}
                        </div>
                        <div style={{ width: 64, textAlign: "right", fontSize: "var(--text-xs)" }}>
                          {req.resp_size != null ? formatSize(req.resp_size) : "—"}
                        </div>
                        <div style={{ width: 88, fontSize: "var(--text-xs)", color: "var(--text-secondary)" }}>
                          {formatTimestamp(req.timestamp)}
                        </div>
                        <div style={{ width: 80 }}>
                          {req.app_name && (
                            <span className={`badge ${
                              req.app_name.toLowerCase().includes("wechat") ? "badge-wechat"
                                : req.app_name.toLowerCase().includes("douyin") ? "badge-douyin"
                                : req.app_name.toLowerCase().includes("alipay") ? "badge-alipay"
                                : "badge-unknown"
                            }`}>{req.app_name}</span>
                          )}
                        </div>
                      </div>
                    ))}
                </div>
              )}
            </div>

            {/* Request detail panel */}
            <div>
              {selectedRequest ? (
                <div className="panel">
                  <div className="panel-header">
                    <span className="panel-title">
                      <span className={`badge badge-${selectedRequest.method.toLowerCase()}`} style={{ marginRight: 8 }}>
                        {selectedRequest.method}
                      </span>
                      {selectedRequest.host}
                      <span className="text-muted truncate" style={{ marginLeft: 8, maxWidth: 160 }}>{selectedRequest.path}</span>
                    </span>
                    <button className="btn btn-sm btn-ghost" onClick={() => setSelectedRequest(null)}>×</button>
                  </div>
                  <div className="tabs">
                    {(["headers", "params", "body", ...(selectedRequest.is_websocket ? ["ws"] as const : [])] as const).map((t) => (
                      <button
                        key={t}
                        className={`tab ${detailTab === t ? "active" : ""}`}
                        onClick={() => setDetailTab(t as typeof detailTab)}
                      >
                        {t.charAt(0).toUpperCase() + t.slice(1)}
                      </button>
                    ))}
                  </div>
                  <div className="panel-body" style={{ maxHeight: 480, overflowY: "auto" }}>
                    {detailTab === "headers" && (
                      <div>
                        <div style={{ marginBottom: "var(--space-4)" }}>
                          <div className="card-title" style={{ marginBottom: "var(--space-2)" }}>Request Headers</div>
                          <table className="table">
                            <tbody>
                              {selectedRequest.req_headers.map(([n, v]) => (
                                <tr key={n}>
                                  <td style={{ fontWeight: 500, whiteSpace: "nowrap" }}>{n}</td>
                                  <td className="mono" style={{ wordBreak: "break-all", fontSize: "var(--text-xs)" }}>{v}</td>
                                </tr>
                              ))}
                            </tbody>
                          </table>
                        </div>
                        <div>
                          <div className="card-title" style={{ marginBottom: "var(--space-2)" }}>Response Headers</div>
                          <table className="table">
                            <tbody>
                              {selectedRequest.resp_headers.map(([n, v]) => (
                                <tr key={n}>
                                  <td style={{ fontWeight: 500, whiteSpace: "nowrap" }}>{n}</td>
                                  <td className="mono" style={{ wordBreak: "break-all", fontSize: "var(--text-xs)" }}>{v}</td>
                                </tr>
                              ))}
                            </tbody>
                          </table>
                        </div>
                      </div>
                    )}
                    {detailTab === "params" && (
                      <div>
                        {selectedRequest.query_params ? (
                          selectedRequest.query_params.split("&").map((p) => {
                            const [k, v] = p.split("=");
                            return (
                              <div key={k} style={{ display: "flex", gap: "var(--space-3)", padding: "var(--space-1) 0", borderBottom: "1px solid var(--border)" }}>
                                <span className="mono" style={{ color: "var(--accent-blue)", minWidth: 120 }}>{decodeURIComponent(k)}</span>
                                <span className="mono" style={{ fontSize: "var(--text-xs)" }}>{decodeURIComponent(v || "")}</span>
                              </div>
                            );
                          })
                        ) : (
                          <div className="empty-state" style={{ padding: "var(--space-6)" }}>
                            <div className="text-muted text-sm">No query parameters</div>
                          </div>
                        )}
                      </div>
                    )}
                    {detailTab === "body" && (
                      <div>
                        {selectedRequest.resp_body ? (
                          <pre className="mono" style={{ fontSize: "var(--text-xs)", whiteSpace: "pre-wrap", wordBreak: "break-all", background: "var(--bg-primary)", padding: "var(--space-3)", borderRadius: "var(--radius-md)" }}>
                            {formatBody(selectedRequest.resp_body, selectedRequest.resp_headers)}
                          </pre>
                        ) : (
                          <div className="empty-state" style={{ padding: "var(--space-6)" }}>
                            <div className="text-muted text-sm">No response body</div>
                          </div>
                        )}
                      </div>
                    )}
                    {detailTab === "ws" && selectedRequest.ws_frames && (
                      <div>
                        {selectedRequest.ws_frames.map((f, i) => (
                          <div key={i} style={{ display: "flex", gap: "var(--space-3)", padding: "var(--space-2)", borderBottom: "1px solid var(--border)", alignItems: "flex-start" }}>
                            <span className={`badge ${f.direction === "←" || f.direction === "IN" ? "badge-get" : "badge-post"}`}>{f.direction}</span>
                            <div style={{ flex: 1 }}>
                              <div className="text-xs text-muted" style={{ marginBottom: 2 }}>{f.timestamp} · {f.size}B</div>
                              <pre className="mono" style={{ fontSize: "var(--text-xs)", whiteSpace: "pre-wrap", wordBreak: "break-all", margin: 0 }}>{f.payload}</pre>
                            </div>
                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                </div>
              ) : (
                <div className="panel">
                  <div className="panel-header"><span className="panel-title">Request Detail</span></div>
                  <div className="panel-body">
                    <div className="empty-state">
                      <div className="empty-state-icon">👆</div>
                      <div className="empty-state-title">Select a request</div>
                      <div className="empty-state-description">Click a row to inspect details</div>
                    </div>
                  </div>
                </div>
              )}

              {/* Replay section */}
              <div className="panel" style={{ marginTop: "var(--space-4)" }}>
                <div className="panel-header">
                  <span className="panel-title">Replay</span>
                </div>
                <div className="panel-body">
                  <div style={{ display: "flex", gap: "var(--space-2)", marginBottom: "var(--space-3)", flexWrap: "wrap" }}>
                    <select
                      value={selectedReplayHost}
                      onChange={(e) => setSelectedReplayHost(e.target.value)}
                      style={{ flex: 1, minWidth: 120 }}
                    >
                      <option value="">Select host...</option>
                      {replayTargets.map((t) => (
                        <option key={t.host} value={t.host}>{t.host} ({t.request_count})</option>
                      ))}
                    </select>
                    <input
                      type="number"
                      min="0"
                      max="5000"
                      value={replayDelay}
                      onChange={(e) => setReplayDelay(Number(e.target.value))}
                      style={{ width: 80 }}
                      title="Delay (ms)"
                    />
                    <button
                      className="btn btn-sm btn-primary"
                      onClick={startReplay}
                      disabled={replaying || !selectedReplayHost}
                    >
                      {replaying ? "..." : "Replay"}
                    </button>
                  </div>
                  {replayResults.length > 0 && (
                    <div style={{ marginTop: "var(--space-3)", maxHeight: 200, overflowY: "auto" }}>
                      {replayResults.slice(0, 5).map((r) => (
                        <div key={r.request_id} style={{ display: "flex", gap: "var(--space-2)", padding: "var(--space-1) 0", borderBottom: "1px solid var(--border)", fontSize: "var(--text-xs)" }}>
                          <span className={`badge ${r.error ? "badge-delete" : r.diff?.has_changes ? "badge-put" : "badge-get"}`}>
                            {r.error ? "Err" : r.mock_response?.status || "?"}
                          </span>
                          <span className="mono truncate" style={{ flex: 1 }}>{r.url}</span>
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              </div>
            </div>
          </div>
        )}

        {/* ── DNS TAB ── */}
        {activeTopTab === "dns" && (
          <div className="panel">
            <div className="panel-header">
              <span className="panel-title">DNS Queries</span>
              <span className="text-sm text-muted">{dnsQueries.length} entries</span>
            </div>
            <div style={{ maxHeight: 500, overflowY: "auto" }}>
              {dnsQueries.length === 0 ? (
                <div className="empty-state">
                  <div className="empty-state-icon">🌐</div>
                  <div className="empty-state-title">No DNS queries</div>
                  <div className="empty-state-description">Enable transparent proxy to start capturing DNS queries</div>
                </div>
              ) : (
                <table className="table">
                  <thead>
                    <tr>
                      <th>App</th>
                      <th>Time</th>
                      <th>Domain</th>
                    </tr>
                  </thead>
                  <tbody>
                    {dnsQueries.map((q, idx) => (
                      <tr key={`${q.timestamp_ms}-${idx}`}>
                        <td>
                          {q.app_name && (
                            <span className={`badge ${
                              q.app_name.toLowerCase().includes("wechat") ? "badge-wechat"
                                : q.app_name.toLowerCase().includes("douyin") ? "badge-douyin"
                                : q.app_name.toLowerCase().includes("alipay") ? "badge-alipay"
                                : "badge-unknown"
                            }`}>{q.app_name}</span>
                          )}
                        </td>
                        <td className="mono text-sm">{new Date(q.timestamp_ms).toLocaleTimeString()}</td>
                        <td className="mono text-sm">{q.domain}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
            </div>
          </div>
        )}

        {/* ── RULES TAB ── */}
        {activeTopTab === "rules" && (
          <div>
            <div className="panel" style={{ marginBottom: "var(--space-4)" }}>
              <div className="panel-header">
                <span className="panel-title">Routing Rules</span>
                <div style={{ display: "flex", gap: "var(--space-2)", alignItems: "center" }}>
                  <select
                    value={selectedRuleFile}
                    onChange={(e) => setSelectedRuleFile(e.target.value)}
                    style={{ width: 140 }}
                  >
                    {ruleFiles.map((f) => <option key={f} value={f}>{f}</option>)}
                  </select>
                  <button
                    className="btn btn-sm btn-secondary"
                    onClick={() => { setEditingRule({ pattern: "DOMAIN-SUFFIX", value: "", action: "DIRECT" }); setShowRuleEditor(true); }}
                  >
                    + Add Rule
                  </button>
                </div>
              </div>
              <div style={{ maxHeight: 400, overflowY: "auto" }}>
                {rules.length === 0 ? (
                  <div className="empty-state">
                    <div className="empty-state-icon">📋</div>
                    <div className="empty-state-title">No rules defined</div>
                    <div className="empty-state-description">Click "Add Rule" to create your first routing rule.</div>
                  </div>
                ) : (
                  <table className="table">
                    <thead>
                      <tr>
                        <th>Pattern</th>
                        <th>Value</th>
                        <th>Action</th>
                        <th style={{ width: 120 }}>Controls</th>
                      </tr>
                    </thead>
                    <tbody>
                      {rules.map((rule, idx) => (
                        <tr key={`${rule.pattern}-${rule.value}-${idx}`}>
                          <td>
                            <span className={`badge ${
                              rule.action === "DIRECT" ? "badge-direct"
                                : rule.action === "PROXY" ? "badge-proxy"
                                : "badge-reject"
                            }`}>{rule.pattern}</span>
                          </td>
                          <td className="mono text-sm">{rule.value}</td>
                          <td>
                            <span className={`badge ${
                              rule.action === "DIRECT" ? "badge-direct"
                                : rule.action === "PROXY" ? "badge-proxy"
                                : "badge-reject"
                            }`}>{rule.action}</span>
                          </td>
                          <td>
                            <div style={{ display: "flex", gap: "var(--space-1)" }}>
                              <button className="btn btn-sm btn-ghost" onClick={() => moveRule(idx, "up")} disabled={idx === 0} title="Move up">↑</button>
                              <button className="btn btn-sm btn-ghost" onClick={() => moveRule(idx, "down")} disabled={idx === rules.length - 1} title="Move down">↓</button>
                              <button className="btn btn-sm btn-ghost" onClick={() => { setEditingRule(rule); setShowRuleEditor(true); }}>Edit</button>
                              <button className="btn btn-sm btn-ghost" onClick={() => deleteRule(rule)}>×</button>
                            </div>
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                )}
              </div>
            </div>

            {/* Rule editor modal */}
            {showRuleEditor && editingRule && (
              <div style={{
                position: "fixed", top: 0, left: 0, right: 0, bottom: 0,
                background: "rgba(0,0,0,0.6)", display: "flex", alignItems: "center", justifyContent: "center", zIndex: 100,
              }}>
                <div className="panel" style={{ width: 480 }}>
                  <div className="panel-header">
                    <span className="panel-title">Add / Edit Rule</span>
                    <button className="btn btn-sm btn-ghost" onClick={() => { setShowRuleEditor(false); setEditingRule(null); }}>×</button>
                  </div>
                  <div className="panel-body">
                    <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-3)" }}>
                      <div>
                        <label className="text-sm text-muted" style={{ display: "block", marginBottom: 4 }}>Pattern</label>
                        <select
                          value={editingRule.pattern}
                          onChange={(e) => setEditingRule({ ...editingRule, pattern: e.target.value as RulePattern })}
                          style={{ width: "100%" }}
                        >
                          <option value="DOMAIN">DOMAIN (exact match)</option>
                          <option value="DOMAIN-SUFFIX">DOMAIN-SUFFIX (matches subdomains)</option>
                          <option value="DOMAIN-KEYWORD">DOMAIN-KEYWORD (contains)</option>
                          <option value="IP-CIDR">IP-CIDR (e.g., 10.0.0.0/8)</option>
                        </select>
                      </div>
                      <div>
                        <label className="text-sm text-muted" style={{ display: "block", marginBottom: 4 }}>Value</label>
                        <input
                          type="text"
                          value={editingRule.value}
                          onChange={(e) => setEditingRule({ ...editingRule, value: e.target.value })}
                          placeholder={editingRule.pattern === "IP-CIDR" ? "10.0.0.0/8" : "example.com"}
                          style={{ width: "100%" }}
                        />
                      </div>
                      <div>
                        <label className="text-sm text-muted" style={{ display: "block", marginBottom: 4 }}>Action</label>
                        <select
                          value={editingRule.action}
                          onChange={(e) => setEditingRule({ ...editingRule, action: e.target.value as RuleAction })}
                          style={{ width: "100%" }}
                        >
                          <option value="DIRECT">DIRECT (bypass proxy)</option>
                          <option value="PROXY">PROXY (send through proxy)</option>
                          <option value="REJECT">REJECT (block connection)</option>
                        </select>
                      </div>
                      <div style={{ display: "flex", gap: "var(--space-2)", justifyContent: "flex-end" }}>
                        <button className="btn btn-sm btn-secondary" onClick={() => { setShowRuleEditor(false); setEditingRule(null); }}>Cancel</button>
                        <button className="btn btn-sm btn-primary" onClick={() => saveRule(editingRule)}>Save</button>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            )}
          </div>
        )}

        {/* ── DEVICES TAB ── */}
        {activeTopTab === "devices" && (
          <div>
            <div className="panel">
              <div className="panel-header">
                <span className="panel-title">Devices</span>
                <span className="text-sm text-muted">{devices.length} registered</span>
              </div>
              {devices.length === 0 ? (
                <div className="empty-state">
                  <div className="empty-state-icon">📱</div>
                  <div className="empty-state-title">No devices</div>
                  <div className="empty-state-description">Devices are registered when they connect through the proxy.</div>
                </div>
              ) : (
                <div style={{ maxHeight: 400, overflowY: "auto" }}>
                  <table className="table">
                    <thead>
                      <tr>
                        <th>Name</th>
                        <th>MAC</th>
                        <th>Last Seen</th>
                        <th>↑ Upload</th>
                        <th>↓ Download</th>
                        <th>Rule</th>
                      </tr>
                    </thead>
                    <tbody>
                      {devices.map((device) => (
                        <tr
                          key={device.id}
                          className={selectedDevice?.id === device.id ? "selected" : ""}
                          onClick={() => setSelectedDevice(selectedDevice?.id === device.id ? null : device)}
                          style={{ cursor: "pointer" }}
                        >
                          <td className="text-sm">{device.name}</td>
                          <td className="mono text-xs">{device.mac_address}</td>
                          <td className="text-xs text-muted">{new Date(device.last_seen_at).toLocaleString()}</td>
                          <td className="text-xs">{formatBytes(device.upload_bytes)}</td>
                          <td className="text-xs">{formatBytes(device.download_bytes)}</td>
                          <td>
                            <select
                              value={device.rule_override || ""}
                              onChange={(e) => updateDeviceRuleOverride(device.mac_address, e.target.value || null)}
                              onClick={(e) => e.stopPropagation()}
                              style={{ width: 90, fontSize: "var(--text-xs)" }}
                            >
                              <option value="">Default</option>
                              <option value="DIRECT">DIRECT</option>
                              <option value="PROXY">PROXY</option>
                              <option value="REJECT">REJECT</option>
                            </select>
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              )}
            </div>

            {/* Device topology */}
            {selectedDevice && (
              <div className="panel" style={{ marginTop: "var(--space-4)" }}>
                <div className="panel-header"><span className="panel-title">Device Topology</span></div>
                <div className="panel-body">
                  <div style={{ display: "flex", alignItems: "center", gap: "var(--space-4)" }}>
                    <div className="card" style={{ flex: 1 }}>
                      <div style={{ fontWeight: 600 }}>ProxyBot PC</div>
                      <div className="text-sm text-muted mono">{networkInfo?.lan_ip || "—"}</div>
                    </div>
                    <div style={{ color: "var(--text-muted)", fontSize: "var(--text-xl)" }}>→</div>
                    <div className="card" style={{ flex: 1 }}>
                      <div style={{ fontWeight: 600 }}>{selectedDevice.name}</div>
                      <div className="text-sm text-muted mono">{selectedDevice.mac_address}</div>
                      <div style={{ display: "flex", gap: "var(--space-4)", marginTop: "var(--space-2)", fontSize: "var(--text-xs)" }}>
                        <span style={{ color: "var(--accent-green)" }}>↑ {formatBytes(selectedDevice.upload_bytes)}</span>
                        <span style={{ color: "var(--accent-blue)" }}>↓ {formatBytes(selectedDevice.download_bytes)}</span>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            )}
          </div>
        )}

        {/* ── AI TAB ── */}
        {activeTopTab === "ai" && (
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
                      <span className={`badge ${
                        alert.severity === "Critical" ? "badge-critical"
                          : alert.severity === "Warning" ? "badge-warning"
                          : "badge-info"
                      }`}>{alert.severity}</span>
                      <div style={{ flex: 1 }}>
                        <div className="text-sm">{alert.alert_type}</div>
                        <div className="text-xs text-muted">{alert.details}</div>
                      </div>
                      {!alert.acknowledged && (
                        <button className="btn btn-sm btn-ghost" onClick={() => acknowledgeAlert(alert.id)}>Ack</button>
                      )}
                    </div>
                  ))}
                </div>
              )}
            </div>

            {/* Vision */}
            <div className="panel">
              <div className="panel-header">
                <span className="panel-title">Vision Screenshot Analyzer</span>
              </div>
              <div className="panel-body">
                <div style={{ display: "flex", gap: "var(--space-3)", marginBottom: "var(--space-4)", flexWrap: "wrap", alignItems: "center" }}>
                  <input
                    type="text"
                    value={visionSessionId}
                    onChange={(e) => setVisionSessionId(e.target.value)}
                    placeholder="Session ID"
                    style={{ width: 140 }}
                  />
                  <div style={{ display: "flex", gap: "var(--space-2)", alignItems: "center" }}>
                    <input
                      type="file"
                      accept="image/*"
                      onChange={handleScreenshotUpload}
                      disabled={visionAnalyzing}
                      id="screenshot-upload"
                      style={{ display: "none" }}
                    />
                    <label htmlFor="screenshot-upload" className="btn btn-sm btn-secondary" style={{ cursor: "pointer" }}>
                      {visionAnalyzing ? "Analyzing..." : "Upload Screenshot"}
                    </label>
                  </div>
                  <button
                    className="btn btn-sm btn-secondary"
                    onClick={fuseVisionWithApi}
                    disabled={visionAnalyses.length === 0}
                  >
                    Fuse with Traffic
                  </button>
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

            {/* Scaffold */}
            <div className="panel">
              <div className="panel-header"><span className="panel-title">Scaffold Generator</span></div>
              <div className="panel-body">
                <div style={{ display: "flex", gap: "var(--space-3)", marginBottom: "var(--space-3)", flexWrap: "wrap" }}>
                  <input
                    type="text"
                    value={scaffoldSessionId}
                    onChange={(e) => setScaffoldSessionId(e.target.value)}
                    placeholder="Session ID"
                    style={{ width: 140 }}
                  />
                  <input
                    type="text"
                    value={scaffoldProjectName}
                    onChange={(e) => setScaffoldProjectName(e.target.value)}
                    placeholder="Project name"
                    style={{ width: 180 }}
                  />
                  <button className="btn btn-sm btn-primary" onClick={generateScaffold} disabled={scaffoldGenerating}>
                    {scaffoldGenerating ? "..." : "Generate"}
                  </button>
                  <button className="btn btn-sm btn-secondary" onClick={writeScaffold} disabled={scaffoldGenerating}>Write</button>
                  <button className="btn btn-sm btn-secondary" onClick={evaluateScaffold} disabled={scaffoldGenerating || !scaffoldResult}>Eval</button>
                  <button className="btn btn-sm btn-primary" onClick={generateScaffoldWithVision} disabled={scaffoldGenerating || !fusedComponentTree} title={!fusedComponentTree ? "Fuse screenshot first" : "Generate with vision"}>
                    {scaffoldGenerating ? "..." : "Vision Scaffold"}
                  </button>
                  <button className="btn btn-sm btn-secondary" onClick={writeScaffoldWithVision} disabled={scaffoldGenerating || !fusedComponentTree} title={!fusedComponentTree ? "Fuse screenshot first" : "Write vision scaffold"}>
                    Write Vision
                  </button>
                </div>
                {scaffoldResult && (
                  <div className="text-xs text-muted">Files: {Object.keys(scaffoldResult.files || {}).length} — {scaffoldResult.components?.length || 0} components</div>
                )}
              </div>
            </div>

            {/* Deploy */}
            <div className="panel">
              <div className="panel-header"><span className="panel-title">Docker Deployment</span></div>
              <div className="panel-body">
                <div style={{ display: "flex", gap: "var(--space-3)", marginBottom: "var(--space-3)", flexWrap: "wrap" }}>
                  <input
                    type="text"
                    value={deploySessionId}
                    onChange={(e) => setDeploySessionId(e.target.value)}
                    placeholder="Session ID"
                    style={{ width: 140 }}
                  />
                  <input
                    type="text"
                    value={deployProjectName}
                    onChange={(e) => setDeployProjectName(e.target.value)}
                    placeholder="Project name"
                    style={{ width: 180 }}
                  />
                  <button className="btn btn-sm btn-primary" onClick={generateDeployment} disabled={deployGenerating}>
                    {deployGenerating ? "..." : "Generate"}
                  </button>
                  <button className="btn btn-sm btn-secondary" onClick={writeDeployment} disabled={deployGenerating}>Write</button>
                </div>
                {deployResult && (
                  <div className="text-xs text-muted">Bundle: {deployResult.bundle_path}</div>
                )}
              </div>
            </div>
          </div>
        )}
      </div>

      {/* Proxy control footer */}
      <div style={{
        position: "fixed", bottom: 0, left: 0, right: 0,
        padding: "var(--space-3) var(--space-4)",
        background: "var(--bg-secondary)", borderTop: "1px solid var(--border)",
        display: "flex", alignItems: "center", justifyContent: "space-between",
        fontSize: "var(--text-sm)", zIndex: 50,
      }}>
        <div style={{ display: "flex", gap: "var(--space-6)", alignItems: "center" }}>
          <div>
            <span className="text-muted">LAN IP: </span>
            <span className="font-mono">{networkInfo?.lan_ip || "—"}</span>
          </div>
          <div>
            <span className="text-muted">pf: </span>
            <span style={{ color: pfEnabled ? "var(--accent-green)" : "var(--text-muted)" }}>
              {pfEnabled ? "Enabled" : "Disabled"}
            </span>
          </div>
          <div>
            <span className="text-muted">TUN: </span>
            <span style={{ color: tunEnabled ? "var(--accent-green)" : "var(--text-muted)" }}>
              {tunEnabled ? "Enabled" : "Disabled"}
            </span>
          </div>
        </div>
        <div style={{ display: "flex", gap: "var(--space-2)" }}>
          {!pfEnabled ? (
            <button className="btn btn-sm btn-secondary" onClick={enableTransparentProxy} disabled={pfLoading || !networkInfo}>
              {pfLoading ? "..." : "Enable pf"}
            </button>
          ) : (
            <button className="btn btn-sm btn-secondary" onClick={disableTransparentProxy} disabled={pfLoading}>
              {pfLoading ? "..." : "Disable pf"}
            </button>
          )}
          {!tunEnabled && (
            <button className="btn btn-sm btn-secondary" onClick={enableTunMode} disabled={tunLoading}>
              {tunLoading ? "..." : "TUN Mode"}
            </button>
          )}
        </div>
      </div>

      {/* Export dialog */}
      {showExportDialog && (
        <div style={{
          position: "fixed", top: 0, left: 0, right: 0, bottom: 0,
          background: "rgba(0,0,0,0.6)", display: "flex", alignItems: "center", justifyContent: "center", zIndex: 100,
        }}>
          <div className="panel" style={{ width: 400 }}>
            <div className="panel-header">
              <span className="panel-title">Export HAR</span>
              <button className="btn btn-sm btn-ghost" onClick={() => { setShowExportDialog(false); setSessionName(""); }}>×</button>
            </div>
            <div className="panel-body">
              <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-3)" }}>
                <div>
                  <label className="text-sm text-muted" style={{ display: "block", marginBottom: 4 }}>Session Name</label>
                  <input
                    type="text"
                    value={sessionName}
                    onChange={(e) => setSessionName(e.target.value)}
                    placeholder="session-1234567890"
                    style={{ width: "100%" }}
                  />
                </div>
                <div className="text-xs text-muted">Saved to ~/.proxybot/exports/</div>
                <div style={{ display: "flex", gap: "var(--space-2)", justifyContent: "flex-end" }}>
                  <button className="btn btn-sm btn-secondary" onClick={() => { setShowExportDialog(false); setSessionName(""); }}>Cancel</button>
                  <button className="btn btn-sm btn-primary" onClick={exportHar} disabled={exporting}>
                    {exporting ? "..." : "Export"}
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* State machine panel */}
      {showStateMachinePanel && authStateMachine && (
        <div style={{
          position: "fixed", top: 0, left: 0, right: 0, bottom: 0,
          background: "rgba(0,0,0,0.6)", display: "flex", alignItems: "center", justifyContent: "center", zIndex: 100,
        }}>
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

export default App;
