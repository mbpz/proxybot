import { invoke } from "@tauri-apps/api/core";

/**
 * Safe invoke wrapper that catches "Command not found" errors gracefully.
 * Returns null on failure instead of throwing.
 */
export async function safeInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T | null> {
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    const msg = String(e);
    if (msg.includes("not found") || msg.includes("not registered")) {
      console.warn(`Command '${cmd}' not available in this build`);
    } else {
      console.error(`Command '${cmd}' failed:`, e);
    }
    return null;
  }
}

/**
 * Safe invoke that returns a default value on failure.
 */
export async function safeInvokeOr<T>(cmd: string, defaultValue: T, args?: Record<string, unknown>): Promise<T> {
  const result = await safeInvoke<T>(cmd, args);
  return result ?? defaultValue;
}
