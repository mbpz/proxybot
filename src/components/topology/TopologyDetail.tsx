import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { safeInvoke } from "../../utils/safeInvoke";
import { NodeDetail, TopologyFilter } from "./types";
import { X } from "lucide-react";

interface Props {
  nodeId: string | null;
  filter: TopologyFilter;
  onClose: () => void;
}

export function TopologyDetail({ nodeId, filter, onClose }: Props) {
  const [detail, setDetail] = useState<NodeDetail | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const navigate = useNavigate();

  useEffect(() => {
    if (!nodeId) {
      setDetail(null);
      return;
    }
    setLoading(true);
    setError(null);
    safeInvoke<NodeDetail>("get_topology_node_detail", { nodeId, filter })
      .then((d) => setDetail(d))
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false));
  }, [nodeId, JSON.stringify(filter)]);

  if (!nodeId) return null;

  function jumpToTraffic() {
    if (!nodeId) return;
    const [kind, key] = nodeId.split(":");
    const params = new URLSearchParams();
    if (kind === "host") params.set("host", key);
    if (kind === "app") params.set("app", key);
    if (kind === "device") params.set("device", key);
    navigate(`/?${params.toString()}`);
  }

  return (
    <div className="w-96 border-l border-border bg-surface-primary overflow-y-auto flex flex-col">
      <div className="flex items-center justify-between px-4 py-2 border-b border-border">
        <span className="font-mono text-sm text-accent-blue">{nodeId}</span>
        <button onClick={onClose} className="text-text-muted hover:text-text-primary">
          <X size={16} />
        </button>
      </div>

      {loading && <div className="p-4 text-text-muted text-sm">Loading...</div>}
      {error && (
        <div className="error-banner m-4">
          <span className="error-banner-message">{error}</span>
        </div>
      )}
      {detail && (
        <div className="p-4 space-y-4 text-sm">
          <div>
            <h3 className="text-text-muted text-xs uppercase mb-1">Metrics</h3>
            <div className="grid grid-cols-2 gap-2">
              <div>Requests: <span className="text-accent-blue">{detail.node.request_count}</span></div>
              <div>Errors: <span className="text-accent-red">{detail.node.error_count}</span></div>
              <div>Error rate: <span className="text-accent-red">{(detail.node.error_rate * 100).toFixed(1)}%</span></div>
              <div>Avg latency: {detail.node.avg_latency_ms.toFixed(0)}ms</div>
            </div>
          </div>

          <div>
            <h3 className="text-text-muted text-xs uppercase mb-1">Status breakdown</h3>
            <div className="flex gap-2">
              {detail.status_breakdown.map((s) => (
                <div key={s.status_class} className="text-xs">
                  <span className="text-text-muted">{s.status_class}:</span>{" "}
                  <span className="text-text-primary">{s.count}</span>
                </div>
              ))}
            </div>
          </div>

          <div>
            <h3 className="text-text-muted text-xs uppercase mb-1">Recent requests</h3>
            <div className="space-y-1 max-h-64 overflow-y-auto">
              {detail.recent_requests.map((r) => (
                <div key={r.id} className="text-xs font-mono border-b border-border pb-1">
                  <span className="text-accent-blue">{r.method}</span> {r.host}{r.path}
                  <span className="ml-2 text-text-muted">
                    {r.status ?? "—"} {r.duration_ms}ms
                  </span>
                </div>
              ))}
            </div>
          </div>

          <button
            onClick={jumpToTraffic}
            className="w-full px-3 py-2 rounded bg-accent-blue text-white text-sm"
          >
            View in Traffic
          </button>
        </div>
      )}
    </div>
  );
}
