import { createContext, useCallback, useContext, useMemo, useState, type ReactNode } from "react";

export const LABS_STORAGE_KEY = "proxybot.labs.enabled";

interface LabsValue {
  enabled: boolean;
  setEnabled(value: boolean): void;
}

const DEFAULT_VALUE: LabsValue = { enabled: false, setEnabled: () => {} };

const LabsContext = createContext<LabsValue>(DEFAULT_VALUE);

function readEnabled(): boolean {
  try {
    return window.localStorage.getItem(LABS_STORAGE_KEY) === "true";
  } catch {
    return false;
  }
}

/**
 * Labs are experimental surfaces (AI, Generate, Deploy, Graph, Topology).
 * They are opt-in, persisted locally only; no backend command is involved.
 */
export function LabsProvider({ children }: { children: ReactNode }) {
  const [enabled, setEnabledState] = useState(readEnabled);

  const setEnabled = useCallback((value: boolean) => {
    setEnabledState(value);
    try {
      window.localStorage.setItem(LABS_STORAGE_KEY, String(value));
    } catch {
      // Storage can be unavailable in restricted webviews; keep in-memory state.
    }
  }, []);

  const value = useMemo(() => ({ enabled, setEnabled }), [enabled, setEnabled]);
  return <LabsContext.Provider value={value}>{children}</LabsContext.Provider>;
}

export function useLabs(): LabsValue {
  return useContext(LabsContext);
}
