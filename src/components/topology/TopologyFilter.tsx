import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { TopologyFilter as Filter } from "./types";

interface Device {
  id: number;
  name: string;
}

const APP_TAGS = ["wechat", "douyin", "alipay", "amazon", "apple", "ai", "tiktok"];

interface Props {
  filter: Filter;
  onChange: (filter: Filter) => void;
  onRefresh: () => void;
}

export function TopologyFilter({ filter, onChange, onRefresh }: Props) {
  const [devices, setDevices] = useState<Device[]>([]);
  const [deviceError, setDeviceError] = useState<string | null>(null);

  const loadDevices = useCallback(async () => {
    setDeviceError(null);
    try {
      setDevices(await invoke<Device[]>("get_devices"));
    } catch (cause) {
      setDeviceError(cause instanceof Error ? cause.message : String(cause));
    }
  }, []);

  useEffect(() => {
    void loadDevices();
  }, [loadDevices]);

  function toggleDevice(id: string) {
    const current = filter.device_ids ?? [];
    const next = current.includes(id) ? current.filter((d) => d !== id) : [...current, id];
    onChange({ ...filter, device_ids: next });
  }

  function toggleAppTag(tag: string) {
    const current = filter.app_tags ?? [];
    const next = current.includes(tag) ? current.filter((t) => t !== tag) : [...current, tag];
    onChange({ ...filter, app_tags: next });
  }

  return (
    <div className="flex flex-wrap items-center gap-3 border-b border-border bg-surface-primary px-4 py-2">
      <div className="flex items-center gap-1">
        <span className="text-xs text-text-muted">Devices:</span>
        {devices.map((d) => {
          const idStr = d.id.toString();
          const active = filter.device_ids?.includes(idStr);
          return (
            <button
              key={d.id}
              type="button"
              aria-pressed={active}
              onClick={() => toggleDevice(idStr)}
              className={`px-2 py-0.5 rounded text-xs ${
                active
                  ? "bg-accent-blue text-white"
                  : "bg-surface-tertiary text-text-secondary hover:text-text-primary"
              }`}
            >
              {d.name}
            </button>
          );
        })}
        {deviceError && (
          <span className="text-xs text-accent-red" role="alert">
            Devices unavailable: {deviceError}{" "}
            <button type="button" className="underline" onClick={() => void loadDevices()}>
              Retry
            </button>
          </span>
        )}
      </div>

      <div className="flex items-center gap-1">
        <span className="text-xs text-text-muted">Apps:</span>
        {APP_TAGS.map((tag) => {
          const active = filter.app_tags?.includes(tag);
          return (
            <button
              key={tag}
              type="button"
              aria-pressed={Boolean(active)}
              onClick={() => toggleAppTag(tag)}
              className={`px-2 py-0.5 rounded text-xs ${
                active
                  ? "bg-accent-green text-white"
                  : "bg-surface-tertiary text-text-secondary hover:text-text-primary"
              }`}
            >
              {tag}
            </button>
          );
        })}
      </div>

      <input
        type="text"
        placeholder="Host contains..."
        aria-label="Filter by host"
        value={filter.host_contains ?? ""}
        onChange={(e) => onChange({ ...filter, host_contains: e.target.value || null })}
        className="px-2 py-1 rounded bg-bg-primary border border-border text-text-primary text-sm"
        style={{ width: 160 }}
      />

      <label className="flex items-center gap-1 text-xs text-text-secondary cursor-pointer">
        <input
          type="checkbox"
          checked={filter.sync_global}
          onChange={(e) => onChange({ ...filter, sync_global: e.target.checked })}
        />
        Sync global
      </label>

      <div className="flex-1" />

      <button
        type="button"
        onClick={onRefresh}
        className="px-3 py-1 rounded text-sm bg-accent-blue text-white hover:opacity-90"
      >
        Refresh
      </button>
    </div>
  );
}
