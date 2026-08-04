import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { TrafficPage } from "../components/traffic/TrafficPage";
import type { InterceptedRequest } from "../generated/desktop-contract";

const { invokeMock, listenMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  listenMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));
vi.mock("@tauri-apps/api/event", () => ({ listen: listenMock }));

let receiveEvent: ((event: { payload: unknown }) => void) | undefined;

const request: InterceptedRequest = {
  id: "7",
  timestamp: "1722427200.125",
  method: "GET",
  host: "api.example.com",
  path: "/items",
  query_params: null,
  status: 200,
  latency_ms: 12,
  scheme: "https",
  req_headers: [],
  req_body: null,
  resp_headers: [],
  resp_body: null,
  resp_size: 42,
  app_name: "Example",
  app_icon: null,
  device_id: null,
  device_name: null,
  client_ip: "10.0.0.2",
  upstream_ip: "203.0.113.8",
  is_websocket: false,
  ws_frames: null,
  grpc_decoded: null,
  graphql_op: null,
};

beforeEach(() => {
  invokeMock.mockReset();
  listenMock.mockReset();
  receiveEvent = undefined;
  listenMock.mockImplementation(async (_event: string, receive: typeof receiveEvent) => {
    receiveEvent = receive;
    return () => {};
  });
  invokeMock.mockImplementation((command: string) => {
    if (command === "list_filter_presets") return Promise.resolve([]);
    if (command === "parse_filter") return Promise.resolve({ ok: true, error: null });
    if (command === "get_traffic_page") {
      return Promise.resolve({
        records: [],
        normalized_records: [],
        total: 0,
        page: 0,
        page_size: 50,
        has_more: false,
      });
    }
    if (command === "load_history") return Promise.resolve([]);
    return Promise.resolve(undefined);
  });
});

describe("TrafficPage", () => {
  it("loads one generated-contract query for the initial result set", async () => {
    render(<TrafficPage />);

    expect(await screen.findByText(/0 requests/)).toBeInTheDocument();
    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith("get_traffic_page", {
        query: expect.objectContaining({ expression: "", order: "newest", page_size: 50 }),
        records: null,
      }),
    );
    expect(await screen.findByText("Load")).toBeInTheDocument();
  });

  it("submits a Filter DSL result set once instead of evaluating every row", async () => {
    render(<TrafficPage />);
    await waitFor(() =>
      expect(invokeMock.mock.calls.filter(([command]) => command === "get_traffic_page")).toHaveLength(1),
    );

    fireEvent.change(screen.getByTestId("filter-input"), {
      target: { value: "method:GET" },
    });

    await waitFor(() =>
      expect(invokeMock.mock.calls.filter(([command]) => command === "get_traffic_page")).toHaveLength(2),
    );
    expect(invokeMock.mock.calls.some(([command]) => command === "evaluate_filter")).toBe(false);
  });

  it("coalesces a Capture Event burst into one persisted query refresh", async () => {
    render(<TrafficPage />);
    await waitFor(() =>
      expect(invokeMock.mock.calls.filter(([command]) => command === "get_traffic_page")).toHaveLength(1),
    );
    await waitFor(() => expect(receiveEvent).toBeDefined());

    act(() => {
      receiveEvent?.({ payload: request });
      receiveEvent?.({ payload: { ...request, id: "8" } });
      receiveEvent?.({ payload: { ...request, id: "9" } });
    });

    await waitFor(() =>
      expect(invokeMock.mock.calls.filter(([command]) => command === "get_traffic_page")).toHaveLength(2),
    );
  });
});
