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

test.describe("Sidebar navigation", () => {
  test.beforeEach(async ({ page }) => {
    await mockTauriCommands(page, BASE_MOCKS);
    await page.goto("/");
  });

  test("defaults to traffic page", async ({ page }) => {
    // Traffic link should be active (has border-accent-blue class after theme refactor)
    const trafficLink = page.getByRole("link", { name: "Traffic" });
    await expect(trafficLink).toBeVisible();
    await expect(trafficLink).toHaveClass(/border-accent-blue/);
    // Traffic page content should be visible
    await expect(page.getByText("No requests captured yet")).toBeVisible();
  });

  test("navigates to Rules page", async ({ page }) => {
    await page.getByRole("link", { name: "Rules" }).click();
    await expect(page).toHaveURL(/\/rules/);
  });

  test("navigates to Certs page", async ({ page }) => {
    await page.getByRole("link", { name: "Certs" }).click();
    await expect(page).toHaveURL(/\/certs/);
  });

  test("navigates to Devices page", async ({ page }) => {
    await page.getByRole("link", { name: "Devices" }).click();
    await expect(page).toHaveURL(/\/devices/);
  });

  test("navigates to DNS page", async ({ page }) => {
    await page.getByRole("link", { name: "DNS" }).click();
    await expect(page).toHaveURL(/\/dns/);
    await expect(page.getByText("DNS Queries", { exact: true })).toBeVisible();
  });

  test("navigates to Alerts page", async ({ page }) => {
    await page.getByRole("link", { name: "Alerts" }).click();
    await expect(page).toHaveURL(/\/alerts/);
  });

  test("navigates to Replay page", async ({ page }) => {
    await page.getByRole("link", { name: "Replay" }).click();
    await expect(page).toHaveURL(/\/replay/);
  });

  test("navigates to Graph page", async ({ page }) => {
    await page.getByRole("link", { name: "Graph" }).click();
    await expect(page).toHaveURL(/\/graph/);
  });

  test("navigates to Gen page", async ({ page }) => {
    await page.getByRole("link", { name: "Gen" }).click();
    await expect(page).toHaveURL(/\/gen/);
  });

  test("navigates through all pages", async ({ page }) => {
    const pages = ["Rules", "Certs", "Devices", "DNS", "Alerts", "Replay", "Graph", "Gen", "Traffic"];
    for (const name of pages) {
      await page.getByRole("link", { name }).click();
    }
    await expect(page).toHaveURL(/\/?$/);
  });
});
