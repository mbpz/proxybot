import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
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
  const [requests, setRequests] = useState<InterceptedRequest[]>([]);
  const [error, setError] = useState("");
  const [networkInfo, setNetworkInfo] = useState<NetworkInfo | null>(null);
  const [pfEnabled, setPfEnabled] = useState(false);
  const [pfLoading, setPfLoading] = useState(false);
  const [dnsQueries, setDnsQueries] = useState<DnsEntry[]>([]);
  const [wssMessages, setWssMessages] = useState<WssMessage[]>([]);
  const [selectedWssTab, setSelectedWssTab] = useState<AppTab>("all");
  const [selectedRequest, setSelectedRequest] = useState<InterceptedRequest | null>(null);
  const [selectedWssMsg, setSelectedWssMsg] = useState<WssMessage | null>(null);
  const [detailTab, setDetailTab] = useState<"general" | "headers" | "body">("general");
  const [caGuideTab, setCaGuideTab] = useState<"ios" | "android">("ios");
  const [searchQuery, setSearchQuery] = useState("");
  const [methodFilter, setMethodFilter] = useState("ALL");
  const [statusFilter, setStatusFilter] = useState("ALL");
  const [appFilter, setAppFilter] = useState("ALL");
  const [theme, setTheme] = useState<'dark' | 'light'>('dark');
  const [keepRunning, setKeepRunning] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [mainTab, setMainTab] = useState<'http' | 'wss' | 'dns'>('http');

  const toggleTheme = () => {
    setTheme((t) => (t === 'dark' ? 'light' : 'dark'));
  };

  const toggleKeepRunning = async (value: boolean) => {
    setKeepRunning(value);
    await invoke("set_keep_running", { keep: value });
  };

  useEffect(() => {
    document.documentElement.classList.remove('dark', 'light');
    document.documentElement.classList.add(theme);
  }, [theme]);

  useEffect(() => {
    invoke<boolean>("get_keep_running").then(setKeepRunning).catch(console.error);
  }, []);

  useEffect(() => {
    const window = getCurrentWindow();
    const unlisten = window.onCloseRequested(async (event) => {
      if (keepRunning) {
        event.preventDefault();
        await invoke("hide_window");
      }
    });
    return () => { unlisten.then(fn => fn()); };
  }, [keepRunning]);

  useEffect(() => {
    invoke<string>("get_ca_cert_path").catch(console.error);
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

    // Load history on mount
    invoke<InterceptedRequest[]>("load_history")
      .then((hist) => {
        if (hist.length > 0) setRequests(hist);
      })
      .catch((e) => console.error("Failed to load history:", e));

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

  const stopProxy = async () => {
    try {
      setError("");
      const result = await invoke<string>("stop_proxy");
      console.log(result);
      setRunning(false);
    } catch (e) {
      setError(String(e));
    }
  };

  const enableTransparentProxy = async () => {
    if (!networkInfo) return;
    try {
      setPfLoading(true);
      setError("");
      // setPfStatus removed;
      // get_network_info populates ProxyState with interface + local_ip
      await invoke<NetworkInfo>("get_network_info");
      // setup_pf reads from ProxyState — no args needed
      const result = await invoke<string>("setup_pf");
      console.log(result);
      setPfEnabled(true);
      // status updated in settings panel;
    } catch (e) {
      setError(String(e));
      // status updated in settings panel;
    } finally {
      setPfLoading(false);
    }
  };

  const disableTransparentProxy = async () => {
    try {
      setPfLoading(true);
      setError("");
      // setPfStatus removed;
      await invoke<void>("teardown_pf");
      setPfEnabled(false);
      // status updated in settings panel;
    } catch (e) {
      setError(String(e));
      // status updated in settings panel
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

  const isBinaryContent = (content: string): boolean => {
    // Null byte or high proportion of non-printable control chars (excluding \n\r\t) indicates binary
    if (content.includes("\0")) return true;
    const nonPrintable = content.split("").filter((c) => {
      const code = c.charCodeAt(0);
      return code < 32 && code !== 9 && code !== 10 && code !== 13;
    }).length;
    return content.length > 0 && nonPrintable / content.length > 0.3;
  };

  const getWssDirectionLabel = (dir: string) => {
    if (dir === "up") return "↑ Sent";
    if (dir === "down") return "↓ Received";
    return dir;
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

  const exportJson = async () => {
    const filtered = filterRequests(requests);
    const data = JSON.stringify(filtered, null, 2);
    const blob = new Blob([data], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `proxybot-requests-${new Date().toISOString().slice(0, 10)}.json`;
    a.click();
    URL.revokeObjectURL(url);
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

  const saveHistory = async () => {
    try {
      await invoke("save_history", { requests });
    } catch (e) {
      setError(String(e));
    }
  };

  const clearHistory = async () => {
    if (!confirm("确定清空所有历史记录？")) return;
    try {
      await invoke("save_history", { requests: [] });
      setRequests([]);
    } catch (e) {
      setError(String(e));
    }
  };

  const copyAsCurl = (req: InterceptedRequest) => {
    const headers = req.request_headers || '';
    const headerLines = headers.split('\n').filter((l: string) => l.trim());
    const headerArgs = headerLines.map((l: string) => {
      const idx = l.indexOf(': ');
      if (idx < 0) return '';
      return `-H "${l}"`;
    }).filter(Boolean).join(' ');
    const bodyArg = req.method === 'POST' && req.request_body ? `-d '${req.request_body}'` : '';
    const curl = `curl ${headerArgs} ${bodyArg} 'https://${req.host}${req.path}'`;
    navigator.clipboard.writeText(curl);
  };

  const replayRequest = async (id: string) => {
    try {
      await invoke("replay_request", { id });
    } catch (e) {
      setError(String(e));
    }
  };

  const downloadCaCert = async () => {
    try {
      const path = await invoke<string>("get_ca_cert_path");
      const { openPath } = await import('@tauri-apps/plugin-opener');
      await openPath(path);
    } catch (e) {
      setError(String(e));
    }
  };

  const filterRequests = (reqs: InterceptedRequest[]) => {
    return reqs.filter((req) => {
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
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <div>
            <h1>ProxyBot</h1>
            <p className="subtitle">HTTPS MITM Proxy</p>
          </div>
          <div style={{ display: 'flex', gap: '8px', alignItems: 'center' }}>
            <button className="btn-clear" onClick={toggleTheme} style={{ padding: '6px 12px', fontSize: '12px' }}>
              {theme === 'dark' ? '☀️ Light' : '🌙 Dark'}
            </button>
            <button className="settings-btn" onClick={() => setShowSettings(true)}>⚙️</button>
          </div>
        </div>
      </header>

      {error && (
        <div className="error global-notice">
          <span>{error}</span>
          <button onClick={() => setError("")}>×</button>
        </div>
      )}

      <div className="top-tabs">
        <button className={mainTab === 'http' ? 'active' : ''} onClick={() => setMainTab('http')}>
          HTTP Requests ({requests.length})
        </button>
        <button className={mainTab === 'wss' ? 'active' : ''} onClick={() => setMainTab('wss')}>
          WSS Messages ({wssMessages.length})
        </button>
        <button className={mainTab === 'dns' ? 'active' : ''} onClick={() => setMainTab('dns')}>
          DNS Queries ({dnsQueries.length})
        </button>
      </div>

                  {mainTab === 'http' && (
        <>
        <section className="requests">
        <h2>Intercepted Requests ({filterRequests(requests).length})</h2>
        <div className="filter-bar">
          <button className="btn-export" onClick={exportHar}>
            📄 HAR
          </button>
          <button className="btn-export" onClick={exportJson}>
            📄 JSON
          </button>
          <button className="btn-save" onClick={saveHistory}>
            💾 保存
          </button>
          <button className="btn-clear-history" onClick={clearHistory}>
            🗑️ 清空
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
                  <th>Actions</th>
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
                      <td className="actions">
                        <button title="Copy as cURL" onClick={(e) => { e.stopPropagation(); copyAsCurl(req); }}>📋</button>
                        <button title="Replay" onClick={(e) => { e.stopPropagation(); replayRequest(req.id); }}>⧉</button>
                      </td>
                    </tr>
                  ))}
              </tbody>
            </table>
          )}
        </div>
      </section>
        </>
      )}

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

      {selectedWssMsg && (
        <div className="detail-panel-overlay" onClick={() => setSelectedWssMsg(null)}>
          <div className="detail-panel" onClick={(e) => e.stopPropagation()}>
            <div className="detail-header">
              <h3>WSS Frame Details</h3>
              <button className="detail-close" onClick={() => setSelectedWssMsg(null)}>×</button>
            </div>
            <div className="detail-content">
              <div className="detail-general">
                <div className="detail-row">
                  <span className="detail-label">Direction:</span>
                  <span className="detail-value">{getWssDirectionLabel(selectedWssMsg.direction)}</span>
                </div>
                <div className="detail-row">
                  <span className="detail-label">Host:</span>
                  <span className="detail-value">{selectedWssMsg.host}</span>
                </div>
                <div className="detail-row">
                  <span className="detail-label">Size:</span>
                  <span className="detail-value">{selectedWssMsg.size} bytes</span>
                </div>
                <div className="detail-row">
                  <span className="detail-label">App:</span>
                  <span className="detail-value">
                    {selectedWssMsg.app_icon ? `${selectedWssMsg.app_icon} ` : ""}{selectedWssMsg.app_name || "Unknown"}
                  </span>
                </div>
                <div className="detail-row">
                  <span className="detail-label">Time:</span>
                  <span className="detail-value">{formatTimestamp(selectedWssMsg.timestamp)}</span>
                </div>
              </div>
              <div className="detail-section">
                <h4>Content</h4>
                {isBinaryContent(selectedWssMsg.content) ? (
                  <pre className="detail-pre">[Binary {selectedWssMsg.size} bytes — not displayed as text]</pre>
                ) : (
                  <pre className="detail-pre">{selectedWssMsg.content}</pre>
                )}
              </div>
            </div>
          </div>
        </div>
      )}

      {mainTab === 'wss' && (
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
                    <tr
                      key={msg.id}
                      className={`wss-row ${msg.direction} ${selectedWssMsg?.id === msg.id ? "row-selected" : ""}`}
                      onClick={() => setSelectedWssMsg(msg)}
                    >
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
      )}

      {mainTab === 'dns' && (
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
      )}

      {showSettings && (
        <div className="detail-panel-overlay" onClick={() => setShowSettings(false)}>
          <div className="detail-panel" onClick={(e) => e.stopPropagation()}>
            <div className="detail-header">
              <h3>⚙️ Settings</h3>
              <button className="detail-close" onClick={() => setShowSettings(false)}>×</button>
            </div>
            <div className="detail-content">
              <div className="settings-section">
                <h4>🚀 代理服务</h4>
                <div className="settings-row">
                  <span>Status:</span>
                  <span className={running ? "status-running" : "status-stopped"}>
                    {running ? `Listening on port 8080` : "Stopped"}
                  </span>
                </div>
                {!running ? (
                  <button className="btn btn-start" onClick={startProxy}>Start Proxy</button>
                ) : (
                  <button className="btn btn-stop" onClick={stopProxy}>Stop Proxy</button>
                )}
              </div>

              <div className="settings-section">
                <h4>🔐 透明代理</h4>
                <div className="settings-row">
                  <span>Status:</span>
                  <span className={pfEnabled ? "status-running" : "status-stopped"}>
                    {pfEnabled ? "Enabled" : "Disabled"}
                  </span>
                </div>
                <button
                  className={`btn ${pfEnabled ? "btn-disable" : "btn-enable"}`}
                  onClick={() => pfEnabled ? disableTransparentProxy() : enableTransparentProxy()}
                  disabled={pfLoading || !networkInfo}
                >
                  {pfLoading ? "Loading..." : pfEnabled ? "Disable Transparent Proxy" : "Enable Transparent Proxy"}
                </button>
              </div>

              <div className="settings-section">
                <h4>📜 CA证书</h4>
                <div className="ca-guide-tabs">
                  <button
                    className={`ca-tab-btn ${caGuideTab === "ios" ? "ca-tab-active" : ""}`}
                    onClick={() => setCaGuideTab("ios")}
                  >
                    iOS
                  </button>
                  <button
                    className={`ca-tab-btn ${caGuideTab === "android" ? "ca-tab-active" : ""}`}
                    onClick={() => setCaGuideTab("android")}
                  >
                    Android
                  </button>
                </div>
                {caGuideTab === "ios" ? (
                  <ol className="ca-steps">
                    <li>Tap "Download CA Certificate" to open in Safari</li>
                    <li>iOS prompts "A profile was downloaded" — tap Allow</li>
                    <li>Settings → General → VPN and Device Management → Install</li>
                    <li>Settings → General → About → Certificate Trust Settings → Enable ProxyBot CA</li>
                  </ol>
                ) : (
                  <ol className="ca-steps">
                    <li>Download CA Certificate and open the downloaded <code>ca.crt</code></li>
                    <li>Enter lock screen PIN when prompted</li>
                    <li>For Android 7+: May need ADB or "Install unknown apps" for browser</li>
                  </ol>
                )}
                <button className="btn-download-ca" onClick={downloadCaCert}>
                  Download CA Certificate
                </button>
              </div>

              <div className="settings-section">
                <h4>🔄 后台运行</h4>
                <div className="settings-row">
                  <span>Keep running when window closes</span>
                  <label className="toggle-switch">
                    <input
                      type="checkbox"
                      checked={keepRunning}
                      onChange={(e) => toggleKeepRunning(e.target.checked)}
                    />
                    <span className="toggle-slider"></span>
                  </label>
                </div>
              </div>

              <div className="settings-section">
                <h4>🗑️ 清除历史</h4>
                <button className="btn-clear-history" onClick={clearHistory}>
                  Clear All History
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </main>
  );
}

export default App;
