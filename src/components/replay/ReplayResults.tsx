interface ReplayResult {
  target_id: string;
  status: number;
  duration_ms: number;
  success: boolean;
  error?: string;
}

interface ReplayResultsProps {
  results: ReplayResult[];
}

export function ReplayResults({ results }: ReplayResultsProps) {
  const successCount = results.filter((r) => r.success).length;
  const failCount = results.length - successCount;

  return (
    <div className="mt-6 panel overflow-hidden">
      <div className="panel-header">
        <h3 className="text-lg font-medium">
          Replay Results{" "}
          <span className="text-accent-green">{successCount} passed</span>
          {failCount > 0 && (
            <span className="text-accent-red ml-2">{failCount} failed</span>
          )}
        </h3>
      </div>

      <table className="table">
        <thead>
          <tr>
            <th>Target</th>
            <th>Status</th>
            <th>Duration</th>
            <th>Result</th>
          </tr>
        </thead>
        <tbody>
          {results.map((result, i) => (
            <tr key={i}>
              <td className="text-sm">{result.target_id}</td>
              <td>
                <span className={`badge ${result.status >= 200 && result.status < 300 ? "badge-get" : "badge-reject"}`}>
                  {result.status}
                </span>
              </td>
              <td className="text-sm">{result.duration_ms}ms</td>
              <td>
                {result.success ? (
                  <span className="text-accent-green">Success</span>
                ) : (
                  <span className="text-accent-red" title={result.error}>
                    {result.error?.slice(0, 50) || "Failed"}
                  </span>
                )}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
