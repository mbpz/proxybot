interface BodyViewProps {
  body?: string;
}

export function BodyView({ body }: BodyViewProps) {
  if (!body) {
    return (
      <div className="empty-state">
        <div className="empty-state-icon">📄</div>
        <div className="empty-state-title">No body content</div>
      </div>
    );
  }

  let formattedBody = body;
  let isJson = false;
  try {
    const parsed = JSON.parse(body);
    formattedBody = JSON.stringify(parsed, null, 2);
    isJson = true;
  } catch {
    // Not JSON, use as-is
  }

  const copyBody = () => {
    navigator.clipboard.writeText(formattedBody);
  };

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
          {isJson ? "JSON" : "Text"} • {body.length} bytes
        </span>
        <button onClick={copyBody} className="btn btn-ghost btn-sm">
          Copy
        </button>
      </div>

      <pre
        className="p-4 text-sm font-mono overflow-auto whitespace-pre-wrap"
        style={{
          color: "var(--text-primary)",
          background: "var(--bg-secondary)",
          margin: 0,
        }}
      >
        {formattedBody}
      </pre>
    </div>
  );
}
