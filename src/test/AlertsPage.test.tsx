import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AlertsPage } from "../components/alerts/AlertsPage";
import { BrowserMockAdapter } from "../desktop/testing";
import type { TrafficBaseline } from "../generated/desktop-contract";

const baseline: TrafficBaseline = {
  device_id: null,
  domains: [
    {
      value: "api.example.com",
      count: 3,
      first_seen: "2026-08-01 12:00:00",
      last_seen: "2026-08-04 12:00:00",
    },
  ],
  ips: [],
};

function createAdapter(getBaseline: () => TrafficBaseline = () => baseline) {
  return new BrowserMockAdapter({
    get_alerts: () => [],
    get_alert_count: () => 0,
    get_traffic_baseline: getBaseline,
    scan_request_anomalies: () => ({
      new_domains: [],
      new_ips: [],
      privacy_findings: [],
      alerts_generated: 0,
    }),
  });
}

describe("AlertsPage", () => {
  it("loads typed alerts and baseline data without a context-free anomaly scan", async () => {
    const user = userEvent.setup();
    const adapter = createAdapter();

    render(<AlertsPage contract={adapter.contract} />);

    expect(await screen.findByText("No alerts")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Baseline" }));
    expect(await screen.findByText("api.example.com")).toBeInTheDocument();

    expect(screen.queryByRole("button", { name: /scan now/i })).not.toBeInTheDocument();
    expect(adapter.calls.some(({ command }) => command === "scan_request_anomalies")).toBe(false);
  });

  it("surfaces a failed baseline load and retries it", async () => {
    const user = userEvent.setup();
    let shouldFail = true;
    const adapter = createAdapter(() => {
      if (shouldFail) throw new Error("baseline offline");
      return baseline;
    });

    render(<AlertsPage contract={adapter.contract} />);

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Could not load traffic baseline: baseline offline",
    );
    shouldFail = false;
    await user.click(screen.getByRole("button", { name: "Retry" }));
    await user.click(screen.getByRole("button", { name: "Baseline" }));
    expect(await screen.findByText("api.example.com")).toBeInTheDocument();
  });
});
