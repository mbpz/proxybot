import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { SettingsPage } from "../components/settings/SettingsPage";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn((cmd: string) => {
    if (cmd === "get_keep_running") return Promise.resolve(false);
    if (cmd === "is_dashboard_running") return Promise.resolve(false);
    if (cmd === "get_dashboard_url") return Promise.resolve("");
    if (cmd === "get_db_stats") return Promise.resolve(null);
    if (cmd === "get_network_info") return Promise.resolve({ lan_ip: "127.0.0.1" });
    if (cmd === "get_dns_upstream")
      return Promise.resolve({ upstream_type: "plainudp", address: "8.8.8.8:53" });
    if (cmd === "get_ca_metadata") return Promise.resolve(null);
    if (cmd === "get_app_version") return Promise.resolve("0.1.0");
    return Promise.resolve(null);
  }),
}));

describe("SettingsPage", () => {
  it("renders settings page with tabs", async () => {
    render(<SettingsPage />);
    expect(await screen.findByText("Settings")).toBeInTheDocument();
    expect(await screen.findByText("General")).toBeInTheDocument();
    expect(await screen.findByText("Network")).toBeInTheDocument();
  });
});
