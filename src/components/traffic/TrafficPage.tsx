import { useState, useEffect, useMemo } from "react";
import { FilterBar } from "./FilterBar";
import { RequestTable } from "./RequestTable";
import { RequestDetail } from "./RequestDetail";
import { capturedRequestToListItem, normalizedRecordToListItem } from "./model";
import { FilterInput } from "../filter/FilterInput";
import { ErrorBoundary } from "../ui/error-boundary";
import { SkeletonTable } from "../ui/skeleton";
import { Button } from "../ui/Button";
import { Search, Download, Table2, Save, FolderOpen } from "lucide-react";
import { desktop } from "../../desktop/contract";
import type {
  FilterPreset,
  InterceptedRequest,
  NormalizedRecord,
} from "../../generated/desktop-contract";

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
  const [dslExpr, setDslExpr] = useState("");
  const [presets, setPresets] = useState<FilterPreset[]>([]);
  const [loading, setLoading] = useState(true);
  const [harExporting, setHarExporting] = useState(false);
  const [harName, setHarName] = useState("");
  const [showHarDialog, setShowHarDialog] = useState(false);
  const [normalizedView, setNormalizedView] = useState(false);
  const [normalizedData, setNormalizedData] = useState<NormalizedRecord[]>([]);
  const [normPage, setNormPage] = useState(0);
  const [normTotal, setNormTotal] = useState(0);
  const [normLoading, setNormLoading] = useState(false);

  async function loadPresets() {
    try {
      setPresets(await desktop.call("list_filter_presets", {}));
    } catch (e) {
      console.error("Failed to load presets:", e);
    }
  }

  useEffect(() => {
    loadPresets();
  }, []);

  useEffect(() => {
    // Start with empty list - requests will come via events
    setLoading(false);

    const subscription = desktop.subscribe("intercepted-request", {
      next: (request) => setRequests((current) => [request, ...current].slice(0, 999)),
      error: (error) => console.error("Invalid intercepted request event:", error),
    });
    void subscription.ready.catch((error) => console.error("Traffic subscription failed:", error));

    return () => subscription.dispose();
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
    if (filters.appTag) {
      result = result.filter((r) => r.app_name === filters.appTag);
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

  // Apply DSL filter on top of the simple FilterBar result. When the
  // DSL is empty, dslFilteredRequests == filteredRequests so behaviour
  // is unchanged.
  const [dslFilteredRequests, setDslFilteredRequests] =
    useState<InterceptedRequest[]>(filteredRequests);

  useEffect(() => {
    if (!dslExpr.trim()) {
      setDslFilteredRequests(filteredRequests);
      return;
    }
    let cancelled = false;
    (async () => {
      const out: InterceptedRequest[] = [];
      for (const r of filteredRequests) {
        try {
          const matches = await desktop.call("evaluate_filter", {
            expr: dslExpr,
            request: r,
          });
          if (matches) out.push(r);
        } catch {
          // Skip rows that fail to evaluate (e.g. parse error mid-typing).
        }
      }
      if (!cancelled) setDslFilteredRequests(out);
    })();
    return () => {
      cancelled = true;
    };
  }, [filteredRequests, dslExpr]);

  const displayedRequests = useMemo(
    () =>
      normalizedView
        ? normalizedData.map(normalizedRecordToListItem)
        : dslFilteredRequests.map(capturedRequestToListItem),
    [normalizedView, normalizedData, dslFilteredRequests],
  );

  const selectedRequest = useMemo(
    () => displayedRequests.find((request) => request.id === selectedId),
    [displayedRequests, selectedId],
  );

  async function loadNormalized(page = normPage) {
    try {
      setNormLoading(true);
      const result = await desktop.call("get_traffic_page", { page, pageSize: 50 });
      setNormalizedData(result.records);
      setNormTotal(result.total);
    } catch (err) {
      alert("Normalized load failed: " + String(err));
    } finally {
      setNormLoading(false);
    }
  }

  async function toggleNormalized() {
    const next = !normalizedView;
    setNormalizedView(next);
    if (next) loadNormalized();
  }

  async function loadHistory() {
    try {
      setRequests(await desktop.call("load_history", {}));
    } catch (err) {
      alert("Load history failed: " + String(err));
    }
  }

  async function saveHistory() {
    try {
      await desktop.call("save_history", { requests });
      alert("Traffic history saved");
    } catch (err) {
      alert("Save failed: " + String(err));
    }
  }

  async function exportHar() {
    if (!harName) return;
    try {
      setHarExporting(true);
      const har = await desktop.call("export_har", { sessionName: harName });
      const path = await desktop.call("save_har_file", {
        harJson: JSON.stringify(har),
        sessionName: harName,
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
      <FilterInput
        value={dslExpr}
        onChange={setDslExpr}
        presets={presets}
        onSelectPreset={(p) => setDslExpr(p.expr)}
        onPresetsChange={loadPresets}
      />
      <FilterBar filters={filters} onChange={setFilters} />

      {/* Toolbar */}
      <div className="flex items-center gap-2 px-4 py-2 border-b border-border bg-surface-primary">
        <span className="text-xs text-text-muted">{dslFilteredRequests.length} requests</span>
        <div className="flex-1" />
        <Button variant="secondary" size="sm" onClick={loadHistory}>
          <FolderOpen size={14} /> Load
        </Button>
        <Button variant="secondary" size="sm" onClick={saveHistory} disabled={requests.length === 0}>
          <Save size={14} /> Save
        </Button>
        <Button variant="secondary" size="sm" onClick={toggleNormalized}>
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
        <div className="w-3/5 border-r border-border flex flex-col">
          <div className="flex-1 overflow-hidden">
            <ErrorBoundary>
              {loading || normLoading ? (
                <SkeletonTable rows={10} />
              ) : (
                <RequestTable
                  requests={displayedRequests}
                  selectedId={selectedId}
                  onSelect={setSelectedId}
                />
              )}
            </ErrorBoundary>
          </div>
          {normalizedView && normTotal > 0 && (
            <div className="flex items-center justify-between px-4 py-2 border-t border-border bg-surface-primary text-xs text-text-muted">
              <span>Page {normPage + 1} of {Math.ceil(normTotal / 50)} ({normTotal} total)</span>
              <div className="flex gap-1">
                <Button variant="secondary" size="sm" disabled={normPage <= 0} onClick={() => { const page = normPage - 1; setNormPage(page); void loadNormalized(page); }}>Prev</Button>
                <Button variant="secondary" size="sm" disabled={(normPage + 1) * 50 >= normTotal} onClick={() => { const page = normPage + 1; setNormPage(page); void loadNormalized(page); }}>Next</Button>
              </div>
            </div>
          )}
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
