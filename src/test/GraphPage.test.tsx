import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { GraphPage } from "../components/graph/GraphPage";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn((cmd: string) => {
    if (cmd === "get_graph_data") return Promise.resolve(null);
    if (cmd === "get_traffic_dag") return Promise.resolve(null);
    if (cmd === "get_device_dag") return Promise.resolve(null);
    if (cmd === "get_devices") return Promise.resolve([]);
    return Promise.resolve(null);
  }),
}));

describe("GraphPage", () => {
  it("renders graph page with build DAG button", async () => {
    render(<GraphPage />);
    expect(await screen.findByText("Graph")).toBeInTheDocument();
    expect(await screen.findByText("Build DAG")).toBeInTheDocument();
  });
});
