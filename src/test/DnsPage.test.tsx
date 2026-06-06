import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { DnsPage } from "../components/dns/DnsPage";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn((cmd: string) => {
    if (cmd === "get_dns_log") return Promise.resolve([]);
    if (cmd === "get_dns_upstream")
      return Promise.resolve({ upstream_type: "plainudp", address: "8.8.8.8:53" });
    if (cmd === "get_block_lists") return Promise.resolve([]);
    return Promise.resolve(null);
  }),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));

describe("DnsPage", () => {
  it("renders DNS queries panel and shows empty state", async () => {
    render(<DnsPage />);
    expect(await screen.findByText("DNS Queries")).toBeInTheDocument();
    expect(await screen.findByText("No DNS queries")).toBeInTheDocument();
  });
});
