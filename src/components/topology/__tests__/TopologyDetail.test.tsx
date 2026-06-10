import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { TopologyDetail } from "../TopologyDetail";

const EMPTY_FILTER = {
  device_ids: null,
  app_tags: null,
  host_contains: null,
  time_window: { type: "session" as const },
  sync_global: false,
};

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn((cmd: string) => {
    if (cmd === "get_topology_node_detail") {
      return Promise.resolve({
        node: {
          id: "host:test.com",
          kind: "host",
          label: "test.com",
          app_tag: null,
          device_id: null,
          request_count: 10,
          total_bytes: 0,
          avg_latency_ms: 50,
          error_count: 2,
          error_rate: 0.2,
          last_seen: 0,
        },
        recent_requests: [
          {
            id: "1",
            method: "GET",
            host: "test.com",
            path: "/api",
            status: 200,
            duration_ms: 50,
            timestamp: 0,
          },
        ],
        status_breakdown: [
          { status_class: "2xx", count: 8 },
          { status_class: "4xx", count: 2 },
        ],
      });
    }
    return Promise.resolve(null);
  }),
}));

describe("TopologyDetail", () => {
  it("renders nothing when nodeId is null", () => {
    const { container } = render(
      <MemoryRouter>
        <TopologyDetail nodeId={null} filter={EMPTY_FILTER} onClose={() => {}} />
      </MemoryRouter>,
    );
    expect(container.firstChild).toBeNull();
  });

  it("loads and displays node metrics when nodeId is set", async () => {
    render(
      <MemoryRouter>
        <TopologyDetail
          nodeId="host:test.com"
          filter={EMPTY_FILTER}
          onClose={() => {}}
        />
      </MemoryRouter>,
    );
    expect(await screen.findByText("Metrics")).toBeInTheDocument();
    expect(await screen.findByText(/Requests:/)).toBeInTheDocument();
    expect(await screen.findByText(/Status breakdown/)).toBeInTheDocument();
    expect(await screen.findByText(/Recent requests/)).toBeInTheDocument();
    expect(await screen.findByText("View in Traffic")).toBeInTheDocument();
  });

  it("calls onClose when close button is clicked", () => {
    const onClose = vi.fn();
    render(
      <MemoryRouter>
        <TopologyDetail
          nodeId="host:test.com"
          filter={EMPTY_FILTER}
          onClose={onClose}
        />
      </MemoryRouter>,
    );
    const buttons = screen.getAllByRole("button");
    fireEvent.click(buttons[0]);
    expect(onClose).toHaveBeenCalled();
  });
});
