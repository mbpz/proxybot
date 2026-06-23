import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { safeInvoke } from "../../utils/safeInvoke";
import { Download, RefreshCw, Key, AlertCircle, Server } from "lucide-react";
import { Button } from "../ui/Button";
import { DecryptionRules } from "./DecryptionRules";

interface CaMetadata {
  created_at: number;
  serial: string;
  fingerprint?: string;
}

export function CertsPage() {
  const [caMetadata, setCaMetadata] = useState<CaMetadata | null>(null);
  const [loading, setLoading] = useState(true);
  const [exporting, setExporting] = useState(false);
  const [regenerating, setRegenerating] = useState(false);
  const [caServerRunning, setCaServerRunning] = useState(false);
  const [caServerUrl, setCaServerUrl] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    loadCaMetadata();
  }, []);

  async function loadCaMetadata() {
    const result = await safeInvoke<CaMetadata | null>("get_ca_metadata");
    setCaMetadata(result);
    setLoading(false);
  }

  async function handleExport() {
    setExporting(true);
    setError(null);
    try {
      const path = await invoke<string>("export_cert");
      alert(`CA exported to: ${path}`);
    } catch (err) {
      setError(`Failed to export CA: ${err}`);
    } finally {
      setExporting(false);
    }
  }

  async function toggleCaServer() {
    try {
      setError(null);
      if (caServerRunning) {
        setCaServerRunning(false);
        setCaServerUrl(null);
      } else {
        const certPath = await invoke<string>("get_ca_cert_path");
        const netInfo = await invoke<{ lan_ip: string }>("get_network_info").catch(() => ({ lan_ip: "127.0.0.1" }));
        const url = await invoke<string>("start_cert_server", {
          cert_path: certPath,
          local_ip: netInfo.lan_ip,
        });
        setCaServerRunning(true);
        setCaServerUrl(url);
      }
    } catch (err) {
      setError(`CA Server error: ${err}`);
    }
  }

  async function handleRegenerate() {
    if (!confirm("This will regenerate the CA. Existing certificates will be invalidated. Continue?")) {
      return;
    }
    setRegenerating(true);
    setError(null);
    try {
      await invoke("regenerate_ca");
      alert("CA regenerated successfully");
      loadCaMetadata();
    } catch (err) {
      setError(`Failed to regenerate CA: ${err}`);
    } finally {
      setRegenerating(false);
    }
  }

  function formatDate(timestamp: number): string {
    return new Date(timestamp * 1000).toLocaleString();
  }

  if (loading) {
    return (
      <div className="p-6">
        <div className="card">
          <div className="skeleton-row">
            <div className="skeleton-cell lg skeleton" />
          </div>
          <div className="skeleton-row">
            <div className="skeleton-cell md skeleton" />
          </div>
          <div className="skeleton-row">
            <div className="skeleton-cell sm skeleton" />
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="p-6">
      <h1 className="text-2xl font-bold mb-6 flex items-center gap-2">
        <Key size={24} className="text-accent-blue" />
        Certificates
      </h1>

      {error && (
        <div className="error-banner mb-4">
          <AlertCircle size={16} />
          <span className="error-banner-message">{error}</span>
        </div>
      )}

      <div className="card max-w-2xl">
        <h2 className="text-lg font-semibold mb-4">Root CA Certificate</h2>

        {caMetadata ? (
          <div className="space-y-3">
            <div className="flex justify-between">
              <span className="text-text-secondary">Created:</span>
              <span className="font-medium">{formatDate(caMetadata.created_at)}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-text-secondary">Serial:</span>
              <span className="font-mono text-sm">{caMetadata.serial}</span>
            </div>
            {caMetadata.fingerprint && (
              <div className="flex justify-between">
                <span className="text-text-secondary">Fingerprint:</span>
                <span className="font-mono text-xs">{caMetadata.fingerprint}</span>
              </div>
            )}
          </div>
        ) : (
          <p className="text-text-muted">No CA certificate found. Generate one to get started.</p>
        )}

        <div className="flex gap-3 mt-6">
          <button
            onClick={handleExport}
            disabled={exporting || !caMetadata}
            className="btn btn-primary"
          >
            <Download size={16} />
            {exporting ? "Exporting..." : "Export CA"}
          </button>
          <button
            onClick={handleRegenerate}
            disabled={regenerating}
            className="btn btn-danger"
          >
            <RefreshCw size={16} />
            {regenerating ? "Regenerating..." : "Regenerate CA"}
          </button>
          <Button
            variant={caServerRunning ? "danger" : "secondary"}
            size="sm"
            onClick={toggleCaServer}
          >
            <Server size={16} />
            {caServerRunning ? "Stop CA Server" : "Start CA Server"}
          </Button>
        </div>

        {caServerUrl && (
          <div className="mt-3 p-3 rounded-md text-sm font-mono" style={{
            background: "rgba(77,157,224,0.1)",
            border: "1px solid var(--accent-blue)",
            color: "var(--accent-blue)",
          }}>
            CA Server: {caServerUrl}
          </div>
        )}
      </div>

      <DecryptionRules />
    </div>
  );
}
