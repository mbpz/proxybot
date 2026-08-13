import { describe, expect, it, vi } from "vitest";
import { DesktopError } from "./contract";
import { BrowserMockAdapter } from "./testing";
import type {
  Alert,
  DbStats,
  DeviceOnboarding,
  DnsObservation,
  InterceptedRequest,
  Rule,
  TrafficBaseline,
  WsFrame,
} from "../generated/desktop-contract";

const frame: WsFrame = {
  direction: "incoming",
  timestamp: "2026-07-31T12:00:00Z",
  payload: "hello",
  size: 5,
  opcode: 1,
  truncated: false,
};

const request: InterceptedRequest = {
  id: "req-1",
  timestamp: "2026-07-31T12:00:00Z",
  method: "GET",
  host: "example.com",
  path: "/socket",
  query_params: null,
  status: 101,
  latency_ms: 4,
  scheme: "https",
  req_headers: [["upgrade", "websocket"]],
  req_body: null,
  resp_headers: [],
  resp_body: null,
  resp_size: 5,
  app_name: null,
  app_icon: null,
  device_id: null,
  device_name: null,
  client_ip: null,
  upstream_ip: null,
  is_websocket: true,
  ws_frames: [frame],
  grpc_decoded: null,
  graphql_op: null,
};

const rule: Rule = {
  pattern: "DOMAIN-SUFFIX",
  value: "example.com",
  action: { type: "MAPREMOTE", target: "https://mock.local" },
  name: "mock",
  priority: 10,
  enabled: true,
  comment: "",
};

const alert: Alert = {
  id: 7,
  device_id: null,
  severity: "Warning",
  alert_type: "AuthAnomaly",
  details: "Resource accessed before authentication",
  created_at: "2026-08-04 12:00:00",
  acknowledged: false,
};

const baseline: TrafficBaseline = {
  device_id: null,
  domains: [
    {
      value: "api.example.com",
      count: 3,
      first_seen: "2026-08-01 12:00:00",
      last_seen: "2026-08-04 12:00:00",
    },
  ],
  ips: [],
};

const onboarding: DeviceOnboarding = {
  platform: "ios",
  interface: "en0",
  lan_ip: "192.168.1.40",
  proxy_port: 8088,
  server_url: "http://192.168.1.40:19876",
  setup_url: "http://192.168.1.40:19876/ca.crt",
  ca_url: "http://192.168.1.40:19876/ca.crt",
  qr_svg: "<svg />",
};

