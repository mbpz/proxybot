import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { TrafficPage } from "../components/traffic/TrafficPage";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn((cmd: string) => {
    if (cmd === "list_filter_presets") return Promise.resolve([]);
    if (cmd === "get_traffic_page")
      return Promise.resolve({ records: [], total: 0, page: 0, page_size: 50, has_more: false });
    if (cmd === "load_history") return Promise.resolve([]);
    return Promise.resolve(null);
  }),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));

describe("TrafficPage", () => {
  it("renders request toolbar", async () => {
    render(<TrafficPage />);
    expect(await screen.findByText(/0 requests/)).toBeInTheDocument();
    expect(await screen.findByText("Load")).toBeInTheDocument();
  });
});
