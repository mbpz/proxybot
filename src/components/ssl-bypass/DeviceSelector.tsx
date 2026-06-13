import { useSslBypass } from "../../stores/sslBypassStore";

export function DeviceSelector() {
  const store = useSslBypass();
  return (
    <div className="card mb-4">
      <div className="flex justify-between items-center mb-2">
        <h3 className="card-title text-base">Devices</h3>
        <button
          onClick={store.refreshDevices}
          className="btn btn-sm btn-secondary"
          data-testid="ssl-bypass-refresh-devices"
        >
          Refresh
        </button>
      </div>
      {store.devices.length === 0 ? (
        <p className="text-sm text-text-muted">
          No devices found. Connect an Android device via USB and click Refresh.
        </p>
      ) : (
        <select
          value={store.selectedDevice ?? ""}
          onChange={(e) => store.setSelectedDevice(e.target.value || null)}
          className="w-full px-2 py-1 border border-border rounded bg-surface-primary text-text-primary"
          data-testid="ssl-bypass-device-select"
        >
          <option value="">Select a device</option>
          {store.devices.map((d) => (
            <option key={d.id} value={d.id}>
              {d.name} ({d.device_type}) {d.is_connected ? "[connected]" : "[offline]"}
            </option>
          ))}
        </select>
      )}
    </div>
  );
}