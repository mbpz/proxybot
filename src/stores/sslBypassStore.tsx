// React context for SSL Bypass page state.
// Shared between SslBypassPage, DeviceSelector, ScriptList, etc.

import { createContext, useContext, useState, ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";

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

interface SslBypassContext {
  devices: DeviceInfo[];
  processes: ProcessInfo[];
  scripts: BypassScript[];
  selectedDevice: string | null;
  selectedScript: string | null;
  javaInstalled: boolean;
  adbInstalled: boolean;
  refreshDevices: () => Promise<void>;
  refreshProcesses: () => Promise<void>;
  refreshScripts: () => Promise<void>;
  injectScript: (pid: number, scriptId: string) => Promise<SessionHandle | null>;
  checkPrerequisites: () => Promise<void>;
  setSelectedDevice: (id: string | null) => void;
  setSelectedScript: (id: string | null) => void;
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
        refreshDevices,
        refreshProcesses,
        refreshScripts,
        injectScript,
        checkPrerequisites,
        setSelectedDevice,
        setSelectedScript,
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