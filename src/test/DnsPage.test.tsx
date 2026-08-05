import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { DnsPage } from "../components/dns/DnsPage";
import { BrowserMockAdapter } from "../desktop/testing";
import type { DnsObservation } from "../generated/desktop-contract";

const FIRST_QUERY: DnsObservation = {
  domain: "api.example.com",
  timestamp_ms: 1_786_000_000_000,
  app_name: "Example",
  app_icon: null,
  action: "DIRECT",
  resolved_ips: ["203.0.113.10"],
  client_ip: "192.168.1.10",
};

describe("DnsPage", () => {
  it("loads and appends DNS observations through the typed desktop contract", async () => {
    const adapter = new BrowserMockAdapter({
      get_dns_log: () => [FIRST_QUERY],
      get_dns_upstream: () => ({ upstream_type: "plainudp", address: "8.8.8.8:53" }),
    });
    render(<DnsPage contract={adapter.contract} />);

    expect(await screen.findByText("api.example.com")).toBeInTheDocument();
    adapter.emit("dns-query", { ...FIRST_QUERY, domain: "cdn.example.com" });
    expect(await screen.findByText("cdn.example.com")).toBeInTheDocument();
    expect(adapter.calls).toEqual(
      expect.arrayContaining([
        { command: "get_dns_log", args: {} },
        { command: "get_dns_upstream", args: {} },
      ]),
    );
  });

  it("keeps partial load failures visible and retryable", async () => {
    let shouldFail = true;
    const adapter = new BrowserMockAdapter({
      get_dns_log: () => {
        if (shouldFail) {
          shouldFail = false;
          throw new Error("DNS store unavailable");
        }
        return [FIRST_QUERY];
      },
      get_dns_upstream: () => ({ upstream_type: "doh", address: "https://1.1.1.1/dns-query" }),
    });
    const user = userEvent.setup();
    render(<DnsPage contract={adapter.contract} />);

    expect(await screen.findByRole("alert")).toHaveTextContent("DNS store unavailable");
    await user.click(screen.getByRole("button", { name: "Retry" }));
    expect(await screen.findByText("api.example.com")).toBeInTheDocument();
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("does not switch upstream state when the mutation fails", async () => {
    const adapter = new BrowserMockAdapter({
      get_dns_log: () => [],
      get_dns_upstream: () => ({ upstream_type: "plainudp", address: "8.8.8.8:53" }),
      set_dns_upstream: () => {
        throw new Error("DoH endpoint rejected");
      },
    });
    const user = userEvent.setup();
    render(<DnsPage contract={adapter.contract} />);

    await screen.findByText("No DNS queries");
    await user.click(screen.getByRole("button", { name: "DoH" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("DoH endpoint rejected");
    expect(screen.getByRole("button", { name: "UDP" })).toHaveClass("btn-primary");
  });
});
