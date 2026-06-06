import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { DevicesPage } from "../components/devices/DevicesPage";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn((cmd: string) => {
    if (cmd === "get_devices") return Promise.resolve([]);
    if (cmd === "get_network_info") return Promise.resolve({ lan_ip: "127.0.0.1" });
    return Promise.resolve(null);
  }),
}));

describe("DevicesPage", () => {
  it("shows empty state when no devices", async () => {
    render(<DevicesPage />);
    expect(await screen.findByText("No devices")).toBeInTheDocument();
  });
});
