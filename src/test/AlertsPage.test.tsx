import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { AlertsPage } from "../components/alerts/AlertsPage";

// Sample alert data for tests
const mockAlerts = [
  {
    id: 1,
    device_id: null,
    severity: "Critical" as const,
    alert_type: "traffic",
    details: "High traffic volume from Douyin",
    created_at: new Date(Date.now() - 2 * 60 * 1000).toISOString(),
    acknowledged: false,
  },
  {
    id: 2,
    device_id: 1,
    severity: "Warning" as const,
    alert_type: "device",
    details: "New device connected: iPhone 15 Pro",
    created_at: new Date(Date.now() - 15 * 60 * 1000).toISOString(),
    acknowledged: false,
  },
];

// Mutable mock response holder
let mockResponse: unknown = mockAlerts;

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(() => Promise.resolve(mockResponse)),
}));

describe("AlertsPage", () => {
  beforeEach(() => {
    mockResponse = mockAlerts;
    vi.clearAllMocks();
  });

  it("renders alerts page header", async () => {
    render(<AlertsPage />);
    expect(await screen.findByText("Alerts")).toBeInTheDocument();
  });

  it("renders toggle all button in ON state", async () => {
    render(<AlertsPage />);
    expect(await screen.findByText("ON")).toBeInTheDocument();
  });

  it("renders alert items with messages", async () => {
    render(<AlertsPage />);
    expect(await screen.findByText("High traffic volume from Douyin")).toBeInTheDocument();
    expect(await screen.findByText("New device connected: iPhone 15 Pro")).toBeInTheDocument();
  });

  it("renders alert count badge when alerts exist", async () => {
    render(<AlertsPage />);
    expect(await screen.findByText("2 active")).toBeInTheDocument();
  });
});

describe("AlertsPage empty state", () => {
  beforeEach(() => {
    mockResponse = [];
    vi.clearAllMocks();
  });

  it("shows empty state when no alerts", async () => {
    render(<AlertsPage />);
    await waitFor(async () => {
      expect(await screen.findByText("No alerts")).toBeInTheDocument();
    });
  });
});