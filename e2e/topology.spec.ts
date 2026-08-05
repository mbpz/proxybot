import { test, expect } from "@playwright/test";
import { mockTauriCommands } from "./fixtures/tauri-mock";

const MOCKS = {
  is_dashboard_running: false,
  get_dashboard_url: "",
  get_network_info: { lan_ip: "192.168.1.100", interface: "en0" },
  is_pf_enabled: false,
  is_tun_enabled: false,
  get_ca_metadata: null,
  get_dns_log: [],
  get_dns_upstream: { upstream_type: "plainudp", address: "8.8.8.8:53" },
  get_replay_targets: [],
  get_rules: [],
  get_devices: [{ id: 1, name: "iPhone" }],
  get_ca_cert_pem: "",
  list_rule_files: [],
  get_alerts: [],
  get_alert_count: 0,
  get_traffic_baseline: null,
  get_graph_data: { requests: [] },
  build_topology_graph: {
    nodes: [
      {
        id: "device:1",
        kind: "device",
        label: "iPhone",
        app_tag: null,
        device_id: "1",
        request_count: 5,
        total_bytes: 0,
        avg_latency_ms: 0,
        p95_latency_ms: 0,
        error_count: 0,
        error_rate: 0,
        last_seen: 0,
      },
      {
        id: "host:test.com",
        kind: "host",
        label: "test.com",
        app_tag: null,
        device_id: null,
        request_count: 5,
        total_bytes: 0,
        avg_latency_ms: 0,
        p95_latency_ms: 0,
        error_count: 0,
        error_rate: 0,
        last_seen: 0,
      },
    ],
    edges: [
      {
        id: "e1",
        from: "device:1",
        to: "host:test.com",
        request_count: 5,
        total_bytes: 0,
        avg_latency_ms: 0,
        error_rate: 0,
        is_anomalous: false,
      },
    ],
    meta: {
      total_requests: 5,
      total_bytes: 0,
      device_count: 1,
      app_count: 0,
      host_count: 1,
      time_range: [0, 0],
      built_at: 0,
    },
  },
};

test.describe("Topology page", () => {
  test.beforeEach(async ({ page }) => {
    await mockTauriCommands(page, MOCKS);
  });

  test("opens from the Capture context", async ({ page }) => {
    await page.goto("/");
    await page
      .getByRole("navigation", { name: "Capture views" })
      .getByRole("link", { name: "Topology" })
      .click();
    await expect(page).toHaveURL(/\/topology/);
    await expect(page.getByRole("heading", { name: "Topology" })).toBeVisible();
  });

  test("shows tab switcher with three views", async ({ page }) => {
    await page.goto("/topology");
    await expect(page.getByRole("button", { name: "Radial" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Layered" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Grouped" })).toBeVisible();
  });

  test("switches active tab on click", async ({ page }) => {
    await page.goto("/topology");
    await page.getByRole("button", { name: "Layered" }).click();
    await expect(page.getByRole("button", { name: "Layered" })).toHaveClass(/bg-accent-blue/);
  });

  test("shows empty state when no nodes", async ({ page }) => {
    await mockTauriCommands(page, {
      ...MOCKS,
      build_topology_graph: {
        nodes: [],
        edges: [],
        meta: {
          total_requests: 0,
          total_bytes: 0,
          device_count: 0,
          app_count: 0,
          host_count: 0,
          time_range: [0, 0],
          built_at: 0,
        },
      },
    });
    await page.goto("/topology");
    await expect(page.getByText(/No traffic data yet/)).toBeVisible();
  });
});
