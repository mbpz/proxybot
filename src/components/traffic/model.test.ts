import { describe, expect, it } from "vitest";
import { capturedRequestToListItem, normalizedRecordToListItem } from "./model";
import type { InterceptedRequest, NormalizedRecord } from "../../generated/desktop-contract";

describe("Traffic wire projections", () => {
  it("maps the Rust captured-request field names into the UI model", () => {
    const request: InterceptedRequest = {
      id: "req-1",
      timestamp: "1722427200.125",
      method: "POST",
      host: "api.example.com",
      path: "/items",
      query_params: null,
      status: 201,
      latency_ms: 12,
      scheme: "https",
      req_headers: [["content-type", "application/json"]],
      req_body: "{}",
      resp_headers: [],
      resp_body: null,
      resp_size: 42,
      app_name: "Example",
      app_icon: null,
      device_id: null,
      device_name: null,
      client_ip: null,
      upstream_ip: null,
      is_websocket: false,
      ws_frames: null,
      grpc_decoded: null,
      graphql_op: null,
    };

    expect(capturedRequestToListItem(request)).toMatchObject({
      durationMs: 12,
      appName: "Example",
      headers: { "content-type": "application/json" },
      body: "{}",
      size: 42,
    });
  });

  it("keeps normalized records visibly distinct from captured requests", () => {
    const record: NormalizedRecord = {
      id: 7,
      timestamp: "2026-07-31T12:00:00Z",
      method: "GET",
      path: "/health",
      query: {},
      request_headers: { accept: "application/json" },
      request_body: null,
      response_status: 200,
      response_headers: {},
      response_body: { ok: true },
      timing_ms: 3,
      device_id: null,
    };

    expect(normalizedRecordToListItem(record)).toMatchObject({
      id: "7",
      host: "",
      status: 200,
      durationMs: 3,
      headers: { accept: "application/json" },
    });
  });
});
