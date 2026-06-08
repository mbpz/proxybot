import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { TopologyPage } from "../components/topology/TopologyPage";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn((cmd: string) => {
    if (cmd === "build_topology_graph") return Promise.resolve({ nodes: [], edges: [], meta: {} });
    if (cmd === "get_topology_node_detail") return Promise.resolve(null);
    if (cmd === "get_devices") return Promise.resolve([]);
    return Promise.resolve(null);
  }),
}));

describe("TopologyPage", () => {
  it("renders page with title and view tabs", async () => {
    render(
      <MemoryRouter>
        <TopologyPage />
      </MemoryRouter>,
    );
    expect(await screen.findByText("Topology")).toBeInTheDocument();
    expect(await screen.findByText("Radial")).toBeInTheDocument();
    expect(await screen.findByText("Layered")).toBeInTheDocument();
    expect(await screen.findByText("Grouped")).toBeInTheDocument();
  });
});
