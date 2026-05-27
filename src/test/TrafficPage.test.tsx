import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { TrafficPage } from "../pages/TrafficPage";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn((cmd: string) => {
    if (cmd === "get_replay_targets") return Promise.resolve([]);
    return Promise.resolve(null);
  }),
}));

const mockRequest = {
  id: "1",
  timestamp: "1713000000.123",
  method: "GET",
  host: "api.example.com",
  path: "/users",
  status: 200,
  latency_ms: 42,
  scheme: "https",
  req_headers: [["authorization", "Bearer token123"]] as [string, string][],
  resp_headers: [["content-type", "application/json"]] as [string, string][],
  resp_body: '{"users":[]}',
  is_websocket: false,
};

describe("TrafficPage", () => {
  it("shows empty state when no requests", () => {
    render(<TrafficPage requests={[]} onError={() => {}} />);
    expect(screen.getByText("No requests captured")).toBeInTheDocument();
  });

  it("renders request list with entries", () => {
    render(<TrafficPage requests={[mockRequest]} onError={() => {}} />);
    // Host appears in both list and filter dropdown — check list has it
    const hosts = screen.getAllByText("api.example.com");
    expect(hosts.length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText("42ms")).toBeInTheDocument();
  });

  it("shows host filter options", () => {
    render(<TrafficPage requests={[mockRequest]} onError={() => {}} />);
    expect(screen.getByText("All Hosts")).toBeInTheDocument();
  });

  it("shows app tab filters", () => {
    render(<TrafficPage requests={[mockRequest]} onError={() => {}} />);
    expect(screen.getByText("All")).toBeInTheDocument();
    expect(screen.getByText("WeChat")).toBeInTheDocument();
    expect(screen.getByText("Douyin")).toBeInTheDocument();
  });

  it("shows request count", () => {
    render(<TrafficPage requests={[mockRequest, { ...mockRequest, id: "2" }]} onError={() => {}} />);
    expect(screen.getByText("2 requests")).toBeInTheDocument();
  });
});
