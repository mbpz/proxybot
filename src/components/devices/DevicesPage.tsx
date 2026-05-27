import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "../ui/Button";
import { Card, CardHeader } from "../ui/Card";
import { ErrorBoundary } from "../ui/error-boundary";
import { SkeletonTable } from "../ui/skeleton";

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

interface NetworkInfo {
  lan_ip: string;
  interface: string;
}

export function DevicesPage() {
  const [devices, setDevices] = useState<DeviceInfo[]>([]);
  const [selectedDevice, setSelectedDevice] = useState<DeviceInfo | null>(null);
  const [networkInfo, setNetworkInfo] = useState<NetworkInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    loadDevices();
    loadNetworkInfo();
  }, []);

  async function loadDevices() {
    try {
      setLoading(true);
      setError(null);
      const result = await invoke<DeviceInfo[]>("get_devices");
      setDevices(result);
    } catch (err) {
      console.error("Failed to load devices:", err);
      setError(err instanceof Error ? err.message : "Failed to load devices");
    } finally {
      setLoading(false);
    }
  }

  async function loadNetworkInfo() {
    try {
      const info = await invoke<NetworkInfo>("get_network_info");
      setNetworkInfo(info);
    } catch (err) {
      console.error("Failed to load network info:", err);
    }
  }

  async function updateDeviceRuleOverride(
    macAddress: string,
    rule: string | null
  ) {
    try {
      await invoke("set_device_rule_override", {
        macAddress,
        rule,
      });
      await loadDevices();
    } catch (err) {
      console.error("Failed to update device rule:", err);
    }
  }

  function formatBytes(bytes: number): string {
    if (bytes === 0) return "0 B";
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024)
      return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
  }

  function formatTime(dateStr: string): string {
    const d = new Date(dateStr);
    return d.toLocaleString("en-US", {
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
      hour12: false,
    });
  }

  return (
    <div>
      <div className="panel">
        {/* Header */}
        <div className="panel-header">
          <div className="flex items-center gap-3">
            <span className="panel-title">Devices</span>
            <span className="text-sm text-muted">
              {devices.length} registered
            </span>
          </div>
          <Button variant="secondary" size="sm" onClick={loadDevices}>
            Refresh
          </Button>
        </div>

        {/* Error banner */}
        {error && (
          <div className="error-banner mx-4 mt-2">
            <span className="error-banner-message">{error}</span>
            <Button variant="secondary" size="sm" onClick={loadDevices}>
              Retry
            </Button>
          </div>
        )}

        {/* Content */}
        <div style={{ maxHeight: 500, overflowY: "auto" }}>
          <ErrorBoundary>
            {loading ? (
              <SkeletonTable rows={5} />
            ) : devices.length === 0 ? (
              <div className="empty-state">
                <div className="empty-state-icon">📱</div>
                <div className="empty-state-title">No devices</div>
                <div className="empty-state-description">
                  Devices are registered when they connect through the proxy.
                </div>
              </div>
            ) : (
              <table className="table">
                <thead>
                  <tr>
                    <th>Name</th>
                    <th>MAC</th>
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
                      onClick={() =>
                        setSelectedDevice(
                          selectedDevice?.id === device.id ? null : device
                        )
                      }
                      style={{
                        cursor: "pointer",
                        background:
                          selectedDevice?.id === device.id
                            ? "var(--bg-tertiary)"
                            : undefined,
                      }}
                    >
                      <td className="text-sm font-medium">{device.name}</td>
                      <td className="mono text-xs" style={{ color: "var(--text-muted)" }}>
                        {device.mac_address}
                      </td>
                      <td className="text-xs" style={{ color: "var(--text-muted)" }}>
                        {formatTime(device.last_seen_at)}
                      </td>
                      <td>
                        <span className="text-xs" style={{ color: "var(--accent-green)" }}>
                          {formatBytes(device.upload_bytes)}
                        </span>
                      </td>
                      <td>
                        <span className="text-xs" style={{ color: "var(--accent-blue)" }}>
                          {formatBytes(device.download_bytes)}
                        </span>
                      </td>
                      <td>
                        <select
                          value={device.rule_override || ""}
                          onChange={(e) => {
                            e.stopPropagation();
                            updateDeviceRuleOverride(
                              device.mac_address,
                              e.target.value || null
                            );
                          }}
                          onClick={(e) => e.stopPropagation()}
                          style={{
                            background: "var(--bg-tertiary)",
                            border: "1px solid var(--border)",
                            borderRadius: "var(--radius-md)",
                            padding: "var(--space-1) var(--space-2)",
                            fontSize: "var(--text-xs)",
                            color: "var(--text-primary)",
                            width: 90,
                          }}
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
            )}
          </ErrorBoundary>
        </div>
      </div>

      {/* Device topology */}
      {selectedDevice && (
        <div className="panel" style={{ marginTop: "var(--space-4)" }}>
          <div className="panel-header">
            <span className="panel-title">Device Topology</span>
          </div>
          <div className="panel-body">
            <div className="flex items-center gap-4">
              <Card className="flex-1">
                <CardHeader title="ProxyBot PC" />
                <div className="mono text-sm" style={{ color: "var(--text-muted)" }}>
                  {networkInfo?.lan_ip || "—"}
                </div>
              </Card>

              <div style={{ color: "var(--text-muted)", fontSize: "var(--text-xl)" }}>
                →
              </div>

              <Card className="flex-1">
                <CardHeader title={selectedDevice.name} />
                <div className="mono text-xs" style={{ color: "var(--text-muted)" }}>
                  {selectedDevice.mac_address}
                </div>
                <div className="flex gap-4 mt-2">
                  <span className="text-xs" style={{ color: "var(--accent-green)" }}>
                    ↑ {formatBytes(selectedDevice.upload_bytes)}
                  </span>
                  <span className="text-xs" style={{ color: "var(--accent-blue)" }}>
                    ↓ {formatBytes(selectedDevice.download_bytes)}
                  </span>
                </div>
              </Card>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
