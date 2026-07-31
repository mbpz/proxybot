import { describe, expect, it, vi } from "vitest";
import { DesktopError } from "./contract";
import { BrowserMockAdapter } from "./testing";
import type { InterceptedRequest, WsFrame } from "../generated/desktop-contract";

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
  is_websocket: true,
  ws_frames: [frame],
  grpc_decoded: null,
  graphql_op: null,
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
