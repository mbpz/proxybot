import type {
  InterceptedRequest,
  JsonValue,
  NormalizedRecord,
} from "../../generated/desktop-contract";

/** UI projection; deliberately distinct from every desktop wire DTO. */
export interface TrafficListItem {
  id: string;
  method: string;
  host: string;
  path: string;
  status: number | null;
  durationMs: number | null;
  timestamp: string;
  appName: string | null;
  headers: Record<string, string>;
  body: string | null;
  size: number | null;
  isWebSocket: boolean;
}

export function capturedRequestToListItem(request: InterceptedRequest): TrafficListItem {
  return {
    id: request.id,
    method: request.method,
    host: request.host,
    path: request.path,
    status: request.status,
    durationMs: request.latency_ms,
    timestamp: request.timestamp,
    appName: request.app_name,
    headers: Object.fromEntries(request.req_headers),
    body: request.req_body,
    size: request.resp_size,
    isWebSocket: request.is_websocket,
  };
}

export function normalizedRecordToListItem(record: NormalizedRecord): TrafficListItem {
  return {
    id: String(record.id),
    method: record.method,
    host: "",
    path: record.path,
    status: record.response_status,
    durationMs: record.timing_ms,
    timestamp: record.timestamp,
    appName: null,
    headers: jsonHeaders(record.request_headers),
    body: jsonBody(record.request_body),
    size: jsonSize(record.response_body),
    isWebSocket: false,
  };
}

function jsonHeaders(value: JsonValue): Record<string, string> {
  if (!value || Array.isArray(value) || typeof value !== "object") return {};
  return Object.fromEntries(
    Object.entries(value).map(([name, header]) => [
      name,
      typeof header === "string" ? header : JSON.stringify(header),
    ]),
  );
}

function jsonBody(value: JsonValue): string | null {
  if (value === null) return null;
  return typeof value === "string" ? value : JSON.stringify(value, null, 2);
}

function jsonSize(value: JsonValue): number | null {
  const body = jsonBody(value);
  return body === null ? null : new TextEncoder().encode(body).length;
}
