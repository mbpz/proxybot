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
    <div className="mt-6 bg-white rounded-lg shadow overflow-hidden">
      <div className="px-4 py-3 border-b bg-gray-50">
        <h3 className="text-lg font-medium">
          Replay Results{" "}
          <span className="text-green-600">{successCount} passed</span>
          {failCount > 0 && (
            <span className="text-red-600 ml-2">{failCount} failed</span>
          )}
        </h3>
      </div>

      <table className="w-full">
        <thead>
          <tr className="text-sm text-gray-500">
            <th className="px-4 py-2 text-left">Target</th>
            <th className="px-4 py-2 text-left">Status</th>
            <th className="px-4 py-2 text-left">Duration</th>
            <th className="px-4 py-2 text-left">Result</th>
          </tr>
        </thead>
        <tbody>
          {results.map((result, i) => (
            <tr key={i} className="border-t">
              <td className="px-4 py-2 text-sm">{result.target_id}</td>
              <td className="px-4 py-2">
                <span
                  className={`px-2 py-1 rounded text-xs font-mono ${
                    result.status >= 200 && result.status < 300
                      ? "bg-green-100 text-green-800"
                      : "bg-red-100 text-red-800"
                  }`}
                >
                  {result.status}
                </span>
              </td>
              <td className="px-4 py-2 text-sm">{result.duration_ms}ms</td>
              <td className="px-4 py-2">
                {result.success ? (
                  <span className="text-green-600">Success</span>
                ) : (
                  <span className="text-red-600" title={result.error}>
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
