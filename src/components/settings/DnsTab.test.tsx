import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { BrowserMockAdapter } from "../../desktop/testing";
import { DnsTab } from "./DnsTab";

describe("DnsTab", () => {
  it("loads and saves the DNS upstream through the typed desktop contract", async () => {
    const adapter = new BrowserMockAdapter({
      get_dns_upstream: () => ({ upstream_type: "plainudp", address: "8.8.8.8:53" }),
      set_dns_upstream: () => undefined,
    });
    const user = userEvent.setup();
    render(<DnsTab contract={adapter.contract} />);

    const address = await screen.findByRole("textbox", { name: "DNS Server" });
    await user.clear(address);
    await user.type(address, "1.1.1.1:53");
    await user.click(screen.getByRole("button", { name: "Save" }));

    expect(await screen.findByText("DNS upstream saved")).toBeInTheDocument();
    expect(adapter.calls).toContainEqual({
      command: "set_dns_upstream",
      args: { upstream: { upstream_type: "plainudp", address: "1.1.1.1:53" } },
    });
  });

  it("keeps load failures visible and retryable", async () => {
    let shouldFail = true;
    const adapter = new BrowserMockAdapter({
      get_dns_upstream: () => {
        if (shouldFail) {
          shouldFail = false;
          throw new Error("DNS configuration unavailable");
        }
        return { upstream_type: "doh", address: "https://1.1.1.1/dns-query" };
      },
    });
    const user = userEvent.setup();
    render(<DnsTab contract={adapter.contract} />);

    expect(await screen.findByRole("alert")).toHaveTextContent("DNS configuration unavailable");
    await user.click(screen.getByRole("button", { name: "Retry" }));
    expect(await screen.findByRole("textbox", { name: "DoH URL" })).toHaveValue(
      "https://1.1.1.1/dns-query",
    );
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });
});
