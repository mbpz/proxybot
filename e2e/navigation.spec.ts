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
  get_dns_upstream: "8.8.8.8",
  list_filter_presets: [],
  get_replay_targets: [],
  get_rules: [],
  get_devices: [],
  get_ca_cert_pem: "",
  list_rule_files: [],
  get_alerts: [],
  get_alert_count: 0,
  get_traffic_baseline: null,
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

  test("navigates to Rules page", async ({ page }) => {
    await page.getByRole("link", { name: "Rules" }).click();
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
    await captureViews.getByRole("link", { name: "Graph" }).click();
    await expect(page).toHaveURL(/\/graph/);
    await captureViews.getByRole("link", { name: "Topology" }).click();
    await expect(page).toHaveURL(/\/topology/);
  });

  test("keeps Composer in the Replay destination", async ({ page }) => {
    await page.locator("aside").getByRole("link", { name: "Replay" }).click();
    await expect(page).toHaveURL(/\/replay/);
    await page
      .getByRole("navigation", { name: "Replay tools" })
      .getByRole("link", { name: "Composer" })
      .click();
    await expect(page).toHaveURL(/\/composer/);
    await expect(page.locator("aside").getByRole("link", { name: "Replay" })).toHaveAttribute(
      "aria-current",
      "page",
    );
  });

  test("exposes only five destinations in the default sidebar", async ({ page }) => {
    const sidebarLinks = page.locator("aside").getByRole("link");
    await expect(sidebarLinks).toHaveCount(5);
    await expect(sidebarLinks).toHaveText(["Capture", "Setup", "Rules", "Replay", "Settings"]);
  });
});
