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
};

test.describe("App shell", () => {
  test("renders sidebar with all navigation items", async ({ page }) => {
    await mockTauriCommands(page, BASE_MOCKS);
    await page.goto("/");

    const expectedLinks = ["Traffic", "Rules", "Certs", "Devices", "DNS", "Alerts", "Replay", "Graph", "Gen"];
    for (const name of expectedLinks) {
      await expect(page.getByRole("link", { name })).toBeVisible();
    }
  });

  test("sidebar shows ProxyBot title", async ({ page }) => {
    await mockTauriCommands(page, BASE_MOCKS);
    await page.goto("/");
    await expect(page.getByText("ProxyBot")).toBeVisible();
  });

  test("sidebar can be collapsed", async ({ page }) => {
    await mockTauriCommands(page, BASE_MOCKS);
    await page.goto("/");

    // Click the collapse button (X icon when expanded)
    await page.locator("aside button").click();

    // After collapse, sidebar should have w-16 class
    await expect(page.locator("aside")).toHaveClass(/w-16/);
  });

  test("traffic page loads with filter bar", async ({ page }) => {
    await mockTauriCommands(page, BASE_MOCKS);
    await page.goto("/");

    // Filter bar should be visible with method/app selectors
    await expect(page.locator("select").first()).toBeVisible();
    await expect(page.getByPlaceholder("host:*.example.com")).toBeVisible();
    await expect(page.getByPlaceholder("Search path...")).toBeVisible();
  });

  test("DNS page loads from direct URL", async ({ page }) => {
    await mockTauriCommands(page, { ...BASE_MOCKS, get_dns_log: [] });
    await page.goto("/dns");

    await expect(page.getByText("DNS Queries", { exact: true })).toBeVisible();
    await expect(page.getByText("No DNS queries")).toBeVisible();
  });

  test("Rules page loads from direct URL", async ({ page }) => {
    await mockTauriCommands(page, { ...BASE_MOCKS, get_rules: [] });
    await page.goto("/rules");

    await expect(page.locator("aside")).toBeVisible();
  });

  test("Devices page loads from direct URL", async ({ page }) => {
    await mockTauriCommands(page, { ...BASE_MOCKS, get_devices: [] });
    await page.goto("/devices");

    await expect(page.locator("aside")).toBeVisible();
  });
});
