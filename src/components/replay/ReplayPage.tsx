import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ReplayModal } from "./ReplayModal";
import { ReplayResults } from "./ReplayResults";

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
  const [selectedTarget, setSelectedTarget] = useState<ReplayTarget | null>(null);
  const [results, setResults] = useState<ReplayResult[]>([]);
  const [isRunning, setIsRunning] = useState(false);

  useEffect(() => {
    loadTargets();
  }, []);

  async function loadTargets() {
    try {
      const result = await invoke<ReplayTarget[]>("get_replay_targets");
      setTargets(result);
    } catch (err) {
      console.error("Failed to load replay targets:", err);
    }
  }

  async function handleStartReplay() {
    setIsRunning(true);
    setResults([]);
    try {
      const result = await invoke<ReplayResult[]>("execute_replay", {
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
      await invoke("delete_replay_target", { id });
      loadTargets();
    } catch (err) {
      console.error("Failed to delete target:", err);
    }
  }

  return (
    <div className="p-6">
      <div className="flex justify-between items-center mb-6">
        <h1 className="text-2xl font-bold">Replay</h1>
        <div className="flex gap-2">
          <button
            onClick={() => setSelectedTarget(null)}
            className="px-4 py-2 bg-gray-200 rounded hover:bg-gray-300"
          >
            New Target
          </button>
          <button
            onClick={handleStartReplay}
            disabled={isRunning || targets.length === 0}
            className="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 disabled:opacity-50"
          >
            {isRunning ? "Running..." : "Start Replay"}
          </button>
        </div>
      </div>

      {/* Targets Table */}
      <div className="bg-white rounded-lg shadow overflow-hidden">
        <table className="w-full">
          <thead className="bg-gray-50">
            <tr>
              <th className="px-4 py-3 text-left text-sm font-medium text-gray-500">Enabled</th>
              <th className="px-4 py-3 text-left text-sm font-medium text-gray-500">Name</th>
              <th className="px-4 py-3 text-left text-sm font-medium text-gray-500">Method</th>
              <th className="px-4 py-3 text-left text-sm font-medium text-gray-500">URL</th>
              <th className="px-4 py-3 text-left text-sm font-medium text-gray-500">Expected</th>
              <th className="px-4 py-3 text-right text-sm font-medium text-gray-500">Actions</th>
            </tr>
          </thead>
          <tbody>
            {targets.map((target) => (
              <tr key={target.id} className="border-t">
                <td className="px-4 py-3">
                  <input
                    type="checkbox"
                    checked={target.enabled}
                    onChange={async () => {
                      await invoke("toggle_replay_target", { id: target.id, enabled: !target.enabled });
                      loadTargets();
                    }}
                    className="w-4 h-4"
                  />
                </td>
                <td className="px-4 py-3 text-sm">{target.name}</td>
                <td className="px-4 py-3">
                  <span className="px-2 py-1 bg-gray-100 rounded text-xs font-mono">
                    {target.method}
                  </span>
                </td>
                <td className="px-4 py-3 text-sm truncate max-w-xs">{target.url}</td>
                <td className="px-4 py-3 text-sm">{target.expected_status || "-"}</td>
                <td className="px-4 py-3 text-right">
                  <button
                    onClick={() => setSelectedTarget(target)}
                    className="px-2 py-1 text-sm text-blue-600 hover:bg-blue-50 rounded"
                  >
                    Edit
                  </button>
                  <button
                    onClick={() => handleDeleteTarget(target.id)}
                    className="px-2 py-1 text-sm text-red-600 hover:bg-red-50 rounded ml-2"
                  >
                    Delete
                  </button>
                </td>
              </tr>
            ))}
            {targets.length === 0 && (
              <tr>
                <td colSpan={6} className="px-4 py-8 text-center text-gray-500">
                  No replay targets. Click "New Target" to create one.
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>

      {/* Results */}
      {results.length > 0 && <ReplayResults results={results} />}

      {/* Modal */}
      {selectedTarget !== null && (
        <ReplayModal
          target={selectedTarget}
          onSave={async (updated) => {
            await invoke("save_replay_target", { target: updated });
            loadTargets();
            setSelectedTarget(null);
          }}
          onClose={() => setSelectedTarget(null)}
        />
      )}
    </div>
  );
}
