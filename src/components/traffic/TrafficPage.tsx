import { useState, useEffect, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { FilterBar } from "./FilterBar";
import { RequestTable } from "./RequestTable";
import { RequestDetail } from "./RequestDetail";

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
      const result = await invoke<InterceptedRequest[]>("get_requests", {
        filter: {},
        limit: 1000,
      });
      setRequests(result);
    } catch (err) {
      console.error("Failed to load requests:", err);
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

      <div className="flex flex-1 overflow-hidden">
        <div className="w-3/5 border-r overflow-hidden">
          <RequestTable
            requests={filteredRequests}
            selectedId={selectedId}
            onSelect={setSelectedId}
          />
        </div>
        <div className="w-2/5 overflow-hidden">
          {selectedRequest ? (
            <RequestDetail request={selectedRequest} />
          ) : (
            <div className="flex items-center justify-center h-full text-gray-500">
              Select a request to view details
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
