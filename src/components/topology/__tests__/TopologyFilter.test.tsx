import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { TopologyFilter } from "../TopologyFilter";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(() => Promise.resolve([])),
}));

describe("TopologyFilter", () => {
  it("calls onChange when host input changes", () => {
    const onChange = vi.fn();
    render(
      <TopologyFilter
        filter={{
          device_ids: null,
          app_tags: null,
          host_contains: null,
          time_window: { type: "session" },
          sync_global: false,
        }}
        onChange={onChange}
        onRefresh={() => {}}
      />,
    );
    const input = screen.getByPlaceholderText("Host contains...");
    fireEvent.change(input, { target: { value: "weixin" } });
    expect(onChange).toHaveBeenCalledWith(
      expect.objectContaining({ host_contains: "weixin" }),
    );
  });

  it("toggles sync_global checkbox", () => {
    const onChange = vi.fn();
    render(
      <TopologyFilter
        filter={{
          device_ids: null,
          app_tags: null,
          host_contains: null,
          time_window: { type: "session" },
          sync_global: false,
        }}
        onChange={onChange}
        onRefresh={() => {}}
      />,
    );
    const checkbox = screen.getByLabelText(/Sync global/);
    fireEvent.click(checkbox);
    expect(onChange).toHaveBeenCalledWith(
      expect.objectContaining({ sync_global: true }),
    );
  });

  it("calls onRefresh when Refresh button is clicked", () => {
    const onRefresh = vi.fn();
    render(
      <TopologyFilter
        filter={{
          device_ids: null,
          app_tags: null,
          host_contains: null,
          time_window: { type: "session" },
          sync_global: false,
        }}
        onChange={() => {}}
        onRefresh={onRefresh}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Refresh" }));
    expect(onRefresh).toHaveBeenCalled();
  });
});
