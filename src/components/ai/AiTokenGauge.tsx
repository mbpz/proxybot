import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface AiStats {
  provider: string;
  model: string;
  total_tokens: number;
  cost_usd: number;
  requests: number;
}

export function AiTokenGauge() {
  const [stats, setStats] = useState<AiStats[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    loadData();
    const interval = setInterval(loadData, 30000);
    return () => clearInterval(interval);
  }, []);

  async function loadData() {
    try {
      const [statsResult] = await Promise.all([
        invoke<{ stats: AiStats[] }>("get_ai_stats"),
      ]);
      setStats(statsResult.stats || []);
    } catch (e) {
      console.error("Failed to load AI stats:", e);
    } finally {
      setLoading(false);
    }
  }

  const totalTokens = stats.reduce((sum, s) => sum + Number(s.total_tokens), 0);
  const totalCost = stats.reduce((sum, s) => sum + Number(s.cost_usd), 0);
  const totalRequests = stats.reduce((sum, s) => sum + Number(s.requests), 0);

  if (loading) {
    return <div className="text-sm text-muted">Loading AI stats...</div>;
  }

  if (stats.length === 0) {
    return (
      <div className="empty-state">
        <div className="empty-state-icon">🤖</div>
        <div className="empty-state-title">No AI traffic detected</div>
        <div className="empty-state-description">
          AI API calls (OpenAI, Anthropic, etc.) will appear here
        </div>
      </div>
    );
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-4)" }}>
      {/* Summary cards */}
      <div style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: "var(--space-3)" }}>
        <div className="panel">
          <div className="panel-header">
            <span className="panel-title">Total Tokens</span>
          </div>
          <div className="panel-body" style={{ textAlign: "center" }}>
            <div style={{ fontSize: "1.5rem", fontWeight: 600 }}>
              {totalTokens.toLocaleString()}
            </div>
            <div className="text-xs text-muted">across {totalRequests} requests</div>
          </div>
        </div>
        <div className="panel">
          <div className="panel-header">
            <span className="panel-title">Estimated Cost</span>
          </div>
          <div className="panel-body" style={{ textAlign: "center" }}>
            <div style={{ fontSize: "1.5rem", fontWeight: 600 }}>
              ${totalCost.toFixed(4)}
            </div>
            <div className="text-xs text-muted">USD (approx)</div>
          </div>
        </div>
        <div className="panel">
          <div className="panel-header">
            <span className="panel-title">Models Active</span>
          </div>
          <div className="panel-body" style={{ textAlign: "center" }}>
            <div style={{ fontSize: "1.5rem", fontWeight: 600 }}>
              {stats.length}
            </div>
            <div className="text-xs text-muted">providers tracked</div>
          </div>
        </div>
      </div>
    </div>
  );
}