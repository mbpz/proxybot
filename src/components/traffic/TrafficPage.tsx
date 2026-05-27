import { useState, useEffect, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { FilterBar } from "./FilterBar";
import { RequestTable } from "./RequestTable";
import { RequestDetail } from "./RequestDetail";
import { ErrorBoundary } from "../ui/error-boundary";
import { SkeletonTable } from "../ui/skeleton";

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
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    loadRequests();

    // Subscribe to real-time updates
    const unlisten = listen<InterceptedRequest>("traffic-update", (event) => {
      setRequests((prev) => [event.payload, ...prev.slice(0, 999)]);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  async function loadRequests() {
    try {
      setLoading(true);
      setError(null);
      const result = await invoke<InterceptedRequest[]>("load_history", {
        filter: {},
        limit: 1000,
      });
      setRequests(result);
    } catch (err) {
      console.error("Failed to load requests:", err);
      setError(err instanceof Error ? err.message : "Failed to load requests");
    } finally {
      setLoading(false);
    }
  }

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

  return (
    <div className="flex flex-col h-screen">
      <FilterBar filters={filters} onChange={setFilters} />

      {/* Error banner */}
      {error && (
        <div className="error-banner mx-4 mt-2">
          <span className="error-banner-message">{error}</span>
          <button
            className="btn btn-sm btn-secondary"
            onClick={loadRequests}
          >
            Retry
          </button>
        </div>
      )}

      <div className="flex flex-1 overflow-hidden">
        <div
          className="w-3/5"
          style={{ borderRight: "1px solid var(--border)" }}
        >
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
                <div className="empty-state-icon">🔍</div>
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
