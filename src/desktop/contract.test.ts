import { describe, expect, it, vi } from "vitest";
import { DesktopError } from "./contract";
import { BrowserMockAdapter } from "./testing";
import type { InterceptedRequest, Rule, WsFrame } from "../generated/desktop-contract";

const frame: WsFrame = {
  direction: "incoming",
  timestamp: "2026-07-31T12:00:00Z",
  payload: "hello",
  size: 5,
  opcode: 1,
  truncated: false,
};

const request: InterceptedRequest = {
  id: "req-1",
  timestamp: "2026-07-31T12:00:00Z",
  method: "GET",
  host: "example.com",
  path: "/socket",
  query_params: null,
  status: 101,
  latency_ms: 4,
  scheme: "https",
  req_headers: [["upgrade", "websocket"]],
  req_body: null,
  resp_headers: [],
  resp_body: null,
  resp_size: 5,
  app_name: null,
  app_icon: null,
  device_id: null,
  device_name: null,
  client_ip: null,
  upstream_ip: null,
  is_websocket: true,
  ws_frames: [frame],
  grpc_decoded: null,
  graphql_op: null,
};

const rule: Rule = {
  pattern: "DOMAIN-SUFFIX",
  value: "example.com",
  action: { type: "MAPREMOTE", target: "https://mock.local" },
  name: "mock",
  priority: 10,
  enabled: true,
  comment: "",
};

describe("Desktop contract Adapter conformance", () => {
  it("types, validates and records command calls", async () => {
    const adapter = new BrowserMockAdapter({
      get_ws_frames: ({ requestId }) => (requestId === request.id ? [frame] : []),
    });

    await expect(adapter.contract.call("get_ws_frames", { requestId: request.id })).resolves.toEqual([frame]);
    expect(adapter.calls).toEqual([
      { command: "get_ws_frames", args: { requestId: request.id } },
    ]);
  });

  it("rejects unhandled commands and invalid results instead of returning null", async () => {
    const strict = new BrowserMockAdapter();
    await expect(strict.contract.call("load_history", {})).rejects.toMatchObject({
      kind: "contract",
      code: "unhandled_mock_command",
    });

    const invalid = new BrowserMockAdapter({ get_ws_frames: () => [request] as unknown as WsFrame[] });
    await expect(invalid.contract.call("get_ws_frames", { requestId: request.id })).rejects.toMatchObject({
      kind: "contract",
    });
  });

  it("validates tagged Rule actions and unit mutation results", async () => {
    const adapter = new BrowserMockAdapter({
      get_rules: ({ filename }) => (filename === "custom.yaml" ? [rule] : []),
      save_rule: () => undefined,
    });

    await expect(adapter.contract.call("get_rules", { filename: "custom.yaml" })).resolves.toEqual([rule]);
    await expect(adapter.contract.call("save_rule", {
      rule,
      filename: "custom.yaml",
      originalRule: null,
    })).resolves.toBeUndefined();

    const invalid = new BrowserMockAdapter({
      get_rules: () => [{ ...rule, action: "DIRECT" } as unknown as Rule],
    });
    await expect(invalid.contract.call("get_rules", { filename: "custom.yaml" })).rejects.toMatchObject({
      kind: "contract",
    });
  });

  it("validates the complete Captured Request query contract", async () => {
    const adapter = new BrowserMockAdapter({
      get_traffic_page: ({ query }) => ({
        records: query.expression === "method:GET" ? [request] : [],
        normalized_records: [],
        total: query.expression === "method:GET" ? 1 : 0,
        page: query.page,
        page_size: query.page_size,
        has_more: false,
      }),
      parse_filter: ({ expr }) => ({ ok: expr === "method:GET", error: null }),
      save_filter_preset: () => undefined,
    });
    const query = {
      expression: "method:GET",
      method: null,
      host: null,
      status: null,
      application: null,
      search: null,
      order: "newest" as const,
      page: 0,
      page_size: 50,
    };

    await expect(
      adapter.contract.call("get_traffic_page", { query, records: null }),
    ).resolves.toMatchObject({ records: [request], total: 1 });
    await expect(adapter.contract.call("parse_filter", { expr: "method:GET" })).resolves.toEqual({
      ok: true,
      error: null,
    });
    await expect(
      adapter.contract.call("save_filter_preset", {
        preset: { id: "one", name: "GET", expr: "method:GET" },
      }),
    ).resolves.toBeUndefined();

    const invalid = new BrowserMockAdapter({
      get_traffic_page: () =>
        ({ records: [], total: 0, page: 0, page_size: 50, has_more: false }) as never,
    });
    await expect(
      invalid.contract.call("get_traffic_page", { query, records: null }),
    ).rejects.toMatchObject({ kind: "contract" });
  });

  it("preserves event order and makes disposal idempotent", async () => {
    const adapter = new BrowserMockAdapter();
    const received: string[] = [];
    const subscription = adapter.contract.subscribe("ws-frame:new", {
      next: (event) => received.push(event.frame.payload),
    });
    await subscription.ready;

    adapter.emit("ws-frame:new", { request_id: request.id, frame: { ...frame, payload: "one" } });
    adapter.emit("ws-frame:new", { request_id: request.id, frame: { ...frame, payload: "two" } });
    subscription.dispose();
    subscription.dispose();
    adapter.emit("ws-frame:new", { request_id: request.id, frame: { ...frame, payload: "three" } });

    expect(received).toEqual(["one", "two"]);
  });

  it("can dispose before asynchronous listener registration completes", async () => {
    const adapter = new BrowserMockAdapter();
    const next = vi.fn();
    const subscription = adapter.contract.subscribe("ws-frame:new", { next });

    subscription.dispose();
    await subscription.ready;
    adapter.emit("ws-frame:new", { request_id: request.id, frame });

    expect(next).not.toHaveBeenCalled();
  });

  it("reports invalid event payloads through the observer", async () => {
    const adapter = new BrowserMockAdapter();
    const onError = vi.fn<(error: DesktopError) => void>();
    const subscription = adapter.contract.subscribe("intercepted-request", {
      next: vi.fn(),
      error: onError,
    });
    await subscription.ready;

    adapter.emit("intercepted-request", { ...request, req_headers: null } as unknown as InterceptedRequest);
    expect(onError).toHaveBeenCalledWith(expect.objectContaining({ kind: "contract" }));
  });
});
