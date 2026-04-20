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

interface CaMetadata {
  created_at: number;
  serial: string;
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

    return () => {
      unlisten.then((fn) => fn());
      unlistenDns.then((fn) => fn());
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
                  .map((req) => (
                    <tr key={req.id}>
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
