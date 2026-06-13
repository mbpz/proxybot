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
  // SSL Bypass mocks
  frida_list_devices: [],
  frida_list_processes: [],
  list_bypass_scripts: [
    {
      id: "okhttp3-pinner",
      name: "OkHttp3 CertificatePinner",
      description:
        "Bypasses OkHttp3 certificate pinning by hooking CertificatePinner.check",
      target_framework: ["android"],
      is_builtin: true,
    },
    {
      id: "conscrypt",
      name: "Conscrypt TrustManager",
      description: "Bypasses Conscrypt/Java TLS certificate verification",
      target_framework: ["android"],
      is_builtin: true,
    },
    {
      id: "webview-ssl",
      name: "WebView SSL Error",
      description: "Bypasses WebView SSL errors by calling handler.proceed()",
      target_framework: ["android"],
      is_builtin: true,
    },
  ],
  check_java_installed: false,
  check_adb_installed: false,
};

test.beforeEach(async ({ page }) => {
  await mockTauriCommands(page, BASE_MOCKS);
});

test("ssl_bypass_page_renders", async ({ page }) => {
  await page.goto("/ssl-bypass");
  await expect(page.getByRole("heading", { name: "SSL Bypass" })).toBeVisible();
  await expect(page.getByText(/Prerequisites/)).toBeVisible();
});

test("script_list_shows_builtin_scripts", async ({ page }) => {
  await page.goto("/ssl-bypass");
  // The script list is loaded asynchronously via useEffect → invoke.
  // Each script gets a data-testid="ssl-bypass-script-<id>" on its button.
  await expect(page.getByTestId("ssl-bypass-script-okhttp3-pinner")).toBeVisible();
  await expect(page.getByTestId("ssl-bypass-script-conscrypt")).toBeVisible();
  await expect(page.getByTestId("ssl-bypass-script-webview-ssl")).toBeVisible();
});

test("prerequisite_check_shows_status", async ({ page }) => {
  await page.goto("/ssl-bypass");
  // Java and ADB checks happen on mount
  await expect(page.getByText(/Java:/)).toBeVisible();
  await expect(page.getByText(/ADB:/)).toBeVisible();
});

test("device_selector_shows_empty_when_no_devices", async ({ page }) => {
  await page.goto("/ssl-bypass");
  // Click Refresh on the device card to trigger device list fetch
  await page.getByTestId("ssl-bypass-refresh-devices").click();
  // Without an actual device, the empty state should show
  await expect(page.getByText(/No devices found/)).toBeVisible();
});