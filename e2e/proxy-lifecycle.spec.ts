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
  start_proxy: "Proxy listening on 0.0.0.0:8088",
  stop_proxy: "Proxy stopped",
  prepare_device_onboarding: {
    platform: "ios",
    interface: "en0",
    lan_ip: "192.168.1.100",
    proxy_port: 8088,
    server_url: "http://192.168.1.100:19876",
    setup_url: "http://192.168.1.100:19876/ca.crt",
    ca_url: "http://192.168.1.100:19876/ca.crt",
    qr_svg: "<svg><rect width=\"10\" height=\"10\" /></svg>",
  },
  stop_device_onboarding: null,
};

test.describe("App shell", () => {
  test("starts and stops capture from the mounted shell", async ({ page }) => {
    await mockTauriCommands(page, BASE_MOCKS);
    await page.goto("/");

    await expect(page.getByText("Capture stopped")).toBeVisible();
    await page.getByRole("button", { name: "Start Capture" }).click();
    await expect(page.getByText("Capturing")).toBeVisible();
    await page.getByRole("button", { name: "Stop Capture" }).click();
    await expect(page.getByText("Capture stopped")).toBeVisible();
  });

  test("renders sidebar with all navigation items", async ({ page }) => {
    await mockTauriCommands(page, BASE_MOCKS);
    await page.goto("/");

    const expectedLinks = ["Traffic", "Setup", "Rules", "DNS", "Alerts", "Replay", "Graph", "Gen"];
    for (const name of expectedLinks) {
      await expect(page.locator("aside").getByRole("link", { name, exact: true })).toBeVisible();
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
    await expect(page.getByPlaceholder("*.example.com", { exact: true })).toBeVisible();
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

  test("prepares explicit-proxy device setup from one page", async ({ page }) => {
    await mockTauriCommands(page, BASE_MOCKS);
    await page.goto("/setup");

    await page.getByRole("button", { name: "Prepare iOS Setup" }).click();
    await expect(page.getByText("192.168.1.100")).toBeVisible();
    await expect(page.getByText("8088")).toBeVisible();
    await expect(page.getByText(/downloads only the ProxyBot CA certificate/)).toBeVisible();
    await expect(page.getByRole("button", { name: "Stop Setup Server" })).toBeVisible();
  });
});
