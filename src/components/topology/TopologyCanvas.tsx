import { useState } from "react";
import { TopologyGraph, ViewMode } from "./types";
import { RadialView } from "./views/RadialView";
import { LayeredView } from "./views/LayeredView";
import { GroupedView } from "./views/GroupedView";

interface Props {
  graph: TopologyGraph;
  loading: boolean;
  error: string | null;
  onNodeClick: (nodeId: string) => void;
  onRefresh: () => void;
}

const TABS: { mode: ViewMode; label: string }[] = [
  { mode: "radial", label: "Radial" },
  { mode: "layered", label: "Layered" },
  { mode: "grouped", label: "Grouped" },
];

export function TopologyCanvas({ graph, loading, error, onNodeClick, onRefresh }: Props) {
  const [view, setView] = useState<ViewMode>("radial");

  return (
    <div className="flex flex-col flex-1 overflow-hidden">
      <div className="flex items-center gap-2 px-4 py-2 border-b border-border bg-surface-primary">
        {TABS.map((t) => (
          <button
            key={t.mode}
            onClick={() => setView(t.mode)}
            className={`px-3 py-1 rounded text-sm transition-colors ${
              view === t.mode
                ? "bg-accent-blue text-white"
                : "bg-surface-tertiary text-text-secondary hover:text-text-primary"
            }`}
          >
            {t.label}
          </button>
        ))}
        <div className="flex-1" />
        <span className="text-xs text-text-muted">
          {graph ? `${graph.nodes.length} nodes, ${graph.edges.length} edges` : ""}
        </span>
        <button
          onClick={onRefresh}
          className="px-3 py-1 rounded text-sm bg-surface-tertiary text-text-secondary hover:text-text-primary"
        >
          Refresh
        </button>
      </div>

      <div className="flex-1 relative bg-bg-primary">
        {error && (
          <div className="absolute inset-0 flex items-center justify-center">
            <div className="error-banner">
              <span className="error-banner-message">{error}</span>
              <button onClick={onRefresh} className="ml-2 underline">Retry</button>
            </div>
          </div>
        )}
        {loading && !error && (
          <div className="absolute inset-0 flex items-center justify-center text-text-muted">
            Building topology...
          </div>
        )}
        {!loading && !error && graph.nodes.length === 0 && (
          <div className="absolute inset-0 flex items-center justify-center text-text-muted">
            No traffic data yet. Start the proxy to capture requests.
          </div>
        )}
        {!loading && !error && graph.nodes.length > 0 && (
          <>
            {view === "radial" && <RadialView graph={graph} onNodeClick={onNodeClick} />}
            {view === "layered" && <LayeredView graph={graph} onNodeClick={onNodeClick} />}
            {view === "grouped" && <GroupedView graph={graph} onNodeClick={onNodeClick} />}
          </>
        )}
      </div>
    </div>
  );
}
