import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { DeviceInfo, NetworkInfo } from "../types";
import { formatBytes } from "../utils";

interface DevicesPageProps {
  networkInfo: NetworkInfo | null;
}

export function DevicesPage({ networkInfo }: DevicesPageProps) {
  const [devices, setDevices] = useState<DeviceInfo[]>([]);
  const [selectedDevice, setSelectedDevice] = useState<DeviceInfo | null>(null);

  useEffect(() => {
    invoke<DeviceInfo[]>("get_devices").then(setDevices).catch(console.error);
  }, []);

  const loadDevices = async () => {
    try {
      setDevices(await invoke<DeviceInfo[]>("get_devices"));
    } catch (e) {
      console.error("Failed to load devices:", e);
    }
  };

  const updateDeviceRuleOverride = async (macAddress: string, ruleOverride: string | null) => {
    try {
      await invoke("set_device_rule_override", { macAddress, ruleOverride });
      await loadDevices();
    } catch (e) {
      alert(String(e));
    }
  };

  return (
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
              <thead><tr><th>Name</th><th>MAC</th><th>Last Seen</th><th>↑ Upload</th><th>↓ Download</th><th>Rule</th></tr></thead>
              <tbody>
                {devices.map((device) => (
                  <tr key={device.id} className={selectedDevice?.id === device.id ? "selected" : ""}
                    onClick={() => setSelectedDevice(selectedDevice?.id === device.id ? null : device)} style={{ cursor: "pointer" }}>
                    <td className="text-sm">{device.name}</td>
                    <td className="mono text-xs">{device.mac_address}</td>
                    <td className="text-xs text-muted">{new Date(device.last_seen_at).toLocaleString()}</td>
                    <td className="text-xs">{formatBytes(device.upload_bytes)}</td>
                    <td className="text-xs">{formatBytes(device.download_bytes)}</td>
                    <td>
                      <select value={device.rule_override || ""} onClick={(e) => e.stopPropagation()}
                        onChange={(e) => updateDeviceRuleOverride(device.mac_address, e.target.value || null)}
                        style={{ width: 90, fontSize: "var(--text-xs)" }}>
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
  );
}
