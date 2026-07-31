import { test, expect } from "@playwright/test";
import {
  emitTauriEvent,
  mockTauriIPC,
  mockTauriCommands,
} from "./fixtures/tauri-mock";
import {
  capturedRequest,
  type InterceptedRequest,
} from "./fixtures/desktop-contract";

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
};

const MOCK_REQUESTS = [
  capturedRequest({
    id: "1",
    method: "GET",
    host: "api.weixin.qq.com",
    path: "/cgi-bin/micromsg-bin/getcontact",
    status: 200,
    latency_ms: 42,
    app_name: "WeChat",
    req_headers: [["authorization", "Bearer token123"], ["content-type", "application/json"]],
    req_body: '{"contacts":[]}',
    resp_size: 128,
  }),
  capturedRequest({
    id: "2",
    method: "POST",
    host: "api.douyin.com",
    path: "/aweme/v1/feed/",
    status: 200,
    latency_ms: 156,
    app_name: "Douyin",
    req_headers: [["content-type", "application/json"]],
    req_body: '{"items":[]}',
    resp_size: 2048,
  }),
  capturedRequest({
    id: "3",
    method: "GET",
    host: "example.com",
    path: "/api/data",
    status: 404,
    latency_ms: 12,
    resp_size: 64,
  }),
];

/** Inject requests into the page via Tauri event callbacks */
async function injectRequests(page: import("@playwright/test").Page, requests: InterceptedRequest[]) {
  for (const req of requests) {
    await emitTauriEvent(page, "intercepted-request", req);
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
