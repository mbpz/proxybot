// React context for SSL Bypass page state.
// Shared between SslBypassPage, DeviceSelector, ScriptList, etc.

import {
  createContext,
  useContext,
  useEffect,
  useState,
  ReactNode,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export interface DeviceInfo {
  id: string;
  name: string;
  device_type: "Usb" | "Remote" | "Local";
  is_connected: boolean;
}

export interface ProcessInfo {
  pid: number;
  name: string;
  identifier: string;
}

export interface BypassScript {
  id: string;
  name: string;
  description: string;
  target_framework: string[];
  is_builtin: boolean;
}

export interface SessionHandle {
  session_id: string;
  device_id: string;
  pid: number;
  process_name: string;
  attached_at: number;
}

/**
 * A Frida script message streamed from the backend via the
 * `frida:message` Tauri event (spec §9.4). The backend emits one of
 * these for each `console.log()` and runtime error from a running
 * bypass script.
 */
export interface FridaMessage {
  level: string;
  payload: string;
  timestamp_ms: number;
}

interface SslBypassContext {
  devices: DeviceInfo[];
  processes: ProcessInfo[];
  scripts: BypassScript[];
  selectedDevice: string | null;
  selectedScript: string | null;
  javaInstalled: boolean;
  adbInstalled: boolean;
  /** Live log of Frida script messages, newest at the tail. */
  messages: FridaMessage[];
  refreshDevices: () => Promise<void>;
  refreshProcesses: () => Promise<void>;
  refreshScripts: () => Promise<void>;
  injectScript: (pid: number, scriptId: string) => Promise<SessionHandle | null>;
  checkPrerequisites: () => Promise<void>;
  setSelectedDevice: (id: string | null) => void;
  setSelectedScript: (id: string | null) => void;
  /** Drop all messages from the live log. */
  clearMessages: () => void;
}

const Ctx = createContext<SslBypassContext | null>(null);

export function SslBypassProvider({ children }: { children: ReactNode }) {
  const [devices, setDevices] = useState<DeviceInfo[]>([]);
  const [processes, setProcesses] = useState<ProcessInfo[]>([]);
  const [scripts, setScripts] = useState<BypassScript[]>([]);
  const [selectedDevice, setSelectedDevice] = useState<string | null>(null);
  const [selectedScript, setSelectedScript] = useState<string | null>(null);
  const [javaInstalled, setJavaInstalled] = useState(false);
  const [adbInstalled, setAdbInstalled] = useState(false);
  const [messages, setMessages] = useState<FridaMessage[]>([]);

  // Subscribe to frida:message events for the lifetime of the
  // provider. The provider is mounted once at the top of
  // SslBypassPage, so this captures every injection made on the
  // page. Cap retention at 1000 entries so a chatty script cannot
  // grow memory without bound.
  useEffect(() => {
    let disposed = false;
    const unlistenPromise = listen<FridaMessage>("frida:message", (event) => {
      setMessages((prev) => {
        const next = [...prev, event.payload];
        if (next.length > 1000) {
          return next.slice(next.length - 1000);
        }
        return next;
      });
    });
    return () => {
      disposed = true;
      unlistenPromise.then((f) => f()).catch(() => {
        // Listener may already be gone if the provider unmounted
        // before the listen() promise resolved.
        if (!disposed) {
          /* swallow */
        }
      });
    };
  }, []);

  async function refreshDevices() {
    try {
      const result = await invoke<DeviceInfo[]>("frida_list_devices");
      setDevices(result);
    } catch (e) {
      console.error("Failed to list devices:", e);
    }
  }

  async function refreshProcesses() {
    if (!selectedDevice) return;
    try {
      const result = await invoke<ProcessInfo[]>("frida_list_processes", {
        deviceId: selectedDevice,
      });
      setProcesses(result);
    } catch (e) {
      console.error("Failed to list processes:", e);
    }
  }

  async function refreshScripts() {
    try {
      const result = await invoke<BypassScript[]>("list_bypass_scripts");
      setScripts(result);
    } catch (e) {
      console.error("Failed to list scripts:", e);
    }
  }

  async function injectScript(pid: number, scriptId: string) {
    if (!selectedDevice) return null;
    try {
      return await invoke<SessionHandle>("frida_inject_script", {
        deviceId: selectedDevice,
        pid,
        scriptId,
      });
    } catch (e) {
      console.error("Failed to inject script:", e);
      return null;
    }
  }

  async function checkPrerequisites() {
    try {
      const java = await invoke<boolean>("check_java_installed");
      const adb = await invoke<boolean>("check_adb_installed");
      setJavaInstalled(java);
      setAdbInstalled(adb);
    } catch (e) {
      console.error("Failed to check prerequisites:", e);
    }
  }

  function clearMessages() {
    setMessages([]);
  }

  return (
    <Ctx.Provider
      value={{
        devices,
        processes,
        scripts,
        selectedDevice,
        selectedScript,
        javaInstalled,
        adbInstalled,
        messages,
        refreshDevices,
        refreshProcesses,
        refreshScripts,
        injectScript,
        checkPrerequisites,
        setSelectedDevice,
        setSelectedScript,
        clearMessages,
      }}
    >
      {children}
    </Ctx.Provider>
  );
}

export function useSslBypass() {
  const ctx = useContext(Ctx);
  if (!ctx) throw new Error("useSslBypass must be used within SslBypassProvider");
  return ctx;
}