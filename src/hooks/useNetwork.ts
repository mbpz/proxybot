import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { NetworkInfo } from "../types";

export function useNetwork() {
  const [networkInfo, setNetworkInfo] = useState<NetworkInfo | null>(null);
  const [pfEnabled, setPfEnabled] = useState(false);
  const [pfLoading, setPfLoading] = useState(false);

  useEffect(() => {
    invoke<NetworkInfo>("get_network_info")
      .then(setNetworkInfo)
      .catch((e) => console.error("Failed to get network info:", e));
    invoke<boolean>("is_pf_enabled")
      .then(setPfEnabled)
      .catch((e) => console.error("Failed to get pf status:", e));
  }, []);

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
    networkInfo, pfEnabled, pfLoading,
    enablePf, disablePf,
  };
}
