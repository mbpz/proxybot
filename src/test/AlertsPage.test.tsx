import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { AlertsPage } from "../components/alerts/AlertsPage";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn((cmd: string) => {
    if (cmd === "get_alerts") return Promise.resolve([]);
    if (cmd === "get_alert_count") return Promise.resolve(0);
    if (cmd === "get_traffic_baseline") return Promise.resolve(null);
    return Promise.resolve(null);
  }),
}));

describe("AlertsPage", () => {
  it("renders alerts page and shows empty state", async () => {
    render(<AlertsPage />);
    expect(await screen.findByText("Scan Now")).toBeInTheDocument();
    expect(await screen.findByText("Baseline")).toBeInTheDocument();
    expect(await screen.findByText("No alerts")).toBeInTheDocument();
  });
});
