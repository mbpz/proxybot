import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./App.css";

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

function App() {
  const [running, setRunning] = useState(false);
  const [caCertPath, setCaCertPath] = useState("");
  const [requests, setRequests] = useState<InterceptedRequest[]>([]);
  const [error, setError] = useState("");
  const [networkInfo, setNetworkInfo] = useState<NetworkInfo | null>(null);
  const [pfEnabled, setPfEnabled] = useState(false);
  const [pfLoading, setPfLoading] = useState(false);
  const [pfStatus, setPfStatus] = useState("");
  const [tunEnabled, setTunEnabled] = useState(false);
  const [tunLoading, setTunLoading] = useState(false);
  const [tunStatus, setTunStatus] = useState("");
  const [dnsQueries, setDnsQueries] = useState<DnsEntry[]>([]);
  const [selectedTab, setSelectedTab] = useState<AppTab>("all");
  const [caMetadata, setCaMetadata] = useState<CaMetadata | null>(null);
  const [selectedHost, setSelectedHost] = useState<string>("all");
  const [keywordFilter, setKeywordFilter] = useState("");
  const [selectedRequest, setSelectedRequest] = useState<InterceptedRequest | null>(null);
  const [selectedDetailTab, setSelectedDetailTab] = useState<"headers" | "body" | "params" | "ws">("headers");
  const [rules, setRules] = useState<Rule[]>([]);
  const [ruleFiles, setRuleFiles] = useState<string[]>([]);
  const [selectedRuleFile, setSelectedRuleFile] = useState<string>("rules.yaml");
  const [showRuleEditor, setShowRuleEditor] = useState(false);
  const [editingRule, setEditingRule] = useState<Rule | null>(null);
  const [devices, setDevices] = useState<DeviceInfo[]>([]);
  const [selectedDevice, setSelectedDevice] = useState<DeviceInfo | null>(null);
  const [editingDevice, setEditingDevice] = useState<DeviceInfo | null>(null);

  useEffect(() => {
    invoke<string>("get_ca_cert_path").then(setCaCertPath).catch(console.error);
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
      setPfStatus("");

      // Check if pf is already enabled — if so, skip setup to avoid redundant admin prompt
      const alreadyEnabled = await invoke<boolean>("is_pf_enabled");
      if (alreadyEnabled) {
        setPfEnabled(true);
        setPfStatus("Transparent proxy is already enabled");
        return;
      }

      // get_network_info populates ProxyState with interface + local_ip
      await invoke<NetworkInfo>("get_network_info");
      // setup_pf reads from ProxyState — no args needed
      const result = await invoke<string>("setup_pf");
      console.log(result);
      setPfEnabled(true);
      setPfStatus("Transparent proxy enabled successfully");
    } catch (e) {
      setError(String(e));
      setPfStatus("Failed to enable transparent proxy");
    } finally {
      setPfLoading(false);
    }
  };

  const disableTransparentProxy = async () => {
    try {
      setPfLoading(true);
      setError("");
      setPfStatus("");
      await invoke<void>("teardown_pf");
      setPfEnabled(false);
      setPfStatus("Transparent proxy disabled");
    } catch (e) {
      setError(String(e));
      setPfStatus("Failed to disable transparent proxy");
    } finally {
      setPfLoading(false);
    }
  };

  const enableTunMode = async () => {
    try {
      setTunLoading(true);
      setError("");
      setTunStatus("");
      const result = await invoke<string>("setup_tun");
      console.log(result);
      setTunEnabled(true);
      setTunStatus(result);
    } catch (e) {
      setError(String(e));
      setTunStatus("Failed to enable TUN/VPN mode");
    } finally {
      setTunLoading(false);
    }
  };

  const disableTunMode = async () => {
    try {
      setTunLoading(true);
      setError("");
      setTunStatus("");
      await invoke<void>("teardown_tun");
      setTunEnabled(false);
      setTunStatus("TUN/VPN mode disabled");
    } catch (e) {
      setError(String(e));
      setTunStatus("Failed to disable TUN/VPN mode");
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

  const regenerateCa = async () => {
    try {
      if (!window.confirm("Regenerating the CA will break existing HTTPS intercept sessions. Continue?")) {
        return;
      }
      await invoke<void>("regenerate_ca");
      setCaMetadata(null);
      invoke<CaMetadata | null>("get_ca_metadata").then(setCaMetadata).catch(console.error);
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

  const formatHeaders = (headers: [string, string][]): string => {
    return headers.map(([name, value]) => `${name}: ${value}`).join("\n");
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

  const isImageContent = (req: InterceptedRequest): boolean => {
    const contentType = req.resp_headers.find(([name]) => name.toLowerCase() === "content-type");
    if (!contentType) return false;
    const ct = contentType[1].toLowerCase();
    return ct.startsWith("image/");
  };

  const buildImageDataUrl = (req: InterceptedRequest): string => {
    const contentType = req.resp_headers.find(([name]) => name.toLowerCase() === "content-type");
    const ct = contentType ? contentType[1] : "image/png";
    const base64 = btoa(req.resp_body || "");
    return `data:${ct};base64,${base64}`;
  };

  const loadDevices = async () => {
    try {
      const deviceList = await invoke<DeviceInfo[]>("get_devices");
      setDevices(deviceList);
    } catch (e) {
      console.error("Failed to load devices:", e);
    }
  };

  const updateDeviceName = async (macAddress: string, name: string) => {
    try {
      // Use register_device to update the name (upsert behavior)
      await invoke("register_device", { macAddress, name });
      await loadDevices();
      setEditingDevice(null);
    } catch (e) {
      setError(String(e));
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

  const formatBytes = (bytes: number): string => {
    if (bytes < 1024) return `${bytes}B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)}KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)}MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)}GB`;
  };

  return (
    <main className="container">
      <header className="header">
        <h1>ProxyBot</h1>
        <p className="subtitle">HTTPS MITM Proxy</p>
      </header>

      <section className="controls">
        <button
          className={`btn ${running ? "btn-stop" : "btn-start"}`}
          onClick={startProxy}
          disabled={running}
        >
          {running ? "Running..." : "Start Proxy"}
        </button>

        <div className="status">
          Status: <span className={running ? "status-running" : "status-stopped"}>
            {running ? `Listening on port 8080` : "Stopped"}
          </span>
        </div>
      </section>

      <section className="setup-panel">
        <h2>Transparent Proxy Setup</h2>
        {networkInfo ? (
          <div className="network-info">
            <p className="lan-ip">
              <strong>PC LAN IP:</strong> <span className="ip-address">{networkInfo.lan_ip}</span>
            </p>
            <p className="interface-name">
              <strong>Interface:</strong> {networkInfo.interface}
            </p>
          </div>
        ) : (
          <p className="network-loading">Detecting network interface...</p>
        )}

        <div className="setup-buttons">
          {!pfEnabled ? (
            <button
              className="btn btn-enable"
              onClick={enableTransparentProxy}
              disabled={!networkInfo || pfLoading}
            >
              {pfLoading ? "Enabling..." : "Enable Transparent Proxy"}
            </button>
          ) : (
            <button
              className="btn btn-disable"
              onClick={disableTransparentProxy}
              disabled={pfLoading}
            >
              {pfLoading ? "Disabling..." : "Disable Transparent Proxy"}
            </button>
          )}
        </div>

        {pfStatus && (
          <p className={`pf-status ${pfEnabled ? "status-active" : "status-inactive"}`}>
            {pfStatus}
          </p>
        )}

        <div className="dns-status">
          <span className="dns-label">DNS Server:</span>
          <span className={`dns-indicator ${pfEnabled ? "dns-running" : "dns-stopped"}`}>
            {pfEnabled ? "Listening on UDP 5300" : "Not running"}
          </span>
        </div>

        <div className="setup-instructions">
          <h3>Instructions</h3>
          <ol>
            <li>Enable transparent proxy above</li>
            <li>On your phone, go to Wi-Fi settings</li>
            <li>Set the HTTP proxy to this computer's IP ({networkInfo?.lan_ip || "..."})</li>
            <li>Set proxy port to 8080</li>
            <li>For HTTPS interception, install the ProxyBot CA certificate on your phone</li>
          </ol>
          <p className="note">
            <strong>Note:</strong> For transparent proxy mode (no proxy configuration on phone),
            enable the transparent proxy above. This requires administrator privileges.
          </p>
        </div>
      </section>

      <section className="setup-panel">
        <h2>VPN/TUN Mode (Fallback)</h2>
        <p className="tun-description">
          For devices that cannot use transparent proxy (Android 7+ without MDM, iOS without MDM),
          use TUN/VPN mode instead. This creates a VPN interface that captures all device traffic.
        </p>

        <div className="setup-buttons">
          {!tunEnabled ? (
            <button
              className="btn btn-enable"
              onClick={enableTunMode}
              disabled={tunLoading}
            >
              {tunLoading ? "Enabling..." : "Enable TUN/VPN Mode"}
            </button>
          ) : (
            <button
              className="btn btn-disable"
              onClick={disableTunMode}
              disabled={tunLoading}
            >
              {tunLoading ? "Disabling..." : "Disable TUN/VPN Mode"}
            </button>
          )}
        </div>

        {tunStatus && (
          <p className={`tun-status ${tunEnabled ? "status-active" : "status-inactive"}`}>
            {tunStatus}
          </p>
        )}

        <div className="setup-instructions">
          <h3>TUN/VPN Instructions</h3>
          <ol>
            <li>Enable TUN/VPN mode above</li>
            <li>On your phone, install a VPN profile pointing to this computer's IP ({networkInfo?.lan_ip || "..."})</li>
            <li>Connect to the VPN from your phone</li>
            <li>All traffic will be captured by ProxyBot</li>
          </ol>
          <p className="note">
            <strong>Note:</strong> TUN/VPN mode captures all device traffic without requiring
            proxy configuration on the device.
          </p>
        </div>
      </section>

      <section className="ca-info">
        <h2>CA Certificate</h2>
        <p className="ca-path">{caCertPath}</p>
        {caMetadata && (
          <p className="ca-meta">
            Created: {new Date(caMetadata.created_at * 1000).toLocaleString()} | Serial: {caMetadata.serial}
          </p>
        )}
        <p className="ca-instructions">
          To intercept HTTPS traffic, install the CA certificate on your device and trust it.
        </p>
        <div className="ca-buttons">
          <button className="btn btn-download" onClick={downloadCaCert}>
            Download CA Certificate
          </button>
          <button className="btn btn-regenerate" onClick={regenerateCa}>
            Regenerate CA
          </button>
        </div>
      </section>

      {error && (
        <div className="error">{error}</div>
      )}

      <section className="requests">
        <h2>Intercepted Requests ({requests.length})</h2>
        <div className="app-tabs">
          {(["all", "WeChat", "Douyin", "Alipay", "Unknown"] as AppTab[]).map((tab) => (
            <button
              key={tab}
              className={`tab-btn ${selectedTab === tab ? "tab-active" : ""}`}
              onClick={() => setSelectedTab(tab)}
            >
              {tab === "all" ? "All" : tab === "WeChat" ? "WeChat 💬" : tab === "Douyin" ? "Douyin 🎵" : tab === "Alipay" ? "Alipay 💳" : "Unknown"}
            </button>
          ))}
        </div>
        <div className="requests-list">
          <div className="requests-toolbar">
            <select
              className="host-filter"
              value={selectedHost}
              onChange={(e) => setSelectedHost(e.target.value)}
            >
              <option value="all">All Hosts</option>
              {[...new Set(requests.map((r) => r.host))].map((host) => (
                <option key={host} value={host}>{host}</option>
              ))}
            </select>
            <input
              type="text"
              className="keyword-filter"
              placeholder="Filter by keyword..."
              value={keywordFilter}
              onChange={(e) => setKeywordFilter(e.target.value)}
            />
          </div>
          {requests.length === 0 ? (
            <p className="no-requests">No requests yet. Configure your browser or device to use ProxyBot as the proxy.</p>
          ) : (
            <table className="requests-table">
              <thead>
                <tr>
                  <th>App</th>
                  <th>Time</th>
                  <th>Method</th>
                  <th>Host</th>
                  <th>Path</th>
                  <th>Status</th>
                  <th>Size</th>
                  <th>Latency</th>
                </tr>
              </thead>
              <tbody>
                {requests
                  .filter((req) => {
                    if (selectedTab === "all") return true;
                    if (selectedTab === "Unknown") return !req.app_name;
                    return req.app_name === selectedTab;
                  })
                  .filter((req) => {
                    if (selectedHost === "all") return true;
                    return req.host === selectedHost;
                  })
                  .filter((req) => {
                    if (!keywordFilter) return true;
                    const kw = keywordFilter.toLowerCase();
                    return (
                      req.host.toLowerCase().includes(kw) ||
                      req.path.toLowerCase().includes(kw) ||
                      req.method.toLowerCase().includes(kw) ||
                      (req.resp_body && req.resp_body.toLowerCase().includes(kw))
                    );
                  })
                  .map((req) => (
                    <tr
                      key={req.id}
                      className={selectedRequest?.id === req.id ? "selected" : ""}
                      onClick={() => setSelectedRequest(selectedRequest?.id === req.id ? null : req)}
                    >
                      <td className="app-cell">
                        {req.app_icon ? `${req.app_icon} ${req.app_name}` : "-"}
                      </td>
                      <td className="time">{formatTimestamp(req.timestamp)}</td>
                      <td className="method">{req.method}</td>
                      <td className="host">{req.host}</td>
                      <td className="path">{req.path}</td>
                      <td className={`status-code ${req.status && req.status >= 400 ? "status-error" : ""}`}>
                        {req.status || "-"}
                      </td>
                      <td className="size">{req.resp_size ? formatSize(req.resp_size) : "-"}</td>
                      <td className="latency">{req.latency_ms ? `${req.latency_ms}ms` : "-"}</td>
                    </tr>
                  ))}
              </tbody>
            </table>
          )}
        </div>

        {selectedRequest && (
          <div className="request-detail">
            <div className="detail-header">
              <h3>{selectedRequest.method} {selectedRequest.host}{selectedRequest.path}</h3>
              <button className="close-btn" onClick={() => setSelectedRequest(null)}>×</button>
            </div>
            <div className="detail-tabs">
              <button
                className={`detail-tab ${"headers" === selectedDetailTab ? "active" : ""}`}
                onClick={() => setSelectedDetailTab("headers")}
              >
                Headers
              </button>
              <button
                className={`detail-tab ${"body" === selectedDetailTab ? "active" : ""}`}
                onClick={() => setSelectedDetailTab("body")}
              >
                Body
              </button>
              <button
                className={`detail-tab ${"params" === selectedDetailTab ? "active" : ""}`}
                onClick={() => setSelectedDetailTab("params")}
              >
                Params
              </button>
              {selectedRequest.is_websocket && (
                <button
                  className={`detail-tab ${"ws" === selectedDetailTab ? "active" : ""}`}
                  onClick={() => setSelectedDetailTab("ws")}
                >
                  WebSocket
                </button>
              )}
            </div>
            <div className="detail-content">
              {selectedDetailTab === "headers" && (
                <div className="headers-section">
                  <div className="headers-group">
                    <h4>Request Headers</h4>
                    {selectedRequest.req_headers.length > 0 ? (
                      <pre className="headers-pre">{formatHeaders(selectedRequest.req_headers)}</pre>
                    ) : (
                      <p className="no-data">No request headers</p>
                    )}
                  </div>
                  <div className="headers-group">
                    <h4>Response Headers</h4>
                    {selectedRequest.resp_headers.length > 0 ? (
                      <pre className="headers-pre">{formatHeaders(selectedRequest.resp_headers)}</pre>
                    ) : (
                      <p className="no-data">No response headers</p>
                    )}
                  </div>
                </div>
              )}
              {selectedDetailTab === "body" && (
                <div className="body-section">
                  {isImageContent(selectedRequest) ? (
                    <div className="image-preview">
                      <img src={buildImageDataUrl(selectedRequest)} alt="Response" />
                    </div>
                  ) : (
                    <pre className="body-pre">{formatBody(selectedRequest.resp_body, selectedRequest.resp_headers)}</pre>
                  )}
                </div>
              )}
              {selectedDetailTab === "params" && (
                <div className="params-section">
                  {selectedRequest.query_params ? (
                    <pre className="params-pre">{selectedRequest.query_params}</pre>
                  ) : (
                    <p className="no-data">No query parameters</p>
                  )}
                </div>
              )}
              {selectedDetailTab === "ws" && selectedRequest.ws_frames && (
                <div className="ws-section">
                  {selectedRequest.ws_frames.map((frame, idx) => (
                    <div key={idx} className={`ws-frame ${frame.direction.toLowerCase()}`}>
                      <span className="ws-direction">{frame.direction}</span>
                      <span className="ws-time">{formatTimestamp(frame.timestamp)}</span>
                      <pre className="ws-payload">{frame.payload}</pre>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </div>
        )}
      </section>

      <section className="dns-log">
        <h2>DNS Queries ({dnsQueries.length})</h2>
        <div className="dns-log-list">
          {dnsQueries.length === 0 ? (
            <p className="no-dns-queries">No DNS queries yet. Enable transparent proxy to start capturing.</p>
          ) : (
            <table className="dns-table">
              <thead>
                <tr>
                  <th>App</th>
                  <th>Time</th>
                  <th>Domain</th>
                </tr>
              </thead>
              <tbody>
                {dnsQueries.map((query, idx) => (
                  <tr key={`${query.timestamp_ms}-${idx}`}>
                    <td className="app-cell">
                      {query.app_icon ? `${query.app_icon} ${query.app_name}` : "-"}
                    </td>
                    <td className="time">
                      {new Date(query.timestamp_ms).toLocaleTimeString()}
                    </td>
                    <td className="domain">{query.domain}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
      </section>

      <section className="devices-panel">
        <h2>Devices ({devices.length})</h2>
        {devices.length === 0 ? (
          <p className="no-devices">No devices yet. Devices are registered when they connect through the proxy.</p>
        ) : (
          <div className="devices-content">
            <div className="devices-list">
              <table className="devices-table">
                <thead>
                  <tr>
                    <th>Name</th>
                    <th>IP/MAC</th>
                    <th>Last Seen</th>
                    <th>Upload</th>
                    <th>Download</th>
                    <th>Rule Override</th>
                  </tr>
                </thead>
                <tbody>
                  {devices.map((device) => (
                    <tr
                      key={device.id}
                      className={selectedDevice?.id === device.id ? "selected" : ""}
                      onClick={() => setSelectedDevice(selectedDevice?.id === device.id ? null : device)}
                    >
                      <td className="device-name">
                        {editingDevice?.id === device.id ? (
                          <input
                            type="text"
                            value={editingDevice.name}
                            onChange={(e) => setEditingDevice({ ...editingDevice, name: e.target.value })}
                            onBlur={() => updateDeviceName(device.mac_address, editingDevice.name)}
                            onKeyDown={(e) => {
                              if (e.key === "Enter") {
                                updateDeviceName(device.mac_address, editingDevice.name);
                              } else if (e.key === "Escape") {
                                setEditingDevice(null);
                              }
                            }}
                            autoFocus
                            onClick={(e) => e.stopPropagation()}
                          />
                        ) : (
                          <span onDoubleClick={(e) => {
                            e.stopPropagation();
                            setEditingDevice(device);
                          }}>{device.name}</span>
                        )}
                      </td>
                      <td className="device-mac">{device.mac_address}</td>
                      <td className="device-last-seen">
                        {new Date(device.last_seen_at).toLocaleString()}
                      </td>
                      <td className="device-upload">{formatBytes(device.upload_bytes)}</td>
                      <td className="device-download">{formatBytes(device.download_bytes)}</td>
                      <td className="device-rule">
                        <select
                          value={device.rule_override || ""}
                          onChange={(e) => updateDeviceRuleOverride(device.mac_address, e.target.value || null)}
                          onClick={(e) => e.stopPropagation()}
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

            {selectedDevice && (
              <div className="device-topology">
                <h3>Device Topology</h3>
                <div className="topology-diagram">
                  <div className="topology-node topology-pc">
                    <div className="node-icon">PC</div>
                    <div className="node-label">ProxyBot PC</div>
                    <div className="node-ip">{networkInfo?.lan_ip || "..."}</div>
                  </div>
                  <div className="topology-line">
                    <div className="line-arrow">→</div>
                    <div className="line-label">proxy</div>
                  </div>
                  <div className="topology-node topology-device">
                    <div className="node-icon">📱</div>
                    <div className="node-label">{selectedDevice.name}</div>
                    <div className="node-ip">{selectedDevice.mac_address}</div>
                    <div className="node-stats">
                      <span>↑ {formatBytes(selectedDevice.upload_bytes)}</span>
                      <span>↓ {formatBytes(selectedDevice.download_bytes)}</span>
                    </div>
                  </div>
                </div>
              </div>
            )}
          </div>
        )}
      </section>

      <section className="rules-editor">
        <div className="rules-header">
          <h2>Routing Rules ({rules.length})</h2>
          <div className="rules-actions">
            <select
              className="rule-file-select"
              value={selectedRuleFile}
              onChange={(e) => setSelectedRuleFile(e.target.value)}
            >
              {ruleFiles.map((f) => (
                <option key={f} value={f}>{f}</option>
              ))}
            </select>
            <button
              className="btn btn-rule-add"
              onClick={() => { setEditingRule({ pattern: "DOMAIN-SUFFIX", value: "", action: "DIRECT" }); setShowRuleEditor(true); }}
            >
              Add Rule
            </button>
          </div>
        </div>

        {rules.length === 0 ? (
          <p className="no-rules">
            No rules defined. Click "Add Rule" to create your first routing rule.
          </p>
        ) : (
          <div className="rules-list">
            <table className="rules-table">
              <thead>
                <tr>
                  <th>Pattern</th>
                  <th>Value</th>
                  <th>Action</th>
                  <th>Controls</th>
                </tr>
              </thead>
              <tbody>
                {rules.map((rule, idx) => (
                  <tr key={`${rule.pattern}-${rule.value}-${idx}`}>
                    <td className="rule-pattern">
                      <span className={`pattern-badge pattern-${rule.pattern.toLowerCase().replace(/-/g, "")}`}>
                        {rule.pattern}
                      </span>
                    </td>
                    <td className="rule-value">{rule.value}</td>
                    <td className="rule-action">
                      <span className={`action-badge action-${rule.action.toLowerCase()}`}>
                        {rule.action}
                      </span>
                    </td>
                    <td className="rule-controls">
                      <button
                        className="btn-move btn-move-up"
                        onClick={() => moveRule(idx, "up")}
                        disabled={idx === 0}
                        title="Move up"
                      >↑</button>
                      <button
                        className="btn-move btn-move-down"
                        onClick={() => moveRule(idx, "down")}
                        disabled={idx === rules.length - 1}
                        title="Move down"
                      >↓</button>
                      <button
                        className="btn-rule-edit"
                        onClick={() => { setEditingRule(rule); setShowRuleEditor(true); }}
                        title="Edit"
                      >Edit</button>
                      <button
                        className="btn-rule-delete"
                        onClick={() => deleteRule(rule)}
                        title="Delete"
                      >×</button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}

        {showRuleEditor && editingRule && (
          <div className="rule-editor-modal">
            <div className="rule-editor-content">
              <h3>{rules.some(r => r.pattern === editingRule.pattern && r.value === editingRule.value) ? "Edit Rule" : "Add Rule"}</h3>
              <div className="rule-editor-form">
                <div className="form-group">
                  <label>Pattern</label>
                  <select
                    value={editingRule.pattern}
                    onChange={(e) => setEditingRule({ ...editingRule, pattern: e.target.value as RulePattern })}
                  >
                    <option value="DOMAIN">DOMAIN (exact match)</option>
                    <option value="DOMAIN-SUFFIX">DOMAIN-SUFFIX (example.com matches sub.example.com)</option>
                    <option value="DOMAIN-KEYWORD">DOMAIN-KEYWORD (matches if value appears anywhere)</option>
                    <option value="IP-CIDR">IP-CIDR (e.g., 10.0.0.0/8)</option>
                  </select>
                </div>
                <div className="form-group">
                  <label>Value</label>
                  <input
                    type="text"
                    value={editingRule.value}
                    onChange={(e) => setEditingRule({ ...editingRule, value: e.target.value })}
                    placeholder={editingRule.pattern === "IP-CIDR" ? "10.0.0.0/8" : "example.com"}
                  />
                </div>
                <div className="form-group">
                  <label>Action</label>
                  <select
                    value={editingRule.action}
                    onChange={(e) => setEditingRule({ ...editingRule, action: e.target.value as RuleAction })}
                  >
                    <option value="DIRECT">DIRECT (bypass proxy)</option>
                    <option value="PROXY">PROXY (send through proxy)</option>
                    <option value="REJECT">REJECT (block connection)</option>
                  </select>
                </div>
                <div className="form-actions">
                  <button className="btn btn-save" onClick={() => saveRule(editingRule)}>Save</button>
                  <button className="btn btn-cancel" onClick={() => { setShowRuleEditor(false); setEditingRule(null); }}>Cancel</button>
                </div>
              </div>
            </div>
          </div>
        )}
      </section>
    </main>
  );
}

export default App;
