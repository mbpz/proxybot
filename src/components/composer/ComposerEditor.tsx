import { useState } from "react";

interface ComposerEditorProps {
  method: string;
  url: string;
  headers: Record<string, string>;
  body: string;
  onMethodChange: (m: string) => void;
  onUrlChange: (u: string) => void;
  onHeadersChange: (h: Record<string, string>) => void;
  onBodyChange: (b: string) => void;
  onSend: () => void;
  isSending: boolean;
}

const METHODS = ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"];

export function ComposerEditor({
  method,
  url,
  headers,
  body,
  onMethodChange,
  onUrlChange,
  onHeadersChange,
  onBodyChange,
  onSend,
  isSending,
}: ComposerEditorProps) {
  const [headersText, setHeadersText] = useState(
    Object.entries(headers).map(([k, v]) => `${k}: ${v}`).join("\n"),
  );

  function handleHeadersChange(text: string) {
    setHeadersText(text);
    const parsed: Record<string, string> = {};
    text.split("\n").forEach((line) => {
      const idx = line.indexOf(":");
      if (idx > 0) {
        const k = line.slice(0, idx).trim();
        const v = line.slice(idx + 1).trim();
        if (k) parsed[k] = v;
      }
    });
    onHeadersChange(parsed);
  }

  return (
    <div className="flex flex-col h-full">
      {/* Method + URL bar */}
      <div className="flex gap-2 p-4 bg-surface-tertiary border-b border-border">
        <select
          value={method}
          onChange={(e) => onMethodChange(e.target.value)}
          className="px-3 py-2"
        >
          {METHODS.map((m) => (
            <option key={m} value={m}>
              {m}
            </option>
          ))}
        </select>
        <input
          type="text"
          value={url}
          onChange={(e) => onUrlChange(e.target.value)}
          placeholder="https://api.example.com/endpoint"
          className="flex-1 px-3 py-2 font-mono text-sm"
        />
        <button
          onClick={onSend}
          disabled={isSending || !url}
          className="btn btn-primary"
        >
          {isSending ? "Sending..." : "Send"}
        </button>
      </div>

      {/* Headers + Body */}
      <div className="flex-1 flex flex-col p-4 space-y-4 overflow-auto">
        <div>
          <label className="block text-sm font-medium text-text-secondary mb-1">
            Headers (Key: Value, one per line)
          </label>
          <textarea
            value={headersText}
            onChange={(e) => handleHeadersChange(e.target.value)}
            className="w-full h-32 px-3 py-2 font-mono text-sm"
            placeholder="Content-Type: application/json"
          />
        </div>
        <div className="flex-1">
          <label className="block text-sm font-medium text-text-secondary mb-1">
            Body
          </label>
          <textarea
            value={body}
            onChange={(e) => onBodyChange(e.target.value)}
            className="w-full h-64 px-3 py-2 font-mono text-sm"
            placeholder='{"key": "value"}'
          />
        </div>
      </div>
    </div>
  );
}
