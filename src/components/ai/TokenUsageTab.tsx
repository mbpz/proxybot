// ============================================================
// Token Usage Tab
// ============================================================

import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "../ui/Button";
import { SkeletonTable } from "../ui/skeleton";
import { BarChart3 } from "lucide-react";
import type { AiStatRow, AiStatsData } from "./types";

export function TokenUsageTab() {
  const [data, setData] = useState<AiStatRow[]>([]);
  const [contextWindows, setContextWindows] = useState<Record<string, number>>({});
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    load();
  }, []);

  async function load() {
    try {
      setLoading(true);
      setError(null);
      const [statsResult, cwResult] = await Promise.allSettled([
        invoke<AiStatsData>("get_ai_stats"),
        invoke<Record<string, number>>("get_ai_context_windows"),
      ]);

      if (statsResult.status === "fulfilled" && statsResult.value?.stats) {
        setData(statsResult.value.stats);
      }
      if (cwResult.status === "fulfilled") {
        setContextWindows(cwResult.value);
      }

      const errors: string[] = [];
      if (statsResult.status === "rejected") errors.push("Stats: " + String(statsResult.reason));
      if (cwResult.status === "rejected") errors.push("Context: " + String(cwResult.reason));
      if (errors.length) setError(errors.join("; "));
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }

  function formatTokens(n: number): string {
    if (n >= 1_000_000) return (n / 1_000_000).toFixed(1) + "M";
    if (n >= 1_000) return (n / 1_000).toFixed(1) + "K";
    return String(n);
  }

  function formatCost(n: number): string {
    return "$" + n.toFixed(4);
  }

  return (
    <div>
      <div className="flex items-center justify-between mb-4">
        <span className="text-sm text-text-muted">
          {data.length} models tracked
        </span>
        <Button variant="secondary" size="sm" onClick={load}>Refresh</Button>
      </div>

      {error && (
        <div className="error-banner mb-4">
          <span className="error-banner-message">{error}</span>
        </div>
      )}

      {loading ? (
        <SkeletonTable rows={4} />
      ) : data.length === 0 ? (
        <div className="empty-state">
          <BarChart3 size={48} className="empty-state-icon" />
          <div className="empty-state-title">No LLM token data</div>
          <div className="empty-state-description">
            Token usage is tracked when AI providers (OpenAI, Anthropic, etc.)
            are detected in intercepted traffic.
          </div>
        </div>
      ) : (
        <table className="table">
          <thead>
            <tr>
              <th>Provider</th>
              <th>Model</th>
              <th>Total Tokens</th>
              <th>Prompt</th>
              <th>Completion</th>
              <th>Cost</th>
              <th>Requests</th>
              <th>Context Window</th>
            </tr>
          </thead>
          <tbody>
            {data.map((row) => (
              <tr key={`${row.provider}-${row.model}`}>
                <td className="text-sm font-medium">{row.provider}</td>
                <td className="mono text-xs">{row.model}</td>
                <td className="mono text-sm">{formatTokens(row.total_tokens)}</td>
                <td className="text-xs text-text-muted">{formatTokens(row.prompt_tokens)}</td>
                <td className="text-xs text-text-muted">{formatTokens(row.completion_tokens)}</td>
                <td className="mono text-sm" style={{ color: "var(--accent-green)" }}>
                  {formatCost(row.cost_usd)}
                </td>
                <td className="text-sm">{row.requests}</td>
                <td className="text-xs text-text-muted">
                  {contextWindows[row.model]
                    ? formatTokens(contextWindows[row.model])
                    : "—"}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}
