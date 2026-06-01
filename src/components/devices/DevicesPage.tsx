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
      {/* Header */}
      <div
        style={{
          height: 48,
          padding: "0 16px",
          background: "#12121a",
          borderBottom: "1px solid #1e1e2e",
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
        }}
      >
        <span style={{ color: "#fff", fontSize: 14 }}>
          Connected Devices ({devices.length})
        </span>
        <button
          style={{
            padding: "6px 12px",
            borderRadius: 4,
            background: "#00d4ff20",
            border: "1px solid #00d4ff",
            color: "#00d4ff",
            fontSize: 11,
            cursor: "pointer",
          }}
          onClick={loadDevices}
        >
          + Add Device
        </button>
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
            <div style={{ padding: 12, display: "flex", flexDirection: "column", gap: 8 }}>
              {devices.map((device) => {
                const isOnline =
                  Date.now() - new Date(device.last_seen_at).getTime() < 60000;
                const statusColor = isOnline ? "var(--accent-green)" : "var(--accent-purple)";
                return (
                  <div
                    key={device.id}
                    onClick={() =>
                      setSelectedDevice(
                        selectedDevice?.id === device.id ? null : device
                      )
                    }
                    className="device-card"
                    style={{
                      padding: 12,
                      borderRadius: 8,
                      background: "linear-gradient(180deg, var(--bg-tertiary) 0%, var(--bg-secondary) 100%)",
                      border: "1px solid rgba(0, 212, 255, 0.25)",
                      display: "flex",
                      flexDirection: "column",
                      gap: 8,
                      cursor: "pointer",
                      transition: "border-color 0.2s, box-shadow 0.2s",
                    }}
                    onMouseEnter={(e) => {
                      e.currentTarget.style.borderColor = "rgba(0, 212, 255, 0.5)";
                      e.currentTarget.style.boxShadow = "0 0 12px rgba(0, 212, 255, 0.2)";
                    }}
                    onMouseLeave={(e) => {
                      e.currentTarget.style.borderColor = "rgba(0, 212, 255, 0.25)";
                      e.currentTarget.style.boxShadow = "none";
                    }}
                  >
                    {/* Card Header */}
                    <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start" }}>
                      <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                        <span style={{ color: "var(--text-primary)", fontSize: 14 }}>
                          {device.name}
                        </span>
                        <span style={{ color: "var(--text-muted)", fontSize: 11 }}>
                          {device.mac_address}
                        </span>
                      </div>
                      <div
                        style={{
                          width: 10,
                          height: 10,
                          borderRadius: "50%",
                          background: statusColor,
                          boxShadow: `0 0 8px ${statusColor}`,
                        }}
                      />
                    </div>
                    {/* Stats */}
                    <div style={{ display: "flex", gap: 12 }}>
                      <span style={{ color: "var(--text-secondary)", fontSize: 11 }}>
                        Req: {device.upload_bytes.toLocaleString()}
                      </span>
                      <span style={{ color: "var(--text-secondary)", fontSize: 11 }}>
                        Data: {formatBytes(device.upload_bytes + device.download_bytes)}
                      </span>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </ErrorBoundary>
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
