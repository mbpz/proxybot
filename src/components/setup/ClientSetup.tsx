import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { UpdateSettings } from "./UpdateSettings";

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
      <h1 className="text-2xl font-bold mb-2">Client Setup</h1>
      <p className="text-gray-500 mb-6">
        Configure your browsers and development tools to use ProxyBot as their HTTP proxy.
      </p>

      {/* Browsers */}
      <h2 className="text-lg font-medium mb-3">Browsers</h2>
      <div className="space-y-2 mb-8">
        {browsers.map((client) => (
          <div key={client.id} className="border rounded-lg">
            <button
              onClick={() => setExpandedId(expandedId === client.id ? null : client.id)}
              className="w-full flex items-center justify-between px-4 py-3 hover:bg-gray-50"
            >
              <div className="flex items-center gap-3">
                <span className={`w-2 h-2 rounded-full ${client.installed ? "bg-green-500" : "bg-gray-300"}`} />
                <span className="font-medium">{client.name}</span>
                {!client.installed && <span className="text-xs text-gray-400">(not installed)</span>}
              </div>
              <span className="text-gray-400">{expandedId === client.id ? "\u25B2" : "\u25BC"}</span>
            </button>
            {expandedId === client.id && (
              <div className="px-4 py-3 border-t bg-gray-50">
                <p className="text-sm text-gray-600 mb-2">{client.config_instructions}</p>
                <button
                  onClick={() => handleCopyCommand(client.id)}
                  className="px-3 py-1 text-xs bg-blue-600 text-white rounded hover:bg-blue-700"
                >
                  {copiedId === client.id ? "Copied!" : "Copy Command"}
                </button>
              </div>
            )}
          </div>
        ))}
      </div>

      {/* Runtimes */}
      <h2 className="text-lg font-medium mb-3">Runtimes</h2>
      <div className="space-y-2">
        {runtimes.map((client) => (
          <div key={client.id} className="border rounded-lg">
            <button
              onClick={() => setExpandedId(expandedId === client.id ? null : client.id)}
              className="w-full flex items-center justify-between px-4 py-3 hover:bg-gray-50"
            >
              <div className="flex items-center gap-3">
                <span className={`w-2 h-2 rounded-full ${client.installed ? "bg-green-500" : "bg-gray-300"}`} />
                <span className="font-medium">{client.name}</span>
                {!client.installed && <span className="text-xs text-gray-400">(not installed)</span>}
              </div>
              <span className="text-gray-400">{expandedId === client.id ? "\u25B2" : "\u25BC"}</span>
            </button>
            {expandedId === client.id && (
              <div className="px-4 py-3 border-t bg-gray-50">
                <pre className="text-xs font-mono text-gray-700 mb-2 whitespace-pre-wrap">
                  {client.config_instructions}
                </pre>
                <button
                  onClick={() => handleCopyCommand(client.id)}
                  className="px-3 py-1 text-xs bg-blue-600 text-white rounded hover:bg-blue-700"
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
