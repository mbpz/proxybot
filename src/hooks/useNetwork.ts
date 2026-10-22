import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { NetworkInfo } from "../types";

export function useNetwork() {
  const [networkInfo, setNetworkInfo] = useState<NetworkInfo | null>(null);
  const [pfEnabled, setPfEnabled] = useState(false);
  const [pfLoading, setPfLoading] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);

  const reload = useCallback(async () => {
    setLoadError(null);
    try {
      const [info, enabled] = await Promise.all([
        invoke<NetworkInfo>("get_network_info"),
        invoke<boolean>("is_pf_enabled"),
      ]);
      setNetworkInfo(info);
      setPfEnabled(enabled);
    } catch (cause) {
      setLoadError(cause instanceof Error ? cause.message : String(cause));
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  const enablePf = useCallback(async (onError: (msg: string) => void) => {
    try {
      setPfLoading(true);
      onError("");
      const alreadyEnabled = await invoke<boolean>("is_pf_enabled");
      if (alreadyEnabled) {
        setPfEnabled(true);
        return;
      }
      await invoke<NetworkInfo>("get_network_info");
      await invoke<string>("setup_pf");
      setPfEnabled(true);
    } catch (e) {
      onError(String(e));
    } finally {
      setPfLoading(false);
    }
  }, []);

  const disablePf = useCallback(async (onError: (msg: string) => void) => {
    try {
      setPfLoading(true);
      onError("");
      await invoke<void>("teardown_pf");
      setPfEnabled(false);
    } catch (e) {
      onError(String(e));
    } finally {
      setPfLoading(false);
    }
  }, []);

  return {
    networkInfo,
    pfEnabled,
    pfLoading,
    loadError,
    reload,
    enablePf,
    disablePf,
  };
}
