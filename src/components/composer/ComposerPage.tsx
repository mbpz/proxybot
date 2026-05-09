import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ComposerEditor } from "./ComposerEditor";

interface ComposerResponse {
  status: number;
  headers: Record<string, string>;
  body: string;
  duration_ms: number;
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
      <div className="w-2/5 border-r bg-white">
        <div className="p-4 border-b">
          <h1 className="text-xl font-bold">Composer</h1>
          <p className="text-sm text-gray-500">Edit and send HTTP requests</p>
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
        <div className="p-4 border-b bg-gray-50">
          <h2 className="text-lg font-medium">Response</h2>
        </div>
        <div className="flex-1 overflow-auto p-4">
          {error && (
            <div className="p-4 bg-red-50 text-red-700 rounded">{error}</div>
          )}
          {response ? (
            <div className="space-y-4">
              <div className="flex gap-4 text-sm">
                <span className="px-2 py-1 rounded bg-green-100 text-green-800 font-mono">
                  {response.status}
                </span>
                <span className="text-gray-500">{response.duration_ms}ms</span>
              </div>
              <div>
                <h3 className="text-sm font-medium text-gray-700 mb-2">
                  Headers
                </h3>
                <pre className="text-xs font-mono bg-gray-50 p-2 rounded">
                  {Object.entries(response.headers)
                    .map(([k, v]) => `${k}: ${v}`)
                    .join("\n")}
                </pre>
              </div>
              <div>
                <h3 className="text-sm font-medium text-gray-700 mb-2">
                  Body
                </h3>
                <pre className="text-sm font-mono bg-gray-50 p-3 rounded whitespace-pre-wrap break-all max-h-96 overflow-auto">
                  {formatBody(response.body)}
                </pre>
              </div>
            </div>
          ) : (
            <div className="flex items-center justify-center h-full text-gray-400">
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
