import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { UpdateSettings } from "./UpdateSettings";
import { Smartphone, Monitor } from "lucide-react";

interface ClientInfo {
  id: string;
  name: string;
  client_type: string;
  installed: boolean;
  proxy_configured: boolean;
  config_instructions: string;
}

export function ClientSetup() {
  const [clients, setClients] = useState<ClientInfo[]>([]);
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [copiedId, setCopiedId] = useState<string | null>(null);

  useEffect(() => {
    loadClients();
  }, []);

  async function loadClients() {
    try {
      const result = await invoke<ClientInfo[]>("detect_clients");
      setClients(result);
    } catch (err) {
      console.error("Failed to detect clients:", err);
    }
  }

  async function handleCopyCommand(clientId: string) {
    try {
      const cmd = await invoke<string>("get_proxy_config_command", { clientId });
      await navigator.clipboard.writeText(cmd);
      setCopiedId(clientId);
      setTimeout(() => setCopiedId(null), 2000);
    } catch (err) {
      console.error("Failed to copy:", err);
    }
  }

  const browsers = clients.filter((c) => c.client_type === "browser");
  const runtimes = clients.filter((c) => c.client_type === "runtime");

  return (
    <div className="p-6 max-w-2xl">
      <h1 className="text-2xl font-bold mb-2 flex items-center gap-2">
        <Monitor size={24} className="text-accent-blue" />
        Client Setup
      </h1>
      <p className="text-text-muted mb-6">
        Configure your browsers and development tools to use ProxyBot as their HTTP proxy.
      </p>

      {/* Browsers */}
      <h2 className="text-lg font-medium mb-3 flex items-center gap-2">
        <Monitor size={18} className="text-text-secondary" />
        Browsers
      </h2>
      <div className="space-y-2 mb-8">
        {browsers.map((client) => (
          <div key={client.id} className="card">
            <button
              onClick={() => setExpandedId(expandedId === client.id ? null : client.id)}
              className="w-full flex items-center justify-between hover:bg-surface-elevated rounded p-1 -m-1 transition-colors"
            >
              <div className="flex items-center gap-3">
                <span className={`w-2 h-2 rounded-full ${client.installed ? "bg-accent-green" : "bg-text-muted"}`} />
                <span className="font-medium">{client.name}</span>
                {!client.installed && <span className="text-xs text-text-muted">(not installed)</span>}
              </div>
              <span className="text-text-muted">{expandedId === client.id ? "▲" : "▼"}</span>
            </button>
            {expandedId === client.id && (
              <div className="mt-3 pt-3 border-t border-border">
                <p className="text-sm text-text-secondary mb-2">{client.config_instructions}</p>
                <button
                  onClick={() => handleCopyCommand(client.id)}
                  className="btn btn-primary btn-sm"
                >
                  {copiedId === client.id ? "Copied!" : "Copy Command"}
                </button>
              </div>
            )}
          </div>
        ))}
      </div>

      {/* Runtimes */}
      <h2 className="text-lg font-medium mb-3 flex items-center gap-2">
        <Smartphone size={18} className="text-text-secondary" />
        Runtimes
      </h2>
      <div className="space-y-2">
        {runtimes.map((client) => (
          <div key={client.id} className="card">
            <button
              onClick={() => setExpandedId(expandedId === client.id ? null : client.id)}
              className="w-full flex items-center justify-between hover:bg-surface-elevated rounded p-1 -m-1 transition-colors"
            >
              <div className="flex items-center gap-3">
                <span className={`w-2 h-2 rounded-full ${client.installed ? "bg-accent-green" : "bg-text-muted"}`} />
                <span className="font-medium">{client.name}</span>
                {!client.installed && <span className="text-xs text-text-muted">(not installed)</span>}
              </div>
              <span className="text-text-muted">{expandedId === client.id ? "▲" : "▼"}</span>
            </button>
            {expandedId === client.id && (
              <div className="mt-3 pt-3 border-t border-border">
                <pre className="text-xs font-mono text-text-secondary mb-2 whitespace-pre-wrap">
                  {client.config_instructions}
                </pre>
                <button
                  onClick={() => handleCopyCommand(client.id)}
                  className="btn btn-primary btn-sm"
                >
                  {copiedId === client.id ? "Copied!" : "Copy Command"}
                </button>
              </div>
            )}
          </div>
        ))}
      </div>

      {/* Update Settings */}
      <div className="mt-8">
        <UpdateSettings />
      </div>
    </div>
  );
}