describe("Desktop contract Adapter conformance", () => {
  it("types, validates and records command calls", async () => {
    const adapter = new BrowserMockAdapter({
      get_ws_frames: ({ requestId }) => (requestId === request.id ? [frame] : []),
    });

    await expect(adapter.contract.call("get_ws_frames", { requestId: request.id })).resolves.toEqual([frame]);
    expect(adapter.calls).toEqual([
      { command: "get_ws_frames", args: { requestId: request.id } },
    ]);
  });

  it("rejects unhandled commands and invalid results instead of returning null", async () => {
    const strict = new BrowserMockAdapter();
    await expect(strict.contract.call("load_history", {})).rejects.toMatchObject({
      kind: "contract",
      code: "unhandled_mock_command",
    });

    const invalid = new BrowserMockAdapter({ get_ws_frames: () => [request] as unknown as WsFrame[] });
    await expect(invalid.contract.call("get_ws_frames", { requestId: request.id })).rejects.toMatchObject({
      kind: "contract",
    });
  });

  it("validates tagged Rule actions and unit mutation results", async () => {
    const adapter = new BrowserMockAdapter({
      get_rules: ({ filename }) => (filename === "custom.yaml" ? [rule] : []),
      save_rule: () => undefined,
    });

    await expect(adapter.contract.call("get_rules", { filename: "custom.yaml" })).resolves.toEqual([rule]);
    await expect(adapter.contract.call("save_rule", {
      rule,
      filename: "custom.yaml",
      originalRule: null,
    })).resolves.toBeUndefined();

    const invalid = new BrowserMockAdapter({
      get_rules: () => [{ ...rule, action: "DIRECT" } as unknown as Rule],
    });
    await expect(invalid.contract.call("get_rules", { filename: "custom.yaml" })).rejects.toMatchObject({
      kind: "contract",
    });
  });

  it("validates Alert reads, baseline and anomaly scans through the same contract", async () => {
    const adapter = new BrowserMockAdapter({
      get_alerts: () => [alert],
      get_alert_count: () => 1,
      acknowledge_alert: ({ alertId }) => ({ ...alert, id: alertId, acknowledged: true }),
      get_traffic_baseline: () => baseline,
      scan_request_anomalies: () => ({
        new_domains: ["api.example.com"],
        new_ips: [],
        privacy_findings: [],
        alerts_generated: 1,
      }),
    });

    await expect(adapter.contract.call("get_alerts", {
      deviceId: null,
      severity: null,
      since: null,
      acknowledged: null,
      limit: 100,
    })).resolves.toEqual([alert]);
    await expect(adapter.contract.call("get_alert_count", {})).resolves.toBe(1);
    await expect(adapter.contract.call("acknowledge_alert", { alertId: alert.id }))
      .resolves.toMatchObject({ id: alert.id, acknowledged: true });
    await expect(adapter.contract.call("get_traffic_baseline", { deviceId: null }))
      .resolves.toEqual(baseline);
    await expect(adapter.contract.call("scan_request_anomalies", {
      deviceId: null,
      host: "api.example.com",
      ip: null,
      reqBody: null,
      respBody: null,
    })).resolves.toMatchObject({ alerts_generated: 1 });

    const invalid = new BrowserMockAdapter({
      get_alerts: () => [{ ...alert, severity: "urgent" } as unknown as Alert],
    });
    await expect(invalid.contract.call("get_alerts", {
      deviceId: null,
      severity: null,
      since: null,
      acknowledged: null,
      limit: 100,
    })).rejects.toMatchObject({ kind: "contract", code: "contract_violation" });

    const invalidBaseline = new BrowserMockAdapter({
      get_traffic_baseline: () => ({ ...baseline, domains: [{ ...baseline.domains[0], count: "three" }] }) as never,
    });
    await expect(invalidBaseline.contract.call("get_traffic_baseline", { deviceId: null }))
      .rejects.toMatchObject({ kind: "contract", code: "contract_violation" });

    const invalidAnomaly = new BrowserMockAdapter({
      scan_request_anomalies: () => ({
        new_domains: [],
        new_ips: [],
        privacy_findings: [],
        alerts_generated: 1.5,
      }),
    });
    await expect(invalidAnomaly.contract.call("scan_request_anomalies", {
      deviceId: null,
      host: "api.example.com",
      ip: null,
      reqBody: null,
      respBody: null,
    })).rejects.toBeInstanceOf(DesktopError);
  });

  it("validates Device Onboarding preparation and cleanup", async () => {
    const adapter = new BrowserMockAdapter({
      prepare_device_onboarding: ({ platform }) => ({ ...onboarding, platform }),
      stop_device_onboarding: () => undefined,
    });

    await expect(
      adapter.contract.call("prepare_device_onboarding", { platform: "ios" }),
    ).resolves.toEqual(onboarding);
    await expect(adapter.contract.call("stop_device_onboarding", {})).resolves.toBeUndefined();

    const invalid = new BrowserMockAdapter({
      prepare_device_onboarding: () => ({ ...onboarding, proxy_port: "8088" }) as never,
    });
    await expect(
      invalid.contract.call("prepare_device_onboarding", { platform: "ios" }),
    ).rejects.toMatchObject({ kind: "contract", code: "contract_violation" });
  });

  it("validates General settings reads and mutations", async () => {
    const stats: DbStats = {
      http_requests_count: 12,
      dns_queries_count: 8,
      devices_count: 2,
      app_tags_count: 3,
    };
    const adapter = new BrowserMockAdapter({
      get_keep_running: () => true,
      is_dashboard_running: () => false,
      get_dashboard_url: () => "http://192.168.1.40:9090?token=test",
      get_db_stats: () => stats,
      set_keep_running: () => undefined,
      start_dashboard: () => "http://192.168.1.40:9090?token=test",
      stop_dashboard: () => "Dashboard stopped",
    });

    await expect(adapter.contract.call("get_keep_running", {})).resolves.toBe(true);
    await expect(adapter.contract.call("is_dashboard_running", {})).resolves.toBe(false);
    await expect(adapter.contract.call("get_db_stats", {})).resolves.toEqual(stats);
    await expect(adapter.contract.call("set_keep_running", { keep: false })).resolves.toBeUndefined();
    await expect(adapter.contract.call("start_dashboard", {})).resolves.toContain("token=test");
    await expect(adapter.contract.call("stop_dashboard", {})).resolves.toBe("Dashboard stopped");

    const invalid = new BrowserMockAdapter({
      get_db_stats: () => ({ ...stats, devices_count: "two" }) as never,
    });
    await expect(invalid.contract.call("get_db_stats", {})).rejects.toMatchObject({
      kind: "contract",
      code: "contract_violation",
    });
  });

  it("validates DNS reads, mutations and observations", async () => {
    const observation: DnsObservation = {
      domain: "api.example.com",
      timestamp_ms: 1_786_000_000_000,
      app_name: "Example",
      app_icon: null,
      action: "DIRECT",
      resolved_ips: ["203.0.113.10"],
      client_ip: "192.168.1.10",
    };
    const adapter = new BrowserMockAdapter({
      get_dns_log: () => [observation],
      get_dns_upstream: () => ({ upstream_type: "plainudp", address: "8.8.8.8:53" }),
      set_dns_upstream: () => undefined,
      reload_dns_lists: () => undefined,
    });

    await expect(adapter.contract.call("get_dns_log", {})).resolves.toEqual([observation]);
    await expect(adapter.contract.call("get_dns_upstream", {})).resolves.toEqual({
      upstream_type: "plainudp",
      address: "8.8.8.8:53",
    });
    await expect(
      adapter.contract.call("set_dns_upstream", {
        upstream: { upstream_type: "doh", address: "https://1.1.1.1/dns-query" },
      }),
    ).resolves.toBeUndefined();
    await expect(adapter.contract.call("reload_dns_lists", {})).resolves.toBeUndefined();

    const next = vi.fn();
    const subscription = adapter.contract.subscribe("dns-query", { next });
    await subscription.ready;
    adapter.emit("dns-query", observation);
    expect(next).toHaveBeenCalledWith(observation);

    const invalid = new BrowserMockAdapter({
      get_dns_upstream: () => ({ upstream_type: "udp", address: "8.8.8.8:53" }) as never,
    });
    await expect(invalid.contract.call("get_dns_upstream", {})).rejects.toMatchObject({
      kind: "contract",
      code: "contract_violation",
    });
  });

  it("validates the complete Captured Request query contract", async () => {
    const adapter = new BrowserMockAdapter({
      get_traffic_page: ({ query }) => ({
        records: query.expression === "method:GET" ? [request] : [],
        normalized_records: [],
        total: query.expression === "method:GET" ? 1 : 0,
        page: query.page,
        page_size: query.page_size,
        has_more: false,
      }),
      parse_filter: ({ expr }) => ({ ok: expr === "method:GET", error: null }),
      save_filter_preset: () => undefined,
    });
    const query = {
      expression: "method:GET",
      method: null,
      host: null,
      status: null,
      application: null,
      search: null,
      order: "newest" as const,
      page: 0,
      page_size: 50,
    };

    await expect(
      adapter.contract.call("get_traffic_page", { query, records: null }),
    ).resolves.toMatchObject({ records: [request], total: 1 });
    await expect(adapter.contract.call("parse_filter", { expr: "method:GET" })).resolves.toEqual({
      ok: true,
      error: null,
    });
    await expect(
      adapter.contract.call("save_filter_preset", {
        preset: { id: "one", name: "GET", expr: "method:GET" },
      }),
    ).resolves.toBeUndefined();

    const invalid = new BrowserMockAdapter({
      get_traffic_page: () =>
        ({ records: [], total: 0, page: 0, page_size: 50, has_more: false }) as never,
    });
    await expect(
      invalid.contract.call("get_traffic_page", { query, records: null }),
    ).rejects.toMatchObject({ kind: "contract" });
  });

  it("preserves event order and makes disposal idempotent", async () => {
    const adapter = new BrowserMockAdapter();
    const received: string[] = [];
    const subscription = adapter.contract.subscribe("ws-frame:new", {
      next: (event) => received.push(event.frame.payload),
    });
    await subscription.ready;

    adapter.emit("ws-frame:new", { request_id: request.id, frame: { ...frame, payload: "one" } });
    adapter.emit("ws-frame:new", { request_id: request.id, frame: { ...frame, payload: "two" } });
    subscription.dispose();
    subscription.dispose();
    adapter.emit("ws-frame:new", { request_id: request.id, frame: { ...frame, payload: "three" } });

    expect(received).toEqual(["one", "two"]);
  });

  it("can dispose before asynchronous listener registration completes", async () => {
    const adapter = new BrowserMockAdapter();
    const next = vi.fn();
    const subscription = adapter.contract.subscribe("ws-frame:new", { next });

    subscription.dispose();
    await subscription.ready;
    adapter.emit("ws-frame:new", { request_id: request.id, frame });

    expect(next).not.toHaveBeenCalled();
  });

  it("reports invalid event payloads through the observer", async () => {
    const adapter = new BrowserMockAdapter();
    const onError = vi.fn<(error: DesktopError) => void>();
    const subscription = adapter.contract.subscribe("intercepted-request", {
      next: vi.fn(),
      error: onError,
    });
    await subscription.ready;

    adapter.emit("intercepted-request", { ...request, req_headers: null } as unknown as InterceptedRequest);
    expect(onError).toHaveBeenCalledWith(expect.objectContaining({ kind: "contract" }));
  });
});
