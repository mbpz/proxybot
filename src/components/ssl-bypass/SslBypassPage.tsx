import { useEffect } from "react";
import { SslBypassProvider, useSslBypass } from "../../stores/sslBypassStore";
import { DeviceSelector } from "./DeviceSelector";
import { ProcessList } from "./ProcessList";
import { ScriptList } from "./ScriptList";
import { FridaStatus } from "./FridaStatus";
import { ApkPatcher } from "./ApkPatcher";
import { MessageLog } from "./MessageLog";
import { Lock } from "lucide-react";

export function SslBypassPage() {
  return (
    <SslBypassProvider>
      <SslBypassPageInner />
    </SslBypassProvider>
  );
}

function SslBypassPageInner() {
  const store = useSslBypass();

  useEffect(() => {
    store.checkPrerequisites();
    store.refreshScripts();
  }, []);

  return (
    <div className="p-6 max-w-3xl">
      <h1 className="text-2xl font-bold mb-2 flex items-center gap-2">
        <Lock size={24} className="text-accent-blue" />
        SSL Bypass
      </h1>
      <p className="text-sm text-text-muted mb-6">
        Bypass SSL certificate pinning on Android apps using Frida.
      </p>

      <FridaStatus />
      <DeviceSelector />
      <ProcessList />
      <ScriptList />
      <ApkPatcher />
      <MessageLog />
    </div>
  );
}