import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Download, RefreshCw } from "lucide-react";

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

  useEffect(() => {
    loadCaMetadata();
  }, []);

  async function loadCaMetadata() {
    try {
      const result = await invoke<CaMetadata | null>("get_ca_metadata");
      setCaMetadata(result);
    } catch (err) {
      console.error("Failed to load CA metadata:", err);
    } finally {
      setLoading(false);
    }
  }

  async function handleExport() {
    setExporting(true);
    try {
      const path = await invoke<string>("export_cert");
      alert(`CA exported to: ${path}`);
    } catch (err) {
      console.error("Failed to export CA:", err);
      alert("Failed to export CA");
    } finally {
      setExporting(false);
    }
  }

  async function handleRegenerate() {
    if (!confirm("This will regenerate the CA. Existing certificates will be invalidated. Continue?")) {
      return;
    }
    setRegenerating(true);
    try {
      await invoke("regenerate_ca");
      alert("CA regenerated successfully");
      loadCaMetadata();
    } catch (err) {
      console.error("Failed to regenerate CA:", err);
      alert("Failed to regenerate CA");
    } finally {
      setRegenerating(false);
    }
  }

  function formatDate(timestamp: number): string {
    return new Date(timestamp * 1000).toLocaleString();
  }

  if (loading) {
    return <div className="p-6">Loading...</div>;
  }

  return (
    <div className="p-6">
      <h1 className="text-2xl font-bold mb-6">Certificates</h1>

      <div className="bg-white rounded-lg shadow p-6 max-w-2xl">
        <h2 className="text-lg font-semibold mb-4">Root CA Certificate</h2>

        {caMetadata ? (
          <div className="space-y-3">
            <div className="flex justify-between">
              <span className="text-gray-600">Created:</span>
              <span className="font-medium">{formatDate(caMetadata.created_at)}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-600">Serial:</span>
              <span className="font-mono text-sm">{caMetadata.serial}</span>
            </div>
            {caMetadata.fingerprint && (
              <div className="flex justify-between">
                <span className="text-gray-600">Fingerprint:</span>
                <span className="font-mono text-xs">{caMetadata.fingerprint}</span>
              </div>
            )}
          </div>
        ) : (
          <p className="text-gray-500">No CA certificate found. Generate one to get started.</p>
        )}

        <div className="flex gap-3 mt-6">
          <button
            onClick={handleExport}
            disabled={exporting || !caMetadata}
            className="flex items-center gap-2 px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 disabled:opacity-50"
          >
            <Download size={16} />
            {exporting ? "Exporting..." : "Export CA"}
          </button>
          <button
            onClick={handleRegenerate}
            disabled={regenerating}
            className="flex items-center gap-2 px-4 py-2 bg-orange-600 text-white rounded hover:bg-orange-700 disabled:opacity-50"
          >
            <RefreshCw size={16} />
            {regenerating ? "Regenerating..." : "Regenerate CA"}
          </button>
        </div>
      </div>
    </div>
  );
}