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
  TrafficPage as TrafficPageResult,
  TrafficQuery,
} from "../../generated/desktop-contract";

interface FilterState {
  method?: string;
  host?: string;
  status?: number;
  appTag?: string;
  search?: string;
}

const EMPTY_PAGE: TrafficPageResult = {
  records: [],
  normalized_records: [],
  total: 0,
  page: 0,
  page_size: 50,
  has_more: false,
};

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
  const [historyRecords, setHistoryRecords] = useState<InterceptedRequest[] | null>(null);
  const [resultPage, setResultPage] = useState<TrafficPageResult>(EMPTY_PAGE);
  const [queryPage, setQueryPage] = useState(0);
  const [queryLoading, setQueryLoading] = useState(false);
  const [refreshVersion, setRefreshVersion] = useState(0);

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
    // Capture Events invalidate the persisted result set. The desktop Adapter
    // persists each request before emitting, so the debounced query can read it.
    const subscription = desktop.subscribe("intercepted-request", {
      next: (request) => {
        setRequests((current) => [request, ...current].slice(0, 999));
        setHistoryRecords((current) =>
          current === null ? null : [request, ...current].slice(0, 999),
        );
        setRefreshVersion((version) => version + 1);
      },
      error: (error) => console.error("Invalid intercepted request event:", error),
    });
    void subscription.ready.catch((error) => console.error("Traffic subscription failed:", error));

    return () => subscription.dispose();
  }, []);

  const query = useMemo<TrafficQuery>(
    () => ({
      expression: dslExpr,
      method: filters.method ?? null,
      host: filters.host ?? null,
      status: filters.status ?? null,
      application: filters.appTag ?? null,
      search: filters.search ?? null,
      order: "newest",
      page: queryPage,
      page_size: 50,
    }),
    [dslExpr, filters, queryPage],
  );

  useEffect(() => {
    let active = true;
    const handle = setTimeout(() => {
      setQueryLoading(true);
      void (async () => {
        try {
          const page = await desktop.call("get_traffic_page", {
            query,
            records: historyRecords,
          });
          if (active) setResultPage(page);
        } catch (error) {
          // FilterInput renders malformed-expression errors. Keep the previous
          // result set while the user is still editing the expression.
          console.error("Traffic query failed:", error);
        } finally {
          if (active) {
            setLoading(false);
            setQueryLoading(false);
          }
        }
      })();
    }, 100);
    return () => {
      active = false;
      clearTimeout(handle);
    };
  }, [historyRecords, query, refreshVersion]);

  const displayedRequests = useMemo(
    () =>
      normalizedView
        ? resultPage.normalized_records.map(normalizedRecordToListItem)
        : resultPage.records.map(capturedRequestToListItem),
    [normalizedView, resultPage],
  );

  const selectedRequest = useMemo(
    () => displayedRequests.find((request) => request.id === selectedId),
    [displayedRequests, selectedId],
  );

  useEffect(() => {
    if (selectedId && !displayedRequests.some((request) => request.id === selectedId)) {
      setSelectedId(null);
    }
  }, [displayedRequests, selectedId]);

  function toggleNormalized() {
    setNormalizedView((current) => !current);
  }

  async function loadHistory() {
    try {
      const history = await desktop.call("load_history", {});
      setRequests(history);
      setHistoryRecords(history);
      setQueryPage(0);
    } catch (err) {
      alert("Load history failed: " + String(err));
    }
  }

  async function saveHistory() {
    try {
      await desktop.call("save_history", { requests: historyRecords ?? requests });
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
        onChange={(expression) => {
          setDslExpr(expression);
          setQueryPage(0);
        }}
        presets={presets}
        onSelectPreset={(preset) => {
          setDslExpr(preset.expr);
          setQueryPage(0);
        }}
        onPresetsChange={loadPresets}
      />
      <FilterBar
        filters={filters}
        onChange={(nextFilters) => {
          setFilters(nextFilters);
          setQueryPage(0);
        }}
      />

      {/* Toolbar */}
      <div className="flex items-center gap-2 px-4 py-2 border-b border-border bg-surface-primary">
        <span className="text-xs text-text-muted">{resultPage.total} requests</span>
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
        <Button variant="secondary" size="sm" onClick={() => setShowHarDialog(true)} disabled={resultPage.total === 0}>
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
              {loading || queryLoading ? (
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
          {resultPage.total > 0 && (
            <div className="flex items-center justify-between px-4 py-2 border-t border-border bg-surface-primary text-xs text-text-muted">
              <span>Page {queryPage + 1} of {Math.ceil(resultPage.total / 50)} ({resultPage.total} total)</span>
              <div className="flex gap-1">
                <Button variant="secondary" size="sm" disabled={queryPage <= 0} onClick={() => setQueryPage((page) => Math.max(0, page - 1))}>Prev</Button>
                <Button variant="secondary" size="sm" disabled={!resultPage.has_more} onClick={() => setQueryPage((page) => page + 1)}>Next</Button>
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
