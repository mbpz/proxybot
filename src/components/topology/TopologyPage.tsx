import { useCallback, useState } from "react";
import { Network } from "lucide-react";
import { TopologyFilter } from "./types";
import { useTopologyGraph } from "./hooks/useTopologyGraph";
import { TopologyCanvas } from "./TopologyCanvas";
import { TopologyFilter as TopologyFilterBar } from "./TopologyFilter";
import { TopologyDetail } from "./TopologyDetail";

const INITIAL_FILTER: TopologyFilter = {
  device_ids: null,
  app_tags: null,
  host_contains: null,
  time_window: { type: "session" },
  sync_global: false,
};

export function TopologyPage() {
  const [filter, setFilter] = useState<TopologyFilter>(INITIAL_FILTER);
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [refreshKey, setRefreshKey] = useState(0);
  const { graph, loading, error } = useTopologyGraph(filter, 300);

  const handleRefresh = useCallback(() => setRefreshKey((k) => k + 1), []);

  return (
    <div className="p-6 h-screen flex flex-col">
      <h1 className="text-2xl font-bold mb-4 flex items-center gap-2">
        <Network size={24} className="text-accent-blue" />
        Topology
      </h1>
      <div className="flex-1 flex flex-col panel overflow-hidden">
        <TopologyFilterBar filter={filter} onChange={setFilter} onRefresh={handleRefresh} />
        <div className="flex flex-1 overflow-hidden">
          <TopologyCanvas
            graph={graph ?? { nodes: [], edges: [], meta: emptyMeta() }}
            loading={loading || refreshKey > 0 && !graph}
            error={error}
            onNodeClick={setSelectedNodeId}
            onRefresh={handleRefresh}
          />
          <TopologyDetail
            nodeId={selectedNodeId}
            filter={filter}
            onClose={() => setSelectedNodeId(null)}
          />
        </div>
      </div>
    </div>
  );
}

function emptyMeta() {
  return {
    total_requests: 0,
    total_bytes: 0,
    device_count: 0,
    app_count: 0,
    host_count: 0,
    time_range: [0, 0] as [number, number],
    built_at: 0,
  };
}
