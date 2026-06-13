import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useSslBypass } from "../../stores/sslBypassStore";

export function ApkPatcher() {
  const store = useSslBypass();
  const [apkPath, setApkPath] = useState("");
  const [patching, setPatching] = useState(false);
  const [result, setResult] = useState<string | null>(null);

  async function patch() {
    if (!apkPath || !store.selectedScript) return;
    setPatching(true);
    setResult(null);
    try {
      const output = await invoke<string>("patch_apk", {
        apkPath,
        scriptId: store.selectedScript,
      });
      setResult(`Patched: ${output}`);
    } catch (e) {
      setResult(`Error: ${e}`);
    } finally {
      setPatching(false);
    }
  }

  return (
    <div className="card mb-4">
      <h3 className="card-title text-base mb-2">APK Patcher</h3>
      <p className="text-xs text-text-muted mb-3">
        Patch an APK with Frida Gadget + selected bypass script. Requires Java + apktool.jar.
      </p>
      <input
        type="text"
        placeholder="/path/to/app.apk"
        value={apkPath}
        onChange={(e) => setApkPath(e.target.value)}
        className="w-full px-2 py-1 text-sm border border-border rounded bg-surface-primary text-text-primary mb-2"
        data-testid="ssl-bypass-apk-path"
      />
      <button
        onClick={patch}
        disabled={patching || !store.selectedScript || !apkPath}
        className="btn btn-sm btn-primary disabled:opacity-50"
        data-testid="ssl-bypass-patch-apk"
      >
        {patching ? "Patching..." : "Patch APK"}
      </button>
      {result && (
        <pre
          className="mt-3 text-xs whitespace-pre-wrap break-all bg-surface-elevated p-2 rounded"
          data-testid="ssl-bypass-patch-result"
        >
          {result}
        </pre>
      )}
    </div>
  );
}