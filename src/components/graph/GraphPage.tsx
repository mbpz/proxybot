import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { WaterfallChart } from "./WaterfallChart";
import { DependencyGraph } from "./DependencyGraph";
import { AuthStateMachine } from "./AuthStateMachine";
import { GitBranch, RefreshCw } from "lucide-react";
import { Button } from "../ui/Button";
import { ErrorState, InlineNotice, LoadingState } from "../ui/states";
import { GraphData } from "./types";

type ViewType = "waterfall" | "dag" | "auth";

export function GraphPage() {
  const [view, setView] = useState<ViewType>("waterfall");
  const [data, setData] = useState<GraphData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [deviceId, setDeviceId] = useState("");

  useEffect(() => {
    void loadGraphData();
  }, []);

  async function loadGraphData() {
    try {
      setLoading(true);
      setError(null);
      const result = await invoke<GraphData | null>("get_graph_data", { max_requests: 100 });
      setData(result ?? null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }

  async function loadDeviceDag() {
    const parsed = Number.parseInt(deviceId, 10);
    if (!Number.isFinite(parsed)) {
      setError("Device ID must be a number.");
      return;
    }
    try {
      setLoading(true);
      setError(null);
      const result = await invoke<GraphData | null>("get_device_dag", { device_id: parsed });
      setData(result ?? null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }

  async function buildDag() {
    try {
      setLoading(true);
      setError(null);
      setNotice(null);
      await invoke("build_traffic_dag");
      const result = await invoke<GraphData | null>("get_traffic_dag");
      setData(result ?? null);
    } catch (err) {
      // Fall back to the stored waterfall data, but tell the user the DAG build failed.
      try {
        const result = await invoke<GraphData | null>("get_graph_data", { max_requests: 100 });
        setData(result ?? null);
        setNotice(
          `Could not build the traffic DAG (${String(err)}). Showing stored request data instead.`,
        );
      } catch {
        setError(`DAG build failed: ${String(err)}`);
      }
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="flex h-full min-h-0 flex-col">
      <h1 className="mb-4 flex items-center gap-2 text-2xl font-bold">
        <GitBranch size={24} className="text-accent-blue" />
        Graph
      </h1>

      {error && (
        <div className="mb-4">
          <ErrorState
            title="Graph data unavailable"
            message={error}
            onRetry={() => void loadGraphData()}
          />
        </div>
      )}
      {notice && (
        <div className="mb-4">
          <InlineNotice variant="info" onDismiss={() => setNotice(null)}>
            {notice}
          </InlineNotice>
        </div>
      )}

      {/* View Selector */}
      <div className="mb-4 flex flex-wrap items-center gap-2">
        {(["waterfall", "dag", "auth"] as ViewType[]).map((value) => (
          <button
            key={value}
            type="button"
            aria-pressed={view === value}
            onClick={() => setView(value)}
            className={`px-4 py-2 rounded text-sm font-medium transition-colors ${
              view === value
                ? "bg-accent-blue text-white"
                : "bg-surface-tertiary text-text-secondary hover:text-text-primary"
            }`}
          >
            {value === "waterfall" ? "Waterfall" : value === "dag" ? "Dependency" : "Auth Flow"}
          </button>
        ))}
        <div className="flex-1" />
        <label className="sr-only" htmlFor="graph-device-id">
          Device ID
        </label>
        <input
          id="graph-device-id"
          type="text"
          inputMode="numeric"
          value={deviceId}
          onChange={(event) => setDeviceId(event.target.value)}
          placeholder="Device ID"
          style={{ width: 100 }}
        />
        <Button
          variant="secondary"
          size="sm"
          onClick={() => void loadDeviceDag()}
          disabled={loading || !deviceId}
        >
          Device DAG
        </Button>
        <Button variant="secondary" size="sm" onClick={() => void buildDag()} disabled={loading}>
          <RefreshCw size={14} />
          Build DAG
        </Button>
      </div>

      {/* Content */}
      <div className="flex-1 panel overflow-hidden">
        {loading ? (
          <LoadingState label="Building graph..." />
        ) : (
          <>
            {view === "waterfall" && <WaterfallChart data={data} />}
            {view === "dag" && <DependencyGraph data={data} />}
            {view === "auth" && <AuthStateMachine data={data} />}
          </>
        )}
      </div>
    </div>
  );
}
