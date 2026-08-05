import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { BrowserMockAdapter } from "../../desktop/testing";
import type { DbStats } from "../../generated/desktop-contract";
import { GeneralTab } from "./GeneralTab";

const STATS: DbStats = {
  http_requests_count: 12,
  dns_queries_count: 8,
  devices_count: 2,
  app_tags_count: 3,
};

describe("GeneralTab", () => {
  it("loads General settings through the typed desktop contract", async () => {
    const adapter = new BrowserMockAdapter({
      get_keep_running: () => true,
      is_dashboard_running: () => true,
      get_dashboard_url: () => "http://192.168.1.40:9090?token=test",
      get_db_stats: () => STATS,
    });
    render(<GeneralTab contract={adapter.contract} />);

    expect(await screen.findByText("Database")).toBeInTheDocument();
    expect(screen.getByText("12")).toBeInTheDocument();
    expect(screen.getByText("http://192.168.1.40:9090?token=test")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Keep running after window closes" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(adapter.calls).toEqual(
      expect.arrayContaining([
        { command: "get_keep_running", args: {} },
        { command: "is_dashboard_running", args: {} },
        { command: "get_dashboard_url", args: {} },
        { command: "get_db_stats", args: {} },
      ]),
    );
  });

  it("keeps load failures visible and retryable", async () => {
    let shouldFail = true;
    const adapter = new BrowserMockAdapter({
      get_keep_running: () => false,
      is_dashboard_running: () => false,
      get_dashboard_url: () => "",
      get_db_stats: () => {
        if (shouldFail) {
          shouldFail = false;
          throw new Error("database is locked");
        }
        return STATS;
      },
    });
    const user = userEvent.setup();
    render(<GeneralTab contract={adapter.contract} />);

    expect(await screen.findByRole("alert")).toHaveTextContent("database is locked");
    await user.click(screen.getByRole("button", { name: "Retry" }));
    expect(await screen.findByText("Database")).toBeInTheDocument();
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("does not report mutations as successful when desktop commands fail", async () => {
    const adapter = new BrowserMockAdapter({
      get_keep_running: () => false,
      is_dashboard_running: () => false,
      get_dashboard_url: () => "",
      get_db_stats: () => STATS,
      set_keep_running: () => {
        throw new Error("preference write failed");
      },
      start_dashboard: () => {
        throw new Error("dashboard port unavailable");
      },
    });
    const user = userEvent.setup();
    render(<GeneralTab contract={adapter.contract} />);

    const keepRunning = await screen.findByRole("button", {
      name: "Keep running after window closes",
    });
    await user.click(keepRunning);
    expect(await screen.findByRole("alert")).toHaveTextContent("preference write failed");
    expect(keepRunning).toHaveAttribute("aria-pressed", "false");

    await user.click(screen.getByRole("button", { name: "Start" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("dashboard port unavailable");
    expect(screen.getByRole("button", { name: "Start" })).toBeVisible();
  });
});
