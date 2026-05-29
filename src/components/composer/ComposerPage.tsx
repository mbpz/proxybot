import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ComposerEditor } from "./ComposerEditor";
import { Send, AlertCircle } from "lucide-react";

interface ComposerResponse {
  status: number;
  headers: Record<string, string>;
  body: string;
  duration_ms: number;
}

function getStatusColor(status: number): string {
  if (status >= 200 && status < 300) return "text-accent-green";
  if (status >= 300 && status < 400) return "text-accent-blue";
  if (status >= 400 && status < 500) return "text-accent-yellow";
  return "text-accent-red";
}

export function ComposerPage() {
  const [method, setMethod] = useState("GET");
  const [url, setUrl] = useState("");
  const [headers, setHeaders] = useState<Record<string, string>>({
    "Content-Type": "application/json",
  });
  const [body, setBody] = useState("");
  const [isSending, setIsSending] = useState(false);
  const [response, setResponse] = useState<ComposerResponse | null>(null);
  const [error, setError] = useState<string | null>(null);

  const handleSend = useCallback(async () => {
    setIsSending(true);
    setError(null);
    setResponse(null);
    try {
      const result = await invoke<ComposerResponse>("compose_request", {
        method,
        url,
        headers,
        body,
      });
      setResponse(result);
    } catch (err) {
      setError(String(err));
    } finally {
      setIsSending(false);
    }
  }, [method, url, headers, body]);

  return (
    <div className="flex h-full">
      {/* Editor (left 40%) */}
      <div className="w-2/5 border-r border-border bg-surface-secondary">
        <div className="p-4 border-b border-border">
          <h1 className="text-xl font-bold flex items-center gap-2">
            <Send size={20} className="text-accent-blue" />
            Composer
          </h1>
          <p className="text-sm text-text-muted">Edit and send HTTP requests</p>
        </div>
        <ComposerEditor
          method={method}
          url={url}
          headers={headers}
          body={body}
          onMethodChange={setMethod}
          onUrlChange={setUrl}
          onHeadersChange={setHeaders}
          onBodyChange={setBody}
          onSend={handleSend}
          isSending={isSending}
        />
      </div>

      {/* Response (right 60%) */}
      <div className="w-3/5 flex flex-col">
        <div className="p-4 border-b border-border bg-surface-tertiary">
          <h2 className="text-lg font-medium">Response</h2>
        </div>
        <div className="flex-1 overflow-auto p-4">
          {error && (
            <div className="error-banner mb-4">
              <AlertCircle size={16} />
              <span className="error-banner-message">{error}</span>
            </div>
          )}
          {response ? (
            <div className="space-y-4">
              <div className="flex gap-4 text-sm items-center">
                <span className={`font-mono font-bold ${getStatusColor(response.status)}`}>
                  {response.status}
                </span>
                <span className="text-text-muted">{response.duration_ms}ms</span>
              </div>
              <div>
                <h3 className="text-sm font-medium text-text-secondary mb-2">
                  Headers
                </h3>
                <pre className="text-xs font-mono bg-surface-tertiary p-2 rounded border border-border">
                  {Object.entries(response.headers)
                    .map(([k, v]) => `${k}: ${v}`)
                    .join("\n")}
                </pre>
              </div>
              <div>
                <h3 className="text-sm font-medium text-text-secondary mb-2">
                  Body
                </h3>
                <pre className="text-sm font-mono bg-surface-tertiary p-3 rounded border border-border whitespace-pre-wrap break-all max-h-96 overflow-auto">
                  {formatBody(response.body)}
                </pre>
              </div>
            </div>
          ) : (
            <div className="flex items-center justify-center h-full text-text-muted">
              Enter a URL and click Send to see the response
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

function formatBody(body: string): string {
  try {
    return JSON.stringify(JSON.parse(body), null, 2);
  } catch {
    return body;
  }
}
