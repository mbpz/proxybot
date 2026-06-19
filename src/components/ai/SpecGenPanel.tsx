import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "../ui/Button";
import { Badge } from "../ui/Badge";
import type { SpecResult, TrafficRecord, ReplayReport } from "./types";

interface Props {
  sessionId: string;
  trafficRecords?: TrafficRecord[];
  onError: (msg: string) => void;
}

export function SpecGenPanel({ sessionId, trafficRecords, onError }: Props) {
  const [result, setResult] = useState<SpecResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [replay, setReplay] = useState<ReplayReport | null>(null);
  const [replayLoading, setReplayLoading] = useState(false);
  const [records, setRecords] = useState<TrafficRecord[]>(trafficRecords ?? []);
  const [recordsLoading, setRecordsLoading] = useState(false);

  // Load captured traffic for this session from the Rust side. We
  // skip the fetch when the parent passed records in via props, so
  // tests / fixtures can drive the component directly.
  //
  // We also push the sessionId down into the Rust `AppState` via
  // `set_active_session` so the proxy capture pipeline starts
  // tagging newly-recorded `http_requests` rows with this id.
  // Without this call, `get_traffic_records(sessionId)` would
  // always return zero rows for any non-empty sessionId because
  // the column would stay NULL on every insert.
  useEffect(() => {
    if (!sessionId) {
      // Clearing the field unbinds the proxy too, so subsequent
      // captures land with NULL session_id.
      invoke("set_active_session", { sessionId: null }).catch((err) =>
        onError(`set_active_session failed: ${err}`)
      );
      return;
    }
    // Fire-and-forget: we don't block the records load on this.
    invoke("set_active_session", { sessionId }).catch((err) =>
      onError(`set_active_session failed: ${err}`)
    );

    if (trafficRecords !== undefined) {
      setRecords(trafficRecords);
      return;
    }
    let cancelled = false;
    (async () => {
      try {
        setRecordsLoading(true);
        const recs = await invoke<TrafficRecord[]>("get_traffic_records", {
          sessionId,
        });
        if (!cancelled) setRecords(recs);
      } catch (err) {
        if (!cancelled) onError(String(err));
      } finally {
        if (!cancelled) setRecordsLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [sessionId, trafficRecords, onError]);

  async function generate() {
    if (!sessionId) {
      onError("Session ID is required");
      return;
    }
    try {
      setLoading(true);
      setResult(null);
      setReplay(null);
      // We pass `null` for trafficRecords so the Rust side reads
      // them straight out of SQLite. This avoids the round-trip
      // serialization cost of (DB → Rust → JSON → JS → JSON →
      // Rust). The `records` state is still kept for the count
      // badge in the header.
      const r = await invoke<SpecResult>("generate_spec", {
        sessionId,
        trafficRecords: null,
      });
      setResult(r);
      setReplay(r.replay);
    } catch (err) {
      onError(String(err));
    } finally {
      setLoading(false);
    }
  }

  async function copyYaml() {
    if (!result?.openapi?.OpenApi) return;
    try {
      await navigator.clipboard.writeText(result.openapi.OpenApi);
    } catch (err) {
      onError(`Copy failed: ${err}`);
    }
  }

  async function download() {
    if (!sessionId) return;
    try {
      // Rust hands back the concatenated YAML; we trigger the
      // save through a hidden <a download> so the user picks the
      // location via the OS save dialog. This avoids the prior
      // behaviour of Rust writing to the process cwd, which
      // landed files in an unpredictable spot.
      const yaml = await invoke<string>("export_spec", { sessionId });
      const blob = new Blob([yaml], { type: "application/yaml" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `${sessionId || "session"}-spec.yaml`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      // Free the object URL on the next tick so the browser has
      // a chance to start the download before we revoke it.
      setTimeout(() => URL.revokeObjectURL(url), 0);
    } catch (err) {
      onError(String(err));
    }
  }

  async function runReplay() {
    if (!sessionId) return;
    try {
      setReplayLoading(true);
      // Same record-source contract as `generate`: let Rust load
      // straight from SQLite.
      const r = await invoke<ReplayReport>("run_replay_validation", {
        sessionId,
        trafficRecords: null,
      });
      setReplay(r);
    } catch (err) {
      onError(String(err));
    } finally {
      setReplayLoading(false);
    }
  }

  const openapiYaml = result?.openapi?.OpenApi ?? "";
  const paths = parsePaths(openapiYaml);
  const sourceBadge = result
    ? { Llm: "default", Heuristic: "secondary", Hybrid: "secondary" }[result.source]
    : "secondary";

  return (
    <div className="rounded-lg border border-slate-200 dark:border-slate-800 p-4 mt-4">
      <div className="flex items-center gap-3 mb-3">
        <h3 className="text-base font-semibold">OpenAPI / AsyncAPI 生成</h3>
        {result && <Badge variant={sourceBadge as any}>{result.source}</Badge>}
        <span className="text-xs text-text-muted">
          {recordsLoading
            ? "正在加载流量记录..."
            : `流量记录: ${records.length}`}
        </span>
      </div>
      <div className="flex gap-2 mb-4">
        <Button onClick={generate} disabled={loading}>
          {loading ? "生成中..." : "▶ 生成规范"}
        </Button>
        <Button variant="secondary" onClick={copyYaml} disabled={!openapiYaml}>
          复制 YAML
        </Button>
        <Button variant="secondary" onClick={download} disabled={!result}>
          下载文件
        </Button>
        <Button variant="secondary" onClick={runReplay} disabled={!result || replayLoading}>
          {replayLoading ? "验证中..." : "▶ 重放验证"}
        </Button>
      </div>

      {result?.degradation_reason && (
        <div
          className="mb-3 px-3 py-2 rounded text-xs border border-yellow-300 bg-yellow-50 text-yellow-900 dark:border-yellow-700 dark:bg-yellow-950/40 dark:text-yellow-200"
          role="status"
          aria-live="polite"
        >
          <span className="mr-1.5" aria-hidden="true">⚠</span>
          {result.degradation_reason}
        </div>
      )}

      {result && (
        <div className="grid grid-cols-[240px_1fr] gap-3">
          <div className="border-r border-slate-200 dark:border-slate-800 pr-3">
            <div className="text-xs font-semibold text-slate-500 mb-2">Paths ({paths.length})</div>
            <ul className="space-y-1 text-sm">
              {paths.map((p) => (
                <li
                  key={p}
                  className={`cursor-pointer px-2 py-1 rounded ${
                    selectedPath === p ? "bg-slate-200 dark:bg-slate-700" : "hover:bg-slate-100 dark:hover:bg-slate-800"
                  }`}
                  onClick={() => setSelectedPath(p)}
                >
                  {p}
                </li>
              ))}
            </ul>
          </div>
          <div>
            {selectedPath ? (
              <pre className="text-xs bg-slate-50 dark:bg-slate-900 p-3 rounded overflow-x-auto">
                {extractPathDetail(openapiYaml, selectedPath)}
              </pre>
            ) : (
              <div className="text-sm text-slate-500">选择左侧路径查看详情</div>
            )}
            {replay && (
              <div className="mt-4 p-3 rounded bg-slate-50 dark:bg-slate-900">
                <div className="text-2xl font-bold">
                  {Math.round(replay.pass_rate * 100)}%
                </div>
                <div className="text-xs text-slate-500">
                  ✓ {replay.pass} / ✗ {replay.fail} / ⚠ {replay.error} (共 {replay.total})
                </div>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

function parsePaths(yaml: string): string[] {
  const lines = yaml.split("\n");
  const paths: string[] = [];
  for (const line of lines) {
    const m = line.match(/^  (\/[^:]+):/);
    if (m) paths.push(m[1]);
  }
  return paths;
}

function extractPathDetail(yaml: string, path: string): string {
  const lines = yaml.split("\n");
  const start = lines.findIndex((l) => l.startsWith(`  ${path}:`));
  if (start < 0) return "";
  const end = lines.findIndex((l, i) => i > start && /^  \/[^:]+:/.test(l));
  return lines.slice(start, end < 0 ? lines.length : end).join("\n");
}
