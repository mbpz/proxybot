import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface AiStats {
  provider: string;
  model: string;
  total_tokens: number;
  cost_usd: number;
  requests: number;
}

export function AiUsageTable() {
  const [stats, setStats] = useState<AiStats[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    loadData();
    const interval = setInterval(loadData, 30000);
    return () => clearInterval(interval);
  }, []);

  async function loadData() {
    try {
      const result = await invoke<{ stats: AiStats[] }>("get_ai_stats");
      setStats(result.stats || []);
    } catch (e) {
      console.error("Failed to load AI stats:", e);
    } finally {
      setLoading(false);
    }
  }

  if (loading) {
    return <div className="text-sm text-muted">Loading...</div>;
  }

  if (stats.length === 0) {
    return (
      <div className="text-sm text-muted">
        No AI usage data yet. Make AI API calls to see stats.
      </div>
    );
  }

  return (
    <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "0.85rem" }}>
      <thead>
        <tr style={{ borderBottom: "1px solid var(--border)" }}>
          <th style={{ textAlign: "left", padding: "var(--space-2)" }}>Provider</th>
          <th style={{ textAlign: "left", padding: "var(--space-2)" }}>Model</th>
          <th style={{ textAlign: "right", padding: "var(--space-2)" }}>Tokens</th>
          <th style={{ textAlign: "right", padding: "var(--space-2)" }}>Cost</th>
          <th style={{ textAlign: "right", padding: "var(--space-2)" }}>Requests</th>
        </tr>
      </thead>
      <tbody>
        {stats.map((s, i) => (
          <tr key={i} style={{ borderBottom: "1px solid var(--border)" }}>
            <td style={{ padding: "var(--space-2)" }}>
              <span className="badge badge-info">{s.provider}</span>
            </td>
            <td style={{ padding: "var(--space-2)", fontFamily: "monospace", fontSize: "0.8rem" }}>
              {s.model}
            </td>
            <td style={{ padding: "var(--space-2)", textAlign: "right" }}>
              {Number(s.total_tokens).toLocaleString()}
            </td>
            <td style={{ padding: "var(--space-2)", textAlign: "right" }}>
              ${Number(s.cost_usd).toFixed(6)}
            </td>
            <td style={{ padding: "var(--space-2)", textAlign: "right" }}>
              {Number(s.requests).toLocaleString()}
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}