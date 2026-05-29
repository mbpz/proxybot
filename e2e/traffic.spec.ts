import { test, expect } from "@playwright/test";
import { mockTauriIPC, mockTauriCommands } from "./fixtures/tauri-mock";

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

const MOCK_REQUESTS = [
  {
    id: "1",
    method: "GET",
    host: "api.weixin.qq.com",
    path: "/cgi-bin/micromsg-bin/getcontact",
    status: 200,
    duration_ms: 42,
    timestamp: Math.floor(Date.now() / 1000),
    app_tag: "WeChat",
    headers: { authorization: "Bearer token123", "content-type": "application/json" },
    body: '{"contacts":[]}',
    size: 128,
  },
  {
    id: "2",
    method: "POST",
    host: "api.douyin.com",
    path: "/aweme/v1/feed/",
    status: 200,
    duration_ms: 156,
    timestamp: Math.floor(Date.now() / 1000),
    app_tag: "Douyin",
    headers: { "content-type": "application/json" },
    body: '{"items":[]}',
    size: 2048,
  },
  {
    id: "3",
    method: "GET",
    host: "example.com",
    path: "/api/data",
    status: 404,
    duration_ms: 12,
    timestamp: Math.floor(Date.now() / 1000),
    headers: {},
    size: 64,
  },
];

/** Inject requests into the page via Tauri event callbacks */
async function injectRequests(page: import("@playwright/test").Page, requests: typeof MOCK_REQUESTS) {
  for (const req of requests) {
    await page.evaluate((r) => {
      const internals = window.__TAURI_INTERNALS__;
      if (internals?.callbacks) {
        for (const [, cb] of internals.callbacks) {
          try { cb({ payload: r, event: "intercepted-request" }); } catch {}
        }
      }
    }, req);
  }
}

test.describe("Traffic page", () => {
  test("shows empty state when no requests", async ({ page }) => {
    await mockTauriCommands(page, BASE_MOCKS);
    await page.goto("/");
    await expect(page.getByText("No requests captured yet")).toBeVisible();
  });

  test("renders intercepted requests via events", async ({ page }) => {
    await mockTauriCommands(page, BASE_MOCKS);
    await page.goto("/");
    await page.waitForLoadState("networkidle");

    await injectRequests(page, MOCK_REQUESTS);

    await expect(page.getByText("api.weixin.qq.com").first()).toBeVisible();
    await expect(page.getByText("api.douyin.com").first()).toBeVisible();
    await expect(page.getByText("example.com").first()).toBeVisible();
  });

  test("filter by method shows matching requests", async ({ page }) => {
    await mockTauriCommands(page, BASE_MOCKS);
    await page.goto("/");
    await page.waitForLoadState("networkidle");

    await injectRequests(page, MOCK_REQUESTS);

    // Filter by POST method — douyin (POST) should remain
    await page.locator("select").first().selectOption("POST");
    await expect(page.getByText("api.douyin.com").first()).toBeVisible();
    // After filter, the clear button should appear
    await expect(page.getByRole("button", { name: "Clear" })).toBeVisible();
  });

  test("filter by app tag shows matching requests", async ({ page }) => {
    await mockTauriCommands(page, BASE_MOCKS);
    await page.goto("/");
    await page.waitForLoadState("networkidle");

    await injectRequests(page, MOCK_REQUESTS);

    // Filter by WeChat
    await page.locator("select").nth(1).selectOption("WeChat");
    await expect(page.getByText("api.weixin.qq.com").first()).toBeVisible();
    await expect(page.getByRole("button", { name: "Clear" })).toBeVisible();
  });

  test("search filter shows matching requests", async ({ page }) => {
    await mockTauriCommands(page, BASE_MOCKS);
    await page.goto("/");
    await page.waitForLoadState("networkidle");

    await injectRequests(page, MOCK_REQUESTS);

    await page.getByPlaceholder("Search path...").fill("douyin");
    await expect(page.getByText("api.douyin.com").first()).toBeVisible();
    await expect(page.getByRole("button", { name: "Clear" })).toBeVisible();
  });

  test("clicking a request shows detail panel", async ({ page }) => {
    await mockTauriCommands(page, BASE_MOCKS);
    await page.goto("/");
    await page.waitForLoadState("networkidle");

    await injectRequests(page, [MOCK_REQUESTS[0]]);

    // Click on the request row
    await page.getByText("api.weixin.qq.com").first().click();

    // Detail panel should show status and duration
    await expect(page.getByText("Status:").first()).toBeVisible();
    await expect(page.getByText("Duration: 42ms")).toBeVisible();
  });
});
