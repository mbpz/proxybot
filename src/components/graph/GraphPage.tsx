import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { WaterfallChart } from "./WaterfallChart";
import { DependencyGraph } from "./DependencyGraph";
import { AuthStateMachine } from "./AuthStateMachine";
import { GitBranch, AlertCircle } from "lucide-react";

type ViewType = "waterfall" | "dag" | "auth";

interface GraphData {
  requests: RequestNode[];
  edges: Edge[];
}

interface RequestNode {
  id: string;
  host: string;
  path: string;
  method: string;
  status?: number;
  duration_ms: number;
  timestamp: number;
  parentId?: string;
}

interface Edge {
  from: string;
  to: string;
}

export function GraphPage() {
  const [view, setView] = useState<ViewType>("waterfall");
  const [data, setData] = useState<GraphData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    loadGraphData();
  }, []);

  async function loadGraphData() {
    try {
      setLoading(true);
      setError(null);
      const result = await invoke<GraphData>("get_graph_data", { maxRequests: 100 });
      setData(result);
    } catch (err) {
      setError(`Failed to load graph data: ${err}`);
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="p-6 h-screen flex flex-col">
      <h1 className="text-2xl font-bold mb-4 flex items-center gap-2">
        <GitBranch size={24} className="text-accent-blue" />
        Graph
      </h1>

      {error && (
        <div className="error-banner mb-4">
          <AlertCircle size={16} />
          <span className="error-banner-message">{error}</span>
        </div>
      )}

      {/* View Selector */}
      <div className="flex gap-2 mb-4">
        {(["waterfall", "dag", "auth"] as ViewType[]).map((v) => (
          <button
            key={v}
            onClick={() => setView(v)}
            className={`px-4 py-2 rounded text-sm font-medium transition-colors ${
              view === v
                ? "bg-accent-blue text-white"
                : "bg-surface-tertiary text-text-secondary hover:text-text-primary"
            }`}
          >
            {v === "waterfall" ? "Waterfall" : v === "dag" ? "Dependency" : "Auth Flow"}
          </button>
        ))}
      </div>

      {/* Content */}
      <div className="flex-1 panel overflow-hidden">
        {loading ? (
          <div className="flex items-center justify-center h-full text-text-muted">
            <div className="skeleton w-48 h-8" />
          </div>
        ) : (
          <>
            {view === "waterfall" && <WaterfallChart data={data} />}
            {view === "dag" && <DependencyGraph data={data} />}
            {view === "auth" && <AuthStateMachine />}
          </>
        )}
      </div>
    </div>
  );
}
