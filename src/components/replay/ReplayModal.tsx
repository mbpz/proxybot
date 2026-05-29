import { useState } from "react";
import { PlayCircle } from "lucide-react";

interface ReplayTarget {
  id: string;
  name: string;
  method: string;
  url: string;
  headers: Record<string, string>;
  body?: string;
  expected_status?: number;
  enabled: boolean;
}

interface ReplayModalProps {
  target: ReplayTarget | null;
  onSave: (target: ReplayTarget) => void;
  onClose: () => void;
}

const methods = ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"];

export function ReplayModal({ target, onSave, onClose }: ReplayModalProps) {
  const [form, setForm] = useState<ReplayTarget>(
    target || {
      id: crypto.randomUUID(),
      name: "",
      method: "GET",
      url: "https://",
      headers: {},
      body: undefined,
      expected_status: 200,
      enabled: true,
    }
  );

  const [headersText, setHeadersText] = useState(
    Object.entries(form.headers)
      .map(([k, v]) => `${k}: ${v}`)
      .join("\n")
  );

  async function handleReplay() {
    try {
      const response = await fetch(form.url, {
        method: form.method,
        headers: form.headers,
        body: form.method !== "GET" && form.method !== "HEAD" ? form.body : undefined,
      });
      const text = await response.text();
      alert(`Status: ${response.status}\n\nResponse:\n${text.slice(0, 500)}`);
    } catch (err) {
      alert(`Error: ${err}`);
    }
  }

  function handleSave() {
    const headers: Record<string, string> = {};
    headersText.split("\n").forEach((line) => {
      const colonIndex = line.indexOf(":");
      if (colonIndex > 0) {
        const key = line.slice(0, colonIndex).trim();
        const value = line.slice(colonIndex + 1).trim();
        if (key) headers[key] = value;
      }
    });
    onSave({ ...form, headers });
  }

  return (
    <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
      <div className="card w-full max-w-2xl max-h-screen overflow-auto p-6 shadow-lg">
        <h2 className="text-xl font-bold mb-4 flex items-center gap-2">
          <PlayCircle size={20} className="text-accent-blue" />
          {target ? "Edit Target" : "New Target"}
        </h2>

        <div className="space-y-4">
          {/* Name */}
          <div>
            <label className="block text-sm font-medium text-text-secondary mb-1">Name</label>
            <input
              type="text"
              value={form.name}
              onChange={(e) => setForm({ ...form, name: e.target.value })}
              className="w-full px-3 py-2"
              placeholder="My API Test"
            />
          </div>

          {/* Method + URL */}
          <div className="flex gap-2">
            <select
              value={form.method}
              onChange={(e) => setForm({ ...form, method: e.target.value })}
              className="w-24 px-3 py-2"
            >
              {methods.map((m) => (
                <option key={m} value={m}>{m}</option>
              ))}
            </select>
            <input
              type="text"
              value={form.url}
              onChange={(e) => setForm({ ...form, url: e.target.value })}
              className="flex-1 px-3 py-2"
              placeholder="https://api.example.com/endpoint"
            />
          </div>

          {/* Headers */}
          <div>
            <label className="block text-sm font-medium text-text-secondary mb-1">Headers (one per line: Key: Value)</label>
            <textarea
              value={headersText}
              onChange={(e) => setHeadersText(e.target.value)}
              className="w-full h-24 px-3 py-2 font-mono text-sm"
              placeholder="Content-Type: application/json\nAuthorization: Bearer token"
            />
          </div>

          {/* Body */}
          <div>
            <label className="block text-sm font-medium text-text-secondary mb-1">Body</label>
            <textarea
              value={form.body || ""}
              onChange={(e) => setForm({ ...form, body: e.target.value })}
              className="w-full h-32 px-3 py-2 font-mono text-sm"
              placeholder='{"key": "value"}'
            />
          </div>

          {/* Expected Status */}
          <div>
            <label className="block text-sm font-medium text-text-secondary mb-1">Expected Status</label>
            <input
              type="number"
              value={form.expected_status || ""}
              onChange={(e) =>
                setForm({ ...form, expected_status: parseInt(e.target.value) || undefined })
              }
              className="w-32 px-3 py-2"
              placeholder="200"
            />
          </div>
        </div>

        {/* Actions */}
        <div className="flex justify-end gap-3 mt-6">
          <button
            onClick={handleReplay}
            className="btn btn-primary"
          >
            Test Request
          </button>
          <button onClick={onClose} className="btn btn-ghost">
            Cancel
          </button>
          <button onClick={handleSave} className="btn btn-primary">
            Save
          </button>
        </div>
      </div>
    </div>
  );
}
