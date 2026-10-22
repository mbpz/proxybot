import { useState, useEffect, useMemo } from "react";
import { useSearchParams } from "react-router-dom";
import { FilterBar } from "./FilterBar";
import { RequestTable } from "./RequestTable";
import { RequestDetail } from "./RequestDetail";
import { CaptureEmptyState } from "./CaptureEmptyState";
import { capturedRequestToListItem, normalizedRecordToListItem } from "./model";
import { FilterInput } from "../filter/FilterInput";
import { ErrorBoundary } from "../ui/error-boundary";
import { SkeletonTable } from "../ui/skeleton";
import { Button } from "../ui/Button";
import { Dialog } from "../ui/Dialog";
import { EmptyState, ErrorState, InlineNotice } from "../ui/states";
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
  const [searchParams] = useSearchParams();
  const [requests, setRequests] = useState<InterceptedRequest[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [filters, setFilters] = useState<FilterState>(() => {
    const host = searchParams.get("host");
    const app = searchParams.get("app");
    return {
      ...(host ? { host } : {}),
      ...(app ? { appTag: app } : {}),
    };
  });
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
  const [queryError, setQueryError] = useState<string | null>(null);
  const [notice, setNotice] = useState<{
    variant: "success" | "error";
    message: string;
  } | null>(null);

  async function loadPresets() {
    try {
      setPresets(await desktop.call("list_filter_presets", {}));
    } catch (e) {
      setNotice({ variant: "error", message: `Could not load filter presets: ${String(e)}` });
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
      error: (error) =>
        setNotice({ variant: "error", message: `Capture event stream failed: ${error.message}` }),
    });
    void subscription.ready.catch((error) =>
      setNotice({ variant: "error", message: `Capture event stream failed: ${String(error)}` }),
    );

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
          if (active) {
            setResultPage(page);
            setQueryError(null);
          }
        } catch (error) {
          // Keep the previous result set visible, but make the failure explicit
          // and retryable instead of silently rendering stale/empty data.
          if (active) {
            setQueryError(error instanceof Error ? error.message : String(error));
          }
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

  const hasActiveQuery = Boolean(
    dslExpr.trim() ||
      filters.method ||
      filters.host ||
      filters.appTag ||
      filters.search ||
      filters.status !== undefined,
  );

  function toggleNormalized() {
    setNormalizedView((current) => !current);
  }

  async function loadHistory() {
    try {
      const history = await desktop.call("load_history", {});
      setRequests(history);
      setHistoryRecords(history);
      setQueryPage(0);
      setNotice({ variant: "success", message: `Loaded ${history.length} stored requests` });
    } catch (err) {
      setNotice({ variant: "error", message: `Load history failed: ${String(err)}` });
    }
  }

  async function saveHistory() {
    try {
      await desktop.call("save_history", { requests: historyRecords ?? requests });
      setNotice({ variant: "success", message: "Traffic history saved" });
    } catch (err) {
      setNotice({ variant: "error", message: `Save failed: ${String(err)}` });
    }
  }

  async function exportHar() {
    if (!harName) return;
    try {
      setHarExporting(true);
      setNotice(null);
      const har = await desktop.call("export_har", { sessionName: harName });
      const path = await desktop.call("save_har_file", {
        harJson: JSON.stringify(har),
        sessionName: harName,
      });
      setShowHarDialog(false);
      setHarName("");
      setNotice({ variant: "success", message: `HAR exported to: ${path}` });
    } catch (err) {
      setNotice({ variant: "error", message: `Export failed: ${String(err)}` });
    } finally {
      setHarExporting(false);
    }
  }

  return (
    <div className="flex h-full min-h-0 flex-col">
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
        <span className="text-xs text-text-muted">
          {resultPage.total} requests{queryLoading ? " · filtering..." : ""}
        </span>
        <div className="flex-1" />
        <Button variant="secondary" size="sm" onClick={() => void loadHistory()}>
          <FolderOpen size={14} /> Load
        </Button>
        <Button variant="secondary" size="sm" onClick={() => void saveHistory()} disabled={requests.length === 0}>
          <Save size={14} /> Save
        </Button>
        <Button variant="secondary" size="sm" onClick={toggleNormalized}>
          <Table2 size={14} />
          {normalizedView ? "Raw" : "Normalized"}
        </Button>
        <Button
          variant="secondary"
          size="sm"
          onClick={() => setShowHarDialog(true)}
          disabled={resultPage.total === 0}
        >
          <Download size={14} />
          Export HAR
        </Button>
      </div>

      {notice && (
        <div className="px-4 pt-2">
          <InlineNotice variant={notice.variant} onDismiss={() => setNotice(null)}>
            {notice.message}
          </InlineNotice>
        </div>
      )}
      {queryError && (
        <div className="px-4 pt-2">
          <ErrorState
            title="Could not load requests"
            message={queryError}
            onRetry={() => setRefreshVersion((version) => version + 1)}
          />
        </div>
      )}

      <Dialog
        open={showHarDialog}
        title="Export HAR"
        onClose={() => setShowHarDialog(false)}
        footer={
          <>
            <button
              type="button"
              className="btn btn-ghost btn-sm"
              onClick={() => setShowHarDialog(false)}
            >
              Cancel
            </button>
            <button
              type="button"
              className="btn btn-primary btn-sm"
              onClick={() => void exportHar()}
              disabled={harExporting || !harName}
            >
              {harExporting ? "Exporting..." : "Export"}
            </button>
          </>
        }
      >
        <label className="mb-1 block text-sm text-text-muted" htmlFor="har-session-name">
          Session name
        </label>
        <input
          id="har-session-name"
          type="text"
          value={harName}
          onChange={(event) => setHarName(event.target.value)}
          placeholder="Session name..."
          className="w-full"
          onKeyDown={(event) => {
            if (event.key === "Enter" && harName && !harExporting) void exportHar();
          }}
        />
      </Dialog>

      <div className="flex min-h-0 flex-1 overflow-hidden">
        <div className="flex w-3/5 flex-col border-r border-border">
          <div className="min-h-0 flex-1 overflow-hidden">
            <ErrorBoundary>
              {loading ? (
                <SkeletonTable rows={10} />
              ) : resultPage.total === 0 ? (
                hasActiveQuery ? (
                  <EmptyState
                    icon={<Search size={40} />}
                    title="No requests match the current filters"
                    description="Adjust or clear the filter expression to see captured requests."
                    action={
                      <Button
                        variant="secondary"
                        size="sm"
                        onClick={() => {
                          setDslExpr("");
                          setFilters({});
                          setQueryPage(0);
                        }}
                      >
                        Clear filters
                      </Button>
                    }
                  />
                ) : (
                  <CaptureEmptyState />
                )
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
            <div className="flex items-center justify-between border-t border-border bg-surface-primary px-4 py-2 text-xs text-text-muted">
              <span>
                Page {queryPage + 1} of {Math.ceil(resultPage.total / 50)} ({resultPage.total} total)
              </span>
              <div className="flex gap-1">
                <Button
                  variant="secondary"
                  size="sm"
                  disabled={queryPage <= 0}
                  onClick={() => setQueryPage((page) => Math.max(0, page - 1))}
                >
                  Prev
                </Button>
                <Button
                  variant="secondary"
                  size="sm"
                  disabled={!resultPage.has_more}
                  onClick={() => setQueryPage((page) => page + 1)}
                >
                  Next
                </Button>
              </div>
            </div>
          )}
        </div>
        <div className="w-2/5 overflow-hidden">
          <ErrorBoundary>
            {selectedRequest ? (
              <RequestDetail request={selectedRequest} />
            ) : (
              <EmptyState
                icon={<Search size={40} />}
                title="No request selected"
                description="Select a request in the list (click or press Enter) to inspect it."
              />
            )}
          </ErrorBoundary>
        </div>
      </div>
    </div>
  );
}
