import { CodeViewer } from "../shared/CodeViewer";

interface BodyViewProps {
  body?: string;
}

export function BodyView({ body }: BodyViewProps) {
  if (!body) {
    return (
      <div className="empty-state">
        <div className="empty-state-title">No body content</div>
      </div>
    );
  }

  const isJson = (() => {
    try {
      JSON.parse(body);
      return true;
    } catch {
      return false;
    }
  })();

  return (
    <div>
      <div className="flex items-center justify-between px-4 py-2 bg-surface-tertiary border-b border-border">
        <span className="text-xs text-secondary font-mono uppercase">
          {isJson ? "JSON" : "Text"} · {body.length} bytes
        </span>
      </div>
      <CodeViewer
        content={body}
        contentType={isJson ? "application/json" : "text/plain"}
        maxHeight="calc(100vh - 300px)"
      />
    </div>
  );
}
