import type { DnsEntry } from "../types";
import { appBadgeClass } from "../utils";

interface DnsPageProps {
  dnsQueries: DnsEntry[];
}

export function DnsPage({ dnsQueries }: DnsPageProps) {
  return (
    <div className="panel">
      <div className="panel-header">
        <span className="panel-title">DNS Queries</span>
        <span className="text-sm text-muted">{dnsQueries.length} entries</span>
      </div>
      <div style={{ maxHeight: 500, overflowY: "auto" }}>
        {dnsQueries.length === 0 ? (
          <div className="empty-state">
            <div className="empty-state-icon">🌐</div>
            <div className="empty-state-title">No DNS queries</div>
            <div className="empty-state-description">Enable transparent proxy to start capturing DNS queries</div>
          </div>
        ) : (
          <table className="table">
            <thead><tr><th>App</th><th>Time</th><th>Domain</th></tr></thead>
            <tbody>
              {dnsQueries.map((q, idx) => (
                <tr key={`${q.timestamp_ms}-${idx}`}>
                  <td>{q.app_name && <span className={`badge ${appBadgeClass(q.app_name)}`}>{q.app_name}</span>}</td>
                  <td className="mono text-sm">{new Date(q.timestamp_ms).toLocaleTimeString()}</td>
                  <td className="mono text-sm">{q.domain}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}
