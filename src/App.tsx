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
  status: number | null;
  latency_ms: number | null;
  scheme: string;
  app_name?: string;
  app_icon?: string;
  request_headers?: string;
  response_headers?: string;
  response_body?: string;
  request_body?: string;
}

interface WssMessage {
  id: string;
  timestamp: string;
  host: string;
  direction: string;
  size: number;
  content: string;
  app_name?: string;
  app_icon?: string;
}

type AppTab = "all" | "WeChat" | "Douyin" | "Alipay" | "Unknown";

interface NetworkInfo {
  lan_ip: string;
  interface: string;
}

interface DnsEntry {
  domain: string;
  timestamp_ms: number;
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
  const [dnsQueries, setDnsQueries] = useState<DnsEntry[]>([]);
  const [selectedTab, setSelectedTab] = useState<AppTab>("all");
  const [wssMessages, setWssMessages] = useState<WssMessage[]>([]);
  const [selectedWssTab, setSelectedWssTab] = useState<AppTab>("all");
  const [selectedRequest, setSelectedRequest] = useState<InterceptedRequest | null>(null);
  const [detailTab, setDetailTab] = useState<"general" | "headers" | "body">("general");
  const [searchQuery, setSearchQuery] = useState("");
  const [methodFilter, setMethodFilter] = useState("ALL");
  const [statusFilter, setStatusFilter] = useState("ALL");
  const [appFilter, setAppFilter] = useState("ALL");

  useEffect(() => {
    invoke<string>("get_ca_cert_path").then(setCaCertPath).catch(console.error);
    invoke<NetworkInfo>("get_network_info")
      .then(setNetworkInfo)
      .catch((e) => console.error("Failed to get network info:", e));

    const unlisten = listen<InterceptedRequest>("intercepted-request", (event) => {
      setRequests((prev) => [event.payload, ...prev].slice(0, 100));
    });

    const unlistenDns = listen<DnsEntry>("dns-query", (event) => {
      setDnsQueries((prev) => [event.payload, ...prev].slice(0, 50));
    });

    const unlistenWss = listen<WssMessage>("intercepted-wss", (event) => {
      setWssMessages((prev) => [event.payload, ...prev].slice(0, 200));
    });

    // Load initial DNS log
    invoke<DnsEntry[]>("get_dns_log")
      .then(setDnsQueries)
      .catch((e) => console.error("Failed to get DNS log:", e));

    return () => {
      unlisten.then((fn) => fn());
      unlistenDns.then((fn) => fn());
      unlistenWss.then((fn) => fn());
    };
  }, []);

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

  const matchesStatusGroup = (status: number | null, group: string): boolean => {
    if (group === "ALL" || !status) return true;
    if (group === "2xx") return status >= 200 && status < 300;
    if (group === "3xx") return status >= 300 && status < 400;
    if (group === "4xx") return status >= 400 && status < 500;
    if (group === "5xx") return status >= 500;
    return status === parseInt(group);
  };

  const clearFilters = () => {
    setSearchQuery("");
    setMethodFilter("ALL");
    setStatusFilter("ALL");
    setAppFilter("ALL");
  };

