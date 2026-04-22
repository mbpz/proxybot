import { useState } from "react";

interface InterceptedRequest {
  id: string;
  timestamp: string;
  method: string;
  host: string;
  path: string;
  query_params?: string;
  status: number | null;
  latency_ms: number | null;
  scheme: string;
  req_headers: [string, string][];
  req_body?: string;
  resp_headers: [string, string][];
  resp_body?: string;
  resp_size?: number;
  app_name?: string;
  app_icon?: string;
  is_websocket: boolean;
  ws_frames?: WsFrame[];
}

interface WsFrame {
  direction: string;
  timestamp: string;
  payload: string;
  size: number;
}

type DetailTab = "headers" | "params" | "body" | "ws";

interface Props {
  request: InterceptedRequest | null;
}

function formatBody(body: string | undefined, headers: [string, string][]): string {
  if (!body) return "";
  const contentType = headers.find(([name]) => name.toLowerCase() === "content-type");
  if (contentType && contentType[1].includes("application/json")) {
    try {
      return JSON.stringify(JSON.parse(body), null, 2);
    } catch {
      return body;
    }
  }
  return body;
}

function copyToClipboard(text: string) {
  navigator.clipboard.writeText(text).catch(console.error);
}

export function RequestDetail({ request }: Props) {
  const [tab, setTab] = useState<DetailTab>("headers");

  if (!request) {
    return (
      <div className="panel">
        <div className="panel-header">
          <span className="panel-title">Request Detail</span>
        </div>
        <div className="panel-body">
          <div className="empty-state">
            <div className="empty-state-icon">👆</div>
            <div className="empty-state-title">Select a request</div>
            <div className="empty-state-description">
              Click on a request in the list above to see its details.
            </div>
          </div>
        </div>
      </div>
    );
  }

  const parsedBody = formatBody(request.resp_body, request.resp_headers);

  return (
    <div className="panel">
      <div className="panel-header">
        <span className="panel-title">
          <span className={`badge badge-${request.method.toLowerCase()}`} style={{ marginRight: 8 }}>
            {request.method}
          </span>
          {request.host}
          <span className="text-muted">{request.path}</span>
        </span>
        <button
          className="btn btn-sm btn-ghost"
          onClick={() => copyToClipboard(`${request.scheme}://${request.host}${request.path}`)}
          title="Copy URL"
        >
          📋
        </button>
      </div>

      <div className="tabs">
        <button className={`tab ${tab === "headers" ? "active" : ""}`} onClick={() => setTab("headers")}>
          Headers
        </button>
        <button className={`tab ${tab === "params" ? "active" : ""}`} onClick={() => setTab("params")}>
          Params
        </button>
        <button className={`tab ${tab === "body" ? "active" : ""}`} onClick={() => setTab("body")}>
          Body
        </button>
        {request.is_websocket && (
          <button className={`tab ${tab === "ws" ? "active" : ""}`} onClick={() => setTab("ws")}>
            WS Frames
          </button>
        )}
      </div>

      <div className="panel-body" style={{ maxHeight: 400, overflowY: "auto" }}>
        {tab === "headers" && (
          <div>
            <div style={{ marginBottom: "var(--space-4)" }}>
              <div className="card-title" style={{ marginBottom: "var(--space-2)" }}>Request Headers</div>
              <table className="table">
                <tbody>
                  {request.req_headers.map(([name, value]) => (
                    <tr key={name}>
                      <td style={{ fontWeight: 500, whiteSpace: "nowrap" }}>{name}</td>
                      <td className="mono" style={{ wordBreak: "break-all" }}>{value}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            <div>
              <div className="card-title" style={{ marginBottom: "var(--space-2)" }}>Response Headers</div>
              <table className="table">
                <tbody>
                  {request.resp_headers.map(([name, value]) => (
                    <tr key={name}>
                      <td style={{ fontWeight: 500, whiteSpace: "nowrap" }}>{name}</td>
                      <td className="mono" style={{ wordBreak: "break-all" }}>{value}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        )}

        {tab === "params" && (
          <div>
            {request.query_params ? (
              <div>
                {request.query_params.split("&").map((param) => {
                  const [key, value] = param.split("=");
                  return (
                    <div key={key} style={{ display: "flex", gap: "var(--space-3)", padding: "var(--space-1) 0", borderBottom: "1px solid var(--border)" }}>
                      <span className="mono" style={{ color: "var(--accent-blue)", minWidth: 120 }}>{decodeURIComponent(key)}</span>
                      <span className="mono" style={{ color: "var(--text-primary)" }}>{decodeURIComponent(value || "")}</span>
                    </div>
                  );
                })}
              </div>
            ) : (
              <div className="empty-state" style={{ padding: "var(--space-6)" }}>
                <div className="text-muted">No query parameters</div>
              </div>
            )}
          </div>
        )}

        {tab === "body" && (
          <div>
            {parsedBody ? (
              <div style={{ position: "relative" }}>
                <button
                  className="btn btn-sm btn-ghost"
                  style={{ position: "absolute", top: 0, right: 0 }}
                  onClick={() => copyToClipboard(parsedBody)}
                >
                  📋 Copy
                </button>
                <pre className="mono" style={{ fontSize: "var(--text-xs)", whiteSpace: "pre-wrap", wordBreak: "break-all", background: "var(--bg-primary)", padding: "var(--space-3)", borderRadius: "var(--radius-md)" }}>
                  {parsedBody}
                </pre>
              </div>
            ) : (
              <div className="empty-state" style={{ padding: "var(--space-6)" }}>
                <div className="text-muted">No response body</div>
              </div>
            )}
          </div>
        )}

        {tab === "ws" && request.ws_frames && (
          <div>
            {request.ws_frames.length === 0 ? (
              <div className="empty-state" style={{ padding: "var(--space-6)" }}>
                <div className="text-muted">No WebSocket frames captured</div>
              </div>
            ) : (
              <div>
                {request.ws_frames.map((frame, i) => (
                  <div
                    key={i}
                    style={{
                      display: "flex",
                      gap: "var(--space-3)",
                      padding: "var(--space-2)",
                      borderBottom: "1px solid var(--border)",
                      alignItems: "flex-start",
                    }}
                  >
                    <span
                      className={`badge ${frame.direction === "←" || frame.direction === "IN" ? "badge-get" : "badge-post"}`}
                      style={{ flexShrink: 0 }}
                    >
                      {frame.direction}
                    </span>
                    <div style={{ flex: 1, minWidth: 0 }}>
                      <div className="text-xs text-muted" style={{ marginBottom: 2 }}>
                        {frame.timestamp} · {frame.size}B
                      </div>
                      <pre className="mono" style={{ fontSize: "var(--text-xs)", whiteSpace: "pre-wrap", wordBreak: "break-all", margin: 0 }}>
                        {frame.payload}
                      </pre>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
