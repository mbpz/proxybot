interface HeadersViewProps {
  headers: Record<string, string>;
}

export function HeadersView({ headers }: HeadersViewProps) {
  const headerEntries = Object.entries(headers);

  const copyHeaders = () => {
    const text = headerEntries.map(([k, v]) => `${k}: ${v}`).join("\n");
    navigator.clipboard.writeText(text);
  };

  if (headerEntries.length === 0) {
    return (
      <div className="empty-state">
        <div className="empty-state-icon">📋</div>
        <div className="empty-state-title">No headers</div>
      </div>
    );
  }

  return (
    <div>
      <div
        className="flex items-center justify-between px-4 py-2"
        style={{
          background: "var(--bg-tertiary)",
          borderBottom: "1px solid var(--border)",
        }}
      >
        <span className="text-xs text-secondary font-mono uppercase">
          {headerEntries.length} headers
        </span>
        <button onClick={copyHeaders} className="btn btn-ghost btn-sm">
          Copy
        </button>
      </div>

      <table className="table">
        <tbody>
          {headerEntries.map(([key, value]) => (
            <tr key={key}>
              <td
                className="font-mono"
                style={{
                  color: "var(--accent-blue)",
                  width: "30%",
                  padding: "var(--space-2) var(--space-3)",
                  borderBottom: "1px solid var(--border)",
                }}
              >
                {key}
              </td>
              <td
                className="font-mono"
                style={{
                  color: "var(--text-primary)",
                  padding: "var(--space-2) var(--space-3)",
                  borderBottom: "1px solid var(--border)",
                  wordBreak: "break-all",
                }}
              >
                {value}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
