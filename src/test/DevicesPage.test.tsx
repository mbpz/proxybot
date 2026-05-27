import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { DevicesPage } from "../pages/DevicesPage";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue([]),
}));

describe("DevicesPage", () => {
  it("shows empty state when no devices", async () => {
    render(<DevicesPage networkInfo={null} />);
    expect(await screen.findByText("No devices")).toBeInTheDocument();
  });

  it("shows LAN IP in topology when device selected", async () => {
    const { invoke: mockInvoke } = await import("@tauri-apps/api/core");
    (mockInvoke as ReturnType<typeof vi.fn>).mockImplementation((cmd: string) => {
      if (cmd === "get_devices") return Promise.resolve([
        { id: 1, mac_address: "aa:bb:cc", name: "iPhone", created_at: "2024-01-01", last_seen_at: "2024-01-01", upload_bytes: 100, download_bytes: 200, rule_override: null },
      ]);
      return Promise.resolve(null);
    });
    render(<DevicesPage networkInfo={{ lan_ip: "10.0.0.1", interface: "en0" }} />);
    // Wait for device to appear, then click it to show topology
    const device = await screen.findByText("iPhone");
    device.click();
    expect(await screen.findByText("10.0.0.1")).toBeInTheDocument();
  });
});
