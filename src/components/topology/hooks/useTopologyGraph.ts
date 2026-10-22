import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { TopologyFilter, TopologyGraph } from "../types";

interface UseTopologyGraphState {
  graph: TopologyGraph | null;
  loading: boolean;
  error: string | null;
  lastBuiltAt: number | null;
}

export function useTopologyGraph(filter: TopologyFilter, debounceMs = 300) {
  const [state, setState] = useState<UseTopologyGraphState>({
    graph: null,
    loading: true,
    error: null,
    lastBuiltAt: null,
  });
  const [version, setVersion] = useState(0);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const reqIdRef = useRef(0);

  const refresh = useCallback(() => setVersion((current) => current + 1), []);

  useEffect(() => {
    if (debounceRef.current) clearTimeout(debounceRef.current);

    debounceRef.current = setTimeout(async () => {
      const reqId = ++reqIdRef.current;
      setState((prev) => ({ ...prev, loading: true, error: null }));
      try {
        const graph = await invoke<TopologyGraph | null>("build_topology_graph", { filter });
        if (reqId !== reqIdRef.current) return; // stale
        if (!graph) {
          setState({
            graph: null,
            loading: false,
            error: "The topology builder returned no graph. Retry to rebuild it.",
            lastBuiltAt: null,
          });
          return;
        }
        setState({ graph, loading: false, error: null, lastBuiltAt: Date.now() });
      } catch (err) {
        if (reqId !== reqIdRef.current) return;
        setState({
          graph: null,
          loading: false,
          error: err instanceof Error ? err.message : String(err),
          lastBuiltAt: null,
        });
      }
    }, debounceMs);

    return () => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
    };
  }, [JSON.stringify(filter), debounceMs, version]);

  return { ...state, refresh };
}
