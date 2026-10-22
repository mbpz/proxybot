import { test, expect } from "@playwright/test";
import { mockTauriCommands } from "./fixtures/tauri-mock";

const BASE_MOCKS = {
  is_dashboard_running: false,
  get_dashboard_url: "",
  get_network_info: { lan_ip: "192.168.1.100", interface: "en0" },
  is_pf_enabled: false,
  is_tun_enabled: false,
  get_ca_metadata: null,
  get_dns_log: [],
  get_dns_upstream: { upstream_type: "plainudp", address: "8.8.8.8:53" },
  list_filter_presets: [],
  get_replay_targets: [],
  get_rules: [],
  get_devices: [],
  get_ca_cert_pem: "",
  list_rule_files: [],
  get_alerts: [],
  get_alert_count: 0,
  get_traffic_baseline: { device_id: null, domains: [], ips: [] },
  get_graph_data: { requests: [] },
};

test.describe("Product navigation", () => {
  test.beforeEach(async ({ page }) => {
    await mockTauriCommands(page, BASE_MOCKS);
    await page.goto("/");
  });

  test("defaults to the Capture destination", async ({ page }) => {
    const captureLink = page.locator("aside").getByRole("link", { name: "Capture" });
    await expect(captureLink).toHaveAttribute("aria-current", "page");
    await expect(page.getByRole("navigation", { name: "Capture views" })).toBeVisible();
    await expect(page.getByText("No requests captured yet")).toBeVisible();
  });

  test("navigates to Rules from the Capture context", async ({ page }) => {
    await page
      .getByRole("navigation", { name: "Capture views" })
      .getByRole("link", { name: "Rules" })
      .click();
    await expect(page).toHaveURL(/\/rules/);
    await expect(page.getByText("No rules configured")).toBeVisible();
  });

  test("navigates to Setup page", async ({ page }) => {
    await page.locator("aside").getByRole("link", { name: "Setup" }).click();
    await expect(page).toHaveURL(/\/setup/);
  });

  test("navigates through Capture views", async ({ page }) => {
    const captureViews = page.getByRole("navigation", { name: "Capture views" });
    await captureViews.getByRole("link", { name: "DNS" }).click();
    await expect(page).toHaveURL(/\/dns/);
    await expect(page.getByText("DNS Queries", { exact: true })).toBeVisible();
    await captureViews.getByRole("link", { name: "Alerts" }).click();
    await expect(page).toHaveURL(/\/alerts/);
    await expect(page.getByText("No alerts")).toBeVisible();
    await page.getByRole("tab", { name: "Baseline" }).click();
    await expect(page.getByText(/No domain baseline yet/)).toBeVisible();
    // Analysis is Labs-only: it joins the Capture context after opt-in.
    await expect(captureViews.getByRole("link", { name: "Analysis" })).toHaveCount(0);
  });

  test("keeps Composer inside the Capture context", async ({ page }) => {
    await page
      .getByRole("navigation", { name: "Capture views" })
      .getByRole("link", { name: "Replay" })
      .click();
    await expect(page).toHaveURL(/\/replay/);
    await page
      .getByRole("navigation", { name: "Replay tools" })
      .getByRole("link", { name: "Composer" })
      .click();
    await expect(page).toHaveURL(/\/composer/);
    await expect(page.locator("aside").getByRole("link", { name: "Capture" })).toHaveAttribute(
      "aria-current",
      "page",
    );
  });

  test("exposes only three destinations in the default sidebar", async ({ page }) => {
    const sidebarLinks = page.locator("aside").getByRole("link");
    await expect(sidebarLinks).toHaveCount(3);
    await expect(sidebarLinks).toHaveText(["Capture", "Setup", "Settings"]);
  });

  test("Labs routes are gated by default and point to Settings", async ({ page }) => {
    await page.goto("/graph");
    await expect(page.getByText("Labs are off")).toBeVisible();
    await expect(page.getByRole("link", { name: "Open Settings → Labs" })).toBeVisible();
  });
});
