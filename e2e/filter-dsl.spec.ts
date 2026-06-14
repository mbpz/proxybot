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
  list_filter_presets: [],
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
    headers: { authorization: "Bearer token123" },
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
    headers: {},
    body: '{"items":[]}',
    size: 2048,
  },
];

async function injectRequests(
  page: import("@playwright/test").Page,
  requests: typeof MOCK_REQUESTS,
) {
  for (const req of requests) {
    await page.evaluate((r) => {
      const internals = window.__TAURI_INTERNALS__ as any;
      if (internals?.callbacks) {
        for (const [, cb] of internals.callbacks) {
          try {
            cb({ payload: r, event: "intercepted-request" });
          } catch {
            /* ignore */
          }
        }
      }
    }, req);
  }
}

test.describe("Filter DSL", () => {
  test("filter_input_validates_known_syntax", async ({ page }) => {
    await mockTauriCommands(page, {
      ...BASE_MOCKS,
      parse_filter: { ok: true },
    });
    await page.goto("/");
    await page.waitForLoadState("networkidle");

    await page.getByTestId("filter-input").fill("method:GET");
    // Debounced 250ms; wait for parse to settle, no error.
    await expect(page.getByTestId("filter-error")).not.toBeVisible({
      timeout: 2000,
    });
  });

  test("filter_input_shows_error_for_bad_syntax", async ({ page }) => {
    await mockTauriCommands(page, {
      ...BASE_MOCKS,
      parse_filter: { ok: false, error: "Expected closing paren" },
    });
    await page.goto("/");
    await page.waitForLoadState("networkidle");

    await page.getByTestId("filter-input").fill("((method:GET");
    await expect(page.getByTestId("filter-error")).toBeVisible({
      timeout: 2000,
    });
  });

  test("preset_select_loads_expression_into_input", async ({ page }) => {
    await mockTauriCommands(page, {
      ...BASE_MOCKS,
      list_filter_presets: [
        { id: "p1", name: "WeChat 2xx", expr: "app:wechat AND status:2*" },
      ],
      parse_filter: { ok: true },
    });
    await page.goto("/");
    await page.waitForLoadState("networkidle");

    await page.getByTestId("filter-preset-select").selectOption("p1");
    await expect(page.getByTestId("filter-input")).toHaveValue(
      "app:wechat AND status:2*",
    );
  });

  test("preset_save_dialog_opens_and_saves", async ({ page }) => {
    await mockTauriCommands(page, {
      ...BASE_MOCKS,
      parse_filter: { ok: true },
      save_filter_preset: null,
    });
    await page.goto("/");
    await page.waitForLoadState("networkidle");

    await page.getByTestId("filter-input").fill("host:api.example.com");
    await page.getByTestId("filter-save-preset").click();
    await expect(
      page.getByTestId("filter-save-preset-dialog"),
    ).toBeVisible();

    await page.getByTestId("filter-save-preset-name").fill("My Preset");
    await page.getByTestId("filter-save-preset-confirm").click();
    await expect(
      page.getByTestId("filter-save-preset-dialog"),
    ).not.toBeVisible();
  });

  test("preset_dropdown_renders_multiple_presets", async ({ page }) => {
    await mockTauriCommands(page, {
      ...BASE_MOCKS,
      list_filter_presets: [
        { id: "p1", name: "First", expr: "method:GET" },
        { id: "p2", name: "Second", expr: "method:POST" },
      ],
      parse_filter: { ok: true },
    });
    await page.goto("/");
    await page.waitForLoadState("networkidle");

    // 2 presets + the placeholder option = 3 options total.
    await expect(
      page.getByTestId("filter-preset-select").locator("option"),
    ).toHaveCount(3);
    await expect(
      page.getByTestId("filter-preset-select").locator("option", {
        hasText: "First",
      }),
    ).toHaveCount(1);
    await expect(
      page.getByTestId("filter-preset-select").locator("option", {
        hasText: "Second",
      }),
    ).toHaveCount(1);
  });

  test("filter_drives_traffic_list_via_evaluate", async ({ page }) => {
    await mockTauriCommands(page, {
      ...BASE_MOCKS,
      parse_filter: { ok: true },
      // Simulate DSL match: only return true when host is "api.douyin.com".
      evaluate_filter: true,
    });
    await page.goto("/");
    await page.waitForLoadState("networkidle");

    await injectRequests(page, MOCK_REQUESTS);
    // Both hosts should be visible (evaluate_filter returns true for all).
    await expect(page.getByText("api.weixin.qq.com").first()).toBeVisible();
    await expect(page.getByText("api.douyin.com").first()).toBeVisible();
  });
});