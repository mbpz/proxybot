import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Download, RefreshCw, Key, ArrowLeft } from "lucide-react";
import { Link } from "react-router-dom";
import { ConfirmDialog } from "../ui/Dialog";
import { EmptyState, ErrorState, InlineNotice, LoadingState } from "../ui/states";
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
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [confirmRegenerate, setConfirmRegenerate] = useState(false);

  useEffect(() => {
    void loadCaMetadata();
  }, []);

  async function loadCaMetadata() {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<CaMetadata | null>("get_ca_metadata");
      setCaMetadata(result);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setLoading(false);
    }
  }

  async function handleExport() {
    setExporting(true);
    setError(null);
    setNotice(null);
    try {
      const path = await invoke<string>("export_cert");
      setNotice(`CA exported to: ${path}`);
    } catch (err) {
      setError(`Failed to export CA: ${err}`);
    } finally {
      setExporting(false);
    }
  }

  async function handleRegenerate() {
    setConfirmRegenerate(false);
    setRegenerating(true);
    setError(null);
    setNotice(null);
    try {
      await invoke("regenerate_ca");
      setNotice("CA regenerated successfully. Reinstall it on every device.");
      await loadCaMetadata();
    } catch (err) {
      setError(`Failed to regenerate CA: ${err}`);
    } finally {
      setRegenerating(false);
    }
  }

  function formatDate(timestamp: number): string {
    const milliseconds = timestamp > 1e12 ? timestamp : timestamp * 1000;
    return new Date(milliseconds).toLocaleString();
  }

  if (loading) {
    return (
      <div className="p-6">
        <LoadingState label="Loading certificate authority..." />
      </div>
    );
  }

  return (
    <div className="p-6">
      <h1 className="text-2xl font-bold mb-6 flex items-center gap-2">
        <Key size={24} className="text-accent-blue" />
        Advanced Certificates
      </h1>

      <p className="mb-6 max-w-2xl text-sm text-text-secondary">
        Regenerating the CA invalidates every device trust relationship. Use the guided Device
        Setup flow for temporary certificate distribution and explicit-proxy instructions.
      </p>

      <Link to="/setup" className="btn btn-secondary mb-6">
        <ArrowLeft size={15} /> Back to Device Setup
      </Link>

      {error && (
        <div className="mb-4">
          <ErrorState
            title="Certificate operation failed"
            message={error}
            onRetry={() => void loadCaMetadata()}
          />
        </div>
      )}
      {notice && (
        <div className="mb-4">
          <InlineNotice variant="success" onDismiss={() => setNotice(null)}>
            {notice}
          </InlineNotice>
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
          <EmptyState
            icon={<Key size={36} />}
            title="No CA certificate found"
            description="Starting the Capture Session or regenerating the CA creates the local root certificate. Regenerating invalidates existing device trust."
          />
        )}

        <div className="flex gap-3 mt-6">
          <button
            onClick={() => void handleExport()}
            disabled={exporting || !caMetadata}
            className="btn btn-primary"
          >
            <Download size={16} />
            {exporting ? "Exporting..." : "Export CA"}
          </button>
          <button
            onClick={() => setConfirmRegenerate(true)}
            disabled={regenerating}
            className="btn btn-danger"
          >
            <RefreshCw size={16} />
            {regenerating ? "Regenerating..." : "Regenerate CA"}
          </button>
        </div>
      </div>

      <DecryptionRules />

      <ConfirmDialog
        open={confirmRegenerate}
        title="Regenerate CA"
        message="This will regenerate the CA. Existing certificates on every device will be invalidated and must be reinstalled. Continue?"
        confirmLabel="Regenerate CA"
        destructive
        onConfirm={() => void handleRegenerate()}
        onCancel={() => setConfirmRegenerate(false)}
      />
    </div>
  );
}
