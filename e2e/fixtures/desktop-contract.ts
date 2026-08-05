import type {
  InterceptedRequest,
  WsFrame,
} from "../../src/generated/desktop-contract";

type CapturedRequestOverrides = Partial<InterceptedRequest> &
  Pick<InterceptedRequest, "host" | "id">;

/** Build a complete Rust wire DTO so E2E fixtures cannot silently use UI-only shapes. */
export function capturedRequest(
  overrides: CapturedRequestOverrides,
): InterceptedRequest {
  return {
    id: overrides.id,
    timestamp: "2026-08-01T00:00:00Z",
    method: "GET",
    host: overrides.host,
    path: "/",
    query_params: null,
    status: null,
    latency_ms: null,
    scheme: "https",
    req_headers: [],
    req_body: null,
    resp_headers: [],
    resp_body: null,
    resp_size: null,
    app_name: null,
    app_icon: null,
    device_id: null,
    device_name: null,
    client_ip: null,
    upstream_ip: null,
    is_websocket: false,
    ws_frames: null,
    grpc_decoded: null,
    graphql_op: null,
    ...overrides,
  };
}

export type { InterceptedRequest, WsFrame };
