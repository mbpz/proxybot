import { useState, useEffect, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { FilterBar } from "./FilterBar";
import { RequestTable } from "./RequestTable";
import { RequestDetail } from "./RequestDetail";
import { ErrorBoundary } from "../ui/error-boundary";
import { SkeletonTable } from "../ui/skeleton";
import { Button } from "../ui/Button";
import { Search, Download, Table2 } from "lucide-react";

interface InterceptedRequest {
  id: string;
  method: string;
  host: string;
  path: string;
  status?: number;
  duration_ms: number;
  timestamp: number;
  app_tag?: string;
  headers: Record<string, string>;
  body?: string;
  size?: number;
}

interface FilterState {
  method?: string;
  host?: string;
  status?: number;
  appTag?: string;
  search?: string;
}

export function TrafficPage() {
  const [requests, setRequests] = useState<InterceptedRequest[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [filters, setFilters] = useState<FilterState>({});
  const [loading, setLoading] = useState(true);
  const [harExporting, setHarExporting] = useState(false);
  const [harName, setHarName] = useState("");
  const [showHarDialog, setShowHarDialog] = useState(false);
  const [normalizedView, setNormalizedView] = useState(false);

  useEffect(() => {
    // Start with empty list - requests will come via events
    setLoading(false);

    const unlistenPromise = listen<InterceptedRequest>("intercepted-request", (event) => {
      const req = event.payload;
      if (req && typeof req === "object" && req.id && req.host) {
        setRequests((prev) => [req, ...prev].slice(0, 999));
      }
    });

    return () => {
      unlistenPromise.then((fn) => fn());
    };
  }, []);

  const filteredRequests = useMemo(() => {
    let result = requests;

    if (filters.method) {
      result = result.filter((r) => r.method === filters.method);
    }
    if (filters.host) {
      const pattern = filters.host.replace(/\*/g, ".*");
      result = result.filter((r) => new RegExp(pattern).test(r.host));
    }
    if (filters.status) {
      result = result.filter((r) => r.status === filters.status);
    }
    if (filters.search) {
      const search = filters.search.toLowerCase();
      result = result.filter(
        (r) =>
          r.path.toLowerCase().includes(search) ||
          r.host.toLowerCase().includes(search)
      );
    }

    return result;
  }, [requests, filters]);

  const selectedRequest = useMemo(
    () => requests.find((r) => r.id === selectedId),
    [requests, selectedId]
  );

  async function exportHar() {
    if (!harName) return;
    try {
      setHarExporting(true);
      const har = await invoke<{ log: object }>("export_har", { session_name: harName });
      const path = await invoke<string>("save_har_file", {
        har_json: JSON.stringify(har),
        session_name: harName,
      });
      setShowHarDialog(false);
      setHarName("");
      alert(`HAR exported to: ${path}`);
    } catch (err) {
      alert("Export failed: " + String(err));
    } finally {
      setHarExporting(false);
    }
  }

  return (
    <div className="flex flex-col h-screen">
      <FilterBar filters={filters} onChange={setFilters} />

      {/* Toolbar */}
      <div className="flex items-center gap-2 px-4 py-2 border-b border-border bg-surface-primary">
        <span className="text-xs text-text-muted">{filteredRequests.length} requests</span>
        <div className="flex-1" />
        <Button variant="secondary" size="sm" onClick={() => setNormalizedView(!normalizedView)}>
          <Table2 size={14} />
          {normalizedView ? "Raw" : "Normalized"}
        </Button>
        <Button variant="secondary" size="sm" onClick={() => setShowHarDialog(true)} disabled={requests.length === 0}>
          <Download size={14} />
          Export HAR
        </Button>
      </div>

      {/* HAR Export Dialog */}
      {showHarDialog && (
        <div className="error-banner mx-4 mt-2" style={{ background: "var(--bg-tertiary)", border: "1px solid var(--accent-blue)", color: "var(--text-primary)" }}>
          <input
            type="text"
            value={harName}
            onChange={(e) => setHarName(e.target.value)}
            placeholder="Session name..."
            style={{ flex: 1 }}
            onKeyDown={(e) => e.key === "Enter" && exportHar()}
          />
          <Button variant="primary" size="sm" onClick={exportHar} disabled={harExporting || !harName}>
            {harExporting ? "Exporting..." : "Save"}
          </Button>
          <Button variant="ghost" size="sm" onClick={() => setShowHarDialog(false)}>Cancel</Button>
        </div>
      )}

      <div className="flex flex-1 overflow-hidden">
        <div className="w-3/5 border-r border-border">
          <ErrorBoundary>
            {loading ? (
              <SkeletonTable rows={10} />
            ) : (
              <RequestTable
                requests={filteredRequests}
                selectedId={selectedId}
                onSelect={setSelectedId}
              />
            )}
          </ErrorBoundary>
        </div>
        <div className="w-2/5 overflow-hidden">
          <ErrorBoundary>
            {selectedRequest ? (
              <RequestDetail request={selectedRequest} />
            ) : (
              <div className="empty-state">
                <Search size={48} className="empty-state-icon" />
                <div className="empty-state-title">No request selected</div>
                <div className="empty-state-description">
                  Click on a request in the list to view its details
                </div>
              </div>
            )}
          </ErrorBoundary>
        </div>
      </div>
    </div>
  );
}
