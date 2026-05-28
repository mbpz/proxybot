import { useState, useEffect } from "react";
import { ReplayModal } from "./ReplayModal";
import { ReplayResults } from "./ReplayResults";
import { MethodBadge } from "../ui/Badge";
import { Button } from "../ui/Button";
import { ErrorBoundary } from "../ui/error-boundary";
import { SkeletonTable } from "../ui/skeleton";
import { safeInvokeOr } from "../../utils/safeInvoke";

interface ReplayTarget {
  id: string;
  name: string;
  method: string;
  url: string;
  headers: Record<string, string>;
  body?: string;
  expected_status?: number;
  enabled: boolean;
}

interface ReplayResult {
  target_id: string;
  status: number;
  duration_ms: number;
  success: boolean;
  error?: string;
}

export function ReplayPage() {
  const [targets, setTargets] = useState<ReplayTarget[]>([]);
  const [selectedTarget, setSelectedTarget] = useState<ReplayTarget | null>(
    null
  );
  const [results, setResults] = useState<ReplayResult[]>([]);
  const [isRunning, setIsRunning] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    loadTargets();
  }, []);

  async function loadTargets() {
    try {
      setLoading(true);
      setError(null);
      const result = await safeInvokeOr<ReplayTarget[]>("get_replay_targets", []);
      setTargets(result);
    } catch (err) {
      console.error("Failed to load replay targets:", err);
    } finally {
      setLoading(false);
    }
  }

  async function handleStartReplay() {
    setIsRunning(true);
    setResults([]);
    try {
      const result = await safeInvokeOr<ReplayResult[]>("execute_replay", [], {
        targets: targets.filter((t) => t.enabled),
      });
      setResults(result);
    } catch (err) {
      console.error("Replay failed:", err);
    } finally {
      setIsRunning(false);
    }
  }

  async function handleDeleteTarget(id: string) {
    try {
      await safeInvokeOr("delete_replay_target", null, { id });
      loadTargets();
    } catch (err) {
      console.error("Failed to delete target:", err);
    }
  }

  return (
    <div>
      <div className="panel">
        {/* Header */}
        <div className="panel-header">
          <div className="flex items-center gap-3">
            <span className="panel-title">Replay</span>
            <span className="text-sm text-muted">
              {targets.length} targets
            </span>
          </div>
          <div className="flex gap-2">
            <Button
              variant="secondary"
              size="sm"
              onClick={() => setSelectedTarget(null)}
            >
              New Target
            </Button>
            <Button
              variant="primary"
              size="sm"
              onClick={handleStartReplay}
              disabled={isRunning || targets.length === 0}
            >
              {isRunning ? "Running..." : "Start Replay"}
            </Button>
          </div>
        </div>

        {/* Error banner */}
        {error && (
          <div className="error-banner mx-4 mt-2">
            <span className="error-banner-message">{error}</span>
            <Button variant="secondary" size="sm" onClick={loadTargets}>
              Retry
            </Button>
          </div>
        )}

        {/* Content */}
        <div style={{ maxHeight: 500, overflowY: "auto" }}>
          <ErrorBoundary>
            {loading ? (
              <SkeletonTable rows={5} />
            ) : targets.length === 0 ? (
              <div className="empty-state">
                <div className="empty-state-icon">🔄</div>
                <div className="empty-state-title">No replay targets</div>
                <div className="empty-state-description">
                  Click "New Target" to create a replay target.
                </div>
              </div>
            ) : (
              <table className="table">
                <thead>
                  <tr>
                    <th style={{ width: 50 }}>On</th>
                    <th>Name</th>
                    <th style={{ width: 80 }}>Method</th>
                    <th>URL</th>
                    <th style={{ width: 80 }}>Expected</th>
                    <th style={{ width: 100 }}>Actions</th>
                  </tr>
                </thead>
                <tbody>
                  {targets.map((target) => (
                    <tr key={target.id}>
                      <td>
                        <input
                          type="checkbox"
                          checked={target.enabled}
                          onChange={async () => {
                            await safeInvokeOr("toggle_replay_target", null, {
                              id: target.id,
                              enabled: !target.enabled,
                            });
                            loadTargets();
                          }}
                        />
                      </td>
                      <td className="text-sm">{target.name}</td>
                      <td>
                        <MethodBadge method={target.method} />
                      </td>
                      <td
                        className="mono text-xs truncate"
                        style={{ color: "var(--text-muted)", maxWidth: 300 }}
                      >
                        {target.url}
                      </td>
                      <td className="text-sm">
                        {target.expected_status || "-"}
                      </td>
                      <td>
                        <div className="flex gap-1">
                          <button
                            className="btn btn-sm btn-ghost"
                            onClick={() => setSelectedTarget(target)}
                          >
                            Edit
                          </button>
                          <button
                            className="btn btn-sm btn-ghost"
                            onClick={() => handleDeleteTarget(target.id)}
                          >
                            Delete
                          </button>
                        </div>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </ErrorBoundary>
        </div>
      </div>

      {/* Results */}
      {results.length > 0 && <ReplayResults results={results} />}

      {/* Modal */}
      {selectedTarget !== null && (
        <ReplayModal
          target={selectedTarget}
          onSave={async (updated) => {
            await safeInvokeOr("save_replay_target", null, { target: updated });
            loadTargets();
            setSelectedTarget(null);
          }}
          onClose={() => setSelectedTarget(null)}
        />
      )}
    </div>
  );
}
