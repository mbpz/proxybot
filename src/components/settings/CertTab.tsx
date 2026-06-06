import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { safeInvoke } from "../../utils/safeInvoke";
import { ClientSetup } from "../setup/ClientSetup";
import { Button } from "../ui/Button";
import { Key, Download, RefreshCw } from "lucide-react";
import type { CaMetadata } from "../../types";

export function CertTab() {
  const [metadata, setMetadata] = useState<CaMetadata | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    load();
  }, []);

  async function load() {
    const result = await safeInvoke<CaMetadata | null>("get_ca_metadata");
    setMetadata(result);
    setLoading(false);
  }

  async function exportCert() {
    try {
      const path = await invoke<string>("export_cert");
      alert(`CA certificate exported to: ${path}`);
    } catch (e) {
      alert(`Failed to export: ${e}`);
    }
  }

  async function regenerate() {
    if (
      !confirm(
        "Regenerating the CA will invalidate all existing certificates. Devices will need to re-trust the new CA. Continue?"
      )
    ) {
      return;
    }
    try {
      await invoke("regenerate_ca");
      await load();
      alert("CA regenerated successfully.");
    } catch (e) {
      alert(`Failed to regenerate: ${e}`);
    }
  }

  return (
    <div className="space-y-6">
      {/* CA Certificate */}
      <div className="space-y-4">
        <div className="card">
          <div className="flex items-center gap-3 mb-4">
            <Key size={20} className="text-accent-blue" />
            <div className="font-medium text-text-primary">
              Root CA Certificate
            </div>
          </div>

          {loading ? (
            <div className="text-text-muted text-sm">Loading...</div>
          ) : metadata ? (
            <div className="space-y-2 text-sm mb-4">
              <div className="flex justify-between">
                <span className="text-text-muted">Created</span>
                <span className="text-text-primary font-mono">
                  {new Date(metadata.created_at * 1000).toLocaleDateString()}
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-text-muted">Serial</span>
                <span className="text-text-primary font-mono text-xs">
                  {metadata.serial}
                </span>
              </div>
            </div>
          ) : (
            <div className="text-text-muted text-sm mb-4">
              No CA found — one will be generated on first proxy start.
            </div>
          )}

          <div className="flex gap-2">
            <Button variant="secondary" size="sm" onClick={exportCert}>
              <Download size={14} className="mr-1" />
              Export CA
            </Button>
            <Button variant="danger" size="sm" onClick={regenerate}>
              <RefreshCw size={14} className="mr-1" />
              Regenerate
            </Button>
          </div>
        </div>
      </div>

      {/* Client Setup */}
      <div>
        <h3 className="text-sm font-semibold text-text-secondary uppercase tracking-wide mb-3">
          Client Setup
        </h3>
        <ClientSetup />
      </div>
    </div>
  );
}
