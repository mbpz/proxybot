import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { WaterfallChart } from "./WaterfallChart";
import { DependencyGraph } from "./DependencyGraph";
import { AuthStateMachine } from "./AuthStateMachine";

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

  useEffect(() => {
    loadGraphData();
  }, []);

  async function loadGraphData() {
    try {
      setLoading(true);
      const result = await invoke<GraphData>("get_graph_data", { maxRequests: 100 });
      setData(result);
    } catch (err) {
      console.error("Failed to load graph data:", err);
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="p-6 h-screen flex flex-col">
      {/* View Selector */}
      <div className="flex gap-2 mb-4">
        <button
          onClick={() => setView("waterfall")}
          className={`px-4 py-2 rounded ${
            view === "waterfall" ? "bg-blue-600 text-white" : "bg-gray-200"
          }`}
        >
          Waterfall
        </button>
        <button
          onClick={() => setView("dag")}
          className={`px-4 py-2 rounded ${
            view === "dag" ? "bg-blue-600 text-white" : "bg-gray-200"
          }`}
        >
          Dependency
        </button>
        <button
          onClick={() => setView("auth")}
          className={`px-4 py-2 rounded ${
            view === "auth" ? "bg-blue-600 text-white" : "bg-gray-200"
          }`}
        >
          Auth Flow
        </button>
      </div>

      {/* Content */}
      <div className="flex-1 bg-white rounded-lg shadow overflow-hidden">
        {loading ? (
          <div className="flex items-center justify-center h-full">Loading...</div>
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