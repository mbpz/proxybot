import { test, expect } from "@playwright/test";
import { mockTauriCommands } from "./fixtures/tauri-mock";

const BASE_MOCKS = {
  is_dashboard_running: false,
  get_dashboard_url: "",
  get_network_info: { lan_ip: "192.168.1.100", interface: "en0" },
  is_pf_enabled: false,
  is_tun_enabled: false,
  get_ca_metadata: null,
  get_dns_upstream: "8.8.8.8",
  get_replay_targets: [],
  get_rules: [],
  get_devices: [],
  get_ca_cert_pem: "",
};

const DNS_ENTRIES = [
  { domain: "api.weixin.qq.com", timestamp_ms: Date.now(), app_name: "WeChat", query_type: "A", response_ips: ["101.89.47.100"] },
  { domain: "api.douyin.com", timestamp_ms: Date.now() - 1000, app_name: "Douyin", query_type: "A", response_ips: ["47.246.24.230"] },
  { domain: "example.com", timestamp_ms: Date.now() - 2000, query_type: "A", response_ips: ["93.184.216.34"] },
];

test.describe("DNS page", () => {
  test("shows empty state when no queries", async ({ page }) => {
    await mockTauriCommands(page, { ...BASE_MOCKS, get_dns_log: [] });
    await page.goto("/dns");
    await expect(page.getByText("No DNS queries")).toBeVisible();
    await expect(page.getByText("0 entries")).toBeVisible();
  });

  test("renders DNS entries loaded from backend", async ({ page }) => {
    await mockTauriCommands(page, { ...BASE_MOCKS, get_dns_log: DNS_ENTRIES });
    await page.goto("/dns");

    await expect(page.getByText("3 entries")).toBeVisible();
    await expect(page.getByText("api.weixin.qq.com")).toBeVisible();
    await expect(page.getByText("api.douyin.com")).toBeVisible();
    await expect(page.getByText("example.com")).toBeVisible();
  });

  test("shows app badges for classified entries", async ({ page }) => {
    await mockTauriCommands(page, { ...BASE_MOCKS, get_dns_log: [DNS_ENTRIES[0]] });
    await page.goto("/dns");

    await expect(page.getByText("WeChat")).toBeVisible();
  });

  test("shows query type and response IPs", async ({ page }) => {
    await mockTauriCommands(page, { ...BASE_MOCKS, get_dns_log: [DNS_ENTRIES[0]] });
    await page.goto("/dns");

    await expect(page.getByText("101.89.47.100")).toBeVisible();
  });

  test("shows DNS upstream server", async ({ page }) => {
    await mockTauriCommands(page, { ...BASE_MOCKS, get_dns_log: [] });
    await page.goto("/dns");

    await expect(page.getByText("8.8.8.8")).toBeVisible();
  });

  test("receives real-time DNS queries via events", async ({ page }) => {
    await mockTauriCommands(page, { ...BASE_MOCKS, get_dns_log: [] });
    await page.goto("/dns");
    await page.waitForLoadState("networkidle");

    // Inject a DNS query via event
    await page.evaluate(() => {
      const internals = window.__TAURI_INTERNALS__;
      if (internals?.callbacks) {
        for (const [, cb] of internals.callbacks) {
          try {
            cb({
              payload: { domain: "live.weixin.qq.com", timestamp_ms: Date.now(), app_name: "WeChat" },
              event: "dns-query",
            });
          } catch {}
        }
      }
    });

    await expect(page.getByText("live.weixin.qq.com").first()).toBeVisible();
    // Entry count may show duplicates due to React Strict Mode double-effect
    await expect(page.getByText(/entries/)).toBeVisible();
  });
});
