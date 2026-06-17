import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "../ui/Button";
import { Badge } from "../ui/Badge";
import type { SpecResult, TrafficRecord, ReplayReport } from "./types";

interface Props {
  sessionId: string;
  trafficRecords: TrafficRecord[];
  onError: (msg: string) => void;
}

export function SpecGenPanel({ sessionId, trafficRecords, onError }: Props) {
  const [result, setResult] = useState<SpecResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [replay, setReplay] = useState<ReplayReport | null>(null);
  const [replayLoading, setReplayLoading] = useState(false);

  async function generate() {
    if (!sessionId) {
      onError("Session ID is required");
      return;
    }
    try {
      setLoading(true);
      setResult(null);
      setReplay(null);
      const r = await invoke<SpecResult>("generate_spec", {
        sessionId,
        trafficRecords,
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
    await navigator.clipboard.writeText(result.openapi.OpenApi);
  }

  async function download() {
    if (!sessionId) return;
    try {
      const target = `${sessionId}-openapi.yaml`;
      await invoke("export_spec", { sessionId, targetPath: target });
    } catch (err) {
      onError(String(err));
    }
  }

  async function runReplay() {
    if (!sessionId) return;
    try {
      setReplayLoading(true);
      const r = await invoke<ReplayReport>("run_replay_validation", {
        sessionId,
        trafficRecords,
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
