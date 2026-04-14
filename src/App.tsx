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
}

interface NetworkInfo {
  lan_ip: string;
  interface: string;
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

  useEffect(() => {
    invoke<string>("get_ca_cert_path").then(setCaCertPath).catch(console.error);
    invoke<NetworkInfo>("get_network_info")
      .then(setNetworkInfo)
      .catch((e) => console.error("Failed to get network info:", e));

    const unlisten = listen<InterceptedRequest>("intercepted-request", (event) => {
      setRequests((prev) => [event.payload, ...prev].slice(0, 100));
    });

    return () => {
      unlisten.then((fn) => fn());
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
      const result = await invoke<string>("setup_pf", { interface: networkInfo.interface });
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
        <h2>Intercepted Requests ({requests.length})</h2>
        <div className="requests-list">
          {requests.length === 0 ? (
            <p className="no-requests">No requests yet. Configure your browser or device to use ProxyBot as the proxy.</p>
          ) : (
            <table className="requests-table">
              <thead>
                <tr>
                  <th>Time</th>
                  <th>Method</th>
                  <th>Host</th>
                  <th>Path</th>
                  <th>Status</th>
                  <th>Latency</th>
                </tr>
              </thead>
              <tbody>
                {requests.map((req) => (
                  <tr key={req.id}>
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
    </main>
  );
}

export default App;