  const exportHar = async () => {
    try {
      const filtered = filterRequests(requests);
      const har = await invoke<string>("export_har", { requests: filtered });
      const blob = new Blob([har], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `proxybot-${new Date().toISOString().slice(0, 10)}.har`;
      a.click();
      URL.revokeObjectURL(url);
    } catch (e) {
      setError(String(e));
    }
  };

  const filterRequests = (reqs: InterceptedRequest[]) => {
    return reqs.filter((req) => {
      // Tab filter (app tabs)
      if (selectedTab === "all") return true;
      if (selectedTab === "Unknown") return !req.app_name;
      if (req.app_name !== selectedTab) return false;

      // Filter bar filters
      const search = searchQuery.toLowerCase();
      const matchSearch =
        !search ||
        req.host.toLowerCase().includes(search) ||
        req.path.toLowerCase().includes(search) ||
        req.method.toLowerCase().includes(search.toUpperCase());

      const matchMethod = methodFilter === "ALL" || req.method === methodFilter;
      const matchStatus = matchesStatusGroup(req.status, statusFilter);
      const matchApp = appFilter === "ALL" || req.app_name === appFilter;

      return matchSearch && matchMethod && matchStatus && matchApp;
    });
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

      <section className="ca-info">
        <h2>CA Certificate</h2>
        <p className="ca-path">{caCertPath}</p>
        <p className="ca-instructions">
          To intercept HTTPS traffic, install the CA certificate in your system/browser and trust it.
        </p>
      </section>

      {error && (
        <div className="error">{error}</div>
      )}

      <section className="requests">
        <h2>Intercepted Requests ({filterRequests(requests).length})</h2>
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
        <div className="filter-bar">
          <button className="btn-export" onClick={exportHar}>
            Export HAR
          </button>
          <input
            type="text"
            className="filter-search"
            placeholder="Search host, path, method..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
          />
          <select
            className="filter-select"
            value={methodFilter}
            onChange={(e) => setMethodFilter(e.target.value)}
          >
            <option value="ALL">All Methods</option>
            <option value="GET">GET</option>
            <option value="POST">POST</option>
            <option value="PUT">PUT</option>
            <option value="DELETE">DELETE</option>
            <option value="WebSocket">WS</option>
          </select>
          <select
            className="filter-select"
            value={statusFilter}
            onChange={(e) => setStatusFilter(e.target.value)}
          >
            <option value="ALL">All Status</option>
            <option value="2xx">2xx</option>
            <option value="3xx">3xx</option>
            <option value="4xx">4xx</option>
            <option value="5xx">5xx</option>
          </select>
          <select
            className="filter-select"
            value={appFilter}
            onChange={(e) => setAppFilter(e.target.value)}
          >
            <option value="ALL">All Apps</option>
            <option value="WeChat">WeChat</option>
            <option value="Douyin">Douyin</option>
            <option value="Alipay">Alipay</option>
            <option value="Unknown">Unknown</option>
          </select>
          <button className="btn-clear" onClick={clearFilters}>
            Clear
          </button>
        </div>
        <div className="requests-list">
          {filterRequests(requests).length === 0 ? (
            <p className="no-requests">No requests match the current filters.</p>
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
                  <th>Latency</th>
                </tr>
              </thead>
              <tbody>
                {filterRequests(requests).map((req) => (
                    <tr
                      key={req.id}
                      className={selectedRequest?.id === req.id ? "row-selected" : ""}
                      onClick={() => setSelectedRequest(req)}
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
                      <td className="latency">{req.latency_ms ? `${req.latency_ms}ms` : "-"}</td>
                    </tr>
                  ))}
              </tbody>
            </table>
          )}
        </div>
      </section>

      {selectedRequest && (
        <div className="detail-panel-overlay" onClick={() => setSelectedRequest(null)}>
          <div className="detail-panel" onClick={(e) => e.stopPropagation()}>
            <div className="detail-header">
              <h3>Request Details</h3>
              <button className="detail-close" onClick={() => setSelectedRequest(null)}>×</button>
            </div>
            <div className="detail-tabs">
              <button
                className={`detail-tab ${detailTab === "general" ? "tab-active" : ""}`}
                onClick={() => setDetailTab("general")}
              >
                General
              </button>
              <button
                className={`detail-tab ${detailTab === "headers" ? "tab-active" : ""}`}
                onClick={() => setDetailTab("headers")}
              >
                Headers
              </button>
              <button
                className={`detail-tab ${detailTab === "body" ? "tab-active" : ""}`}
                onClick={() => setDetailTab("body")}
              >
                Body
              </button>
            </div>
            <div className="detail-content">
              {detailTab === "general" && (
                <div className="detail-general">
                  <div className="detail-row">
                    <span className="detail-label">Method:</span>
                    <span className="detail-value">{selectedRequest.method}</span>
                  </div>
                  <div className="detail-row">
                    <span className="detail-label">URL:</span>
                    <span className="detail-value">{selectedRequest.scheme}://{selectedRequest.host}{selectedRequest.path}</span>
                  </div>
                  <div className="detail-row">
                    <span className="detail-label">Status:</span>
                    <span className="detail-value">{selectedRequest.status || "-"}</span>
                  </div>
                  <div className="detail-row">
                    <span className="detail-label">Latency:</span>
                    <span className="detail-value">{selectedRequest.latency_ms ? `${selectedRequest.latency_ms}ms` : "-"}</span>
                  </div>
                  <div className="detail-row">
                    <span className="detail-label">App:</span>
                    <span className="detail-value">{selectedRequest.app_name || "Unknown"}</span>
                  </div>
                  <div className="detail-row">
                    <span className="detail-label">Time:</span>
                    <span className="detail-value">{formatTimestamp(selectedRequest.timestamp)}</span>
                  </div>
                </div>
              )}
              {detailTab === "headers" && (
                <div className="detail-headers">
                  <div className="detail-section">
                    <h4>Request Headers</h4>
                    <pre className="detail-pre">
                      {selectedRequest.request_headers || "(no headers)"}
                    </pre>
                  </div>
                  <div className="detail-section">
                    <h4>Response Headers</h4>
                    <pre className="detail-pre">
                      {selectedRequest.response_headers || "(no headers)"}
                    </pre>
                  </div>
                </div>
              )}
              {detailTab === "body" && (
                <div className="detail-body">
                  <div className="detail-section">
                    <h4>Request Body</h4>
                    <pre className="detail-pre">
                      {selectedRequest.request_body || "(no body)"}
                    </pre>
                  </div>
                  <div className="detail-section">
                    <h4>Response Body</h4>
                    <pre className="detail-pre">
                      {selectedRequest.response_body || "(no body)"}
                    </pre>
                  </div>
                </div>
              )}
            </div>
          </div>
        </div>
      )}

      <section className="wss-messages">
        <h2>WSS Messages ({wssMessages.length})</h2>
        <div className="app-tabs">
          {(["all", "WeChat", "Douyin", "Alipay", "Unknown"] as AppTab[]).map((tab) => (
            <button
              key={tab}
              className={`tab-btn ${selectedWssTab === tab ? "tab-active" : ""}`}
              onClick={() => setSelectedWssTab(tab)}
            >
              {tab === "all" ? "All" : tab === "WeChat" ? "WeChat 💬" : tab === "Douyin" ? "Douyin 🎵" : tab === "Alipay" ? "Alipay 💳" : "Unknown"}
            </button>
          ))}
        </div>
        <div className="wss-messages-list">
          {wssMessages.length === 0 ? (
            <p className="no-wss-messages">No WebSocket messages yet. Open WeChat or Douyin on your phone to see WSS traffic.</p>
          ) : (
            <table className="wss-table">
              <thead>
                <tr>
                  <th>App</th>
                  <th>Time</th>
                  <th>Direction</th>
                  <th>Host</th>
                  <th>Size</th>
                  <th>Content Preview</th>
                </tr>
              </thead>
              <tbody>
                {wssMessages
                  .filter((msg) => {
                    if (selectedWssTab === "all") return true;
                    if (selectedWssTab === "Unknown") return !msg.app_name;
                    return msg.app_name === selectedWssTab;
                  })
                  .map((msg) => (
                    <tr key={msg.id}>
                      <td className="app-cell">
                        {msg.app_icon ? `${msg.app_icon} ${msg.app_name}` : "-"}
                      </td>
                      <td className="time">{formatTimestamp(msg.timestamp)}</td>
                      <td className={`direction ${msg.direction === "up" ? "direction-up" : "direction-down"}`}>
                        {msg.direction === "up" ? "↑" : "↓"}
                      </td>
                      <td className="host">{msg.host}</td>
                      <td className="size">{msg.size}</td>
                      <td className="content-preview">{msg.content.length > 50 ? msg.content.slice(0, 50) + "..." : msg.content}</td>
                    </tr>
                  ))}
              </tbody>
            </table>
          )}
        </div>
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
                  <th>Time</th>
                  <th>Domain</th>
                </tr>
              </thead>
              <tbody>
                {dnsQueries.map((query, idx) => (
                  <tr key={`${query.timestamp_ms}-${idx}`}>
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
    </main>
  );
}

export default App;
