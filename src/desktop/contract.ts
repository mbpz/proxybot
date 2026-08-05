import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  desktopCommandNames,
  desktopEventNames,
  unitCommandNames,
  type DesktopCommands,
  type DesktopEvents,
  type Alert,
  type DeviceOnboarding,
  type InterceptedRequest,
  type JsonValue,
  type Rule,
  type RuleAction,
  type TrafficPage,
  type WsFrame,
  type WsFrameEvent,
} from "../generated/desktop-contract";

export type DesktopErrorKind = "transport" | "contract" | "command";

export class DesktopError extends Error {
  constructor(
    readonly kind: DesktopErrorKind,
    readonly operation: string,
    readonly code: string,
    message: string,
    readonly cause?: unknown,
  ) {
    super(message);
    this.name = "DesktopError";
  }
}

export interface DesktopObserver<T> {
  next(payload: T): void;
  error?(error: DesktopError): void;
}

export interface DesktopSubscription {
  readonly ready: Promise<void>;
  dispose(): void;
}

export interface DesktopContract {
  call<K extends keyof DesktopCommands>(
    command: K,
    args: DesktopCommands[K]["args"],
  ): Promise<DesktopCommands[K]["result"]>;

  subscribe<K extends keyof DesktopEvents>(
    event: K,
    observer: DesktopObserver<DesktopEvents[K]>,
  ): DesktopSubscription;
}

/** Internal dependency Seam implemented by Tauri and the strict browser mock. */
export interface DesktopAdapter {
  callRaw(command: string, args: unknown): Promise<unknown>;
  subscribeRaw(event: string, receive: (payload: unknown) => void): Promise<() => void | Promise<void>>;
}

const commandNameSet = new Set<string>(desktopCommandNames);
const eventNameSet = new Set<string>(desktopEventNames);
const unitCommandSet = new Set<string>(unitCommandNames);

const tauriAdapter: DesktopAdapter = {
  callRaw: (command, args) => invoke(command, args as Record<string, unknown>),
  subscribeRaw: async (event, receive) => {
    const unlisten = await listen(event, ({ payload }) => receive(payload));
    return unlisten;
  },
};

export function createDesktopContract(adapter: DesktopAdapter): DesktopContract {
  return {
    async call(command, args) {
      if (!commandNameSet.has(command)) {
        throw contractError(command, "unknown_command", `Unknown desktop command: ${command}`);
      }

      let result: unknown;
      try {
        result = await adapter.callRaw(command, args);
      } catch (cause) {
        if (cause instanceof DesktopError) throw cause;
        throw new DesktopError(
          "command",
          command,
          "legacy",
          messageFrom(cause, `Desktop command failed: ${command}`),
          cause,
        );
      }

      if (unitCommandSet.has(command)) {
        if (result !== null && result !== undefined) {
          throw contractError(command, "invalid_result", "Expected a unit command result");
        }
        return undefined as DesktopCommands[typeof command]["result"];
      }

      validateCommandResult(command, result);
      return result as DesktopCommands[typeof command]["result"];
    },

    subscribe(event, observer) {
      if (!eventNameSet.has(event)) {
        const error = contractError(event, "unknown_event", `Unknown desktop event: ${event}`);
        return { ready: Promise.reject(error), dispose() {} };
      }

      let active = true;
      let stop: (() => void | Promise<void>) | undefined;

      const ready = adapter
        .subscribeRaw(event, (payload) => {
          if (!active) return;
          try {
            validateEventPayload(event, payload);
            observer.next(payload as DesktopEvents[typeof event]);
          } catch (cause) {
            active = false;
            const error =
              cause instanceof DesktopError
                ? cause
                : contractError(event, "invalid_payload", messageFrom(cause, "Invalid event payload"));
            observer.error?.(error);
            void stop?.();
          }
        })
        .then((unlisten) => {
          if (!active) {
            return Promise.resolve(unlisten()).then(() => undefined);
          }
          stop = unlisten;
        })
        .catch((cause) => {
          if (cause instanceof DesktopError) throw cause;
          throw new DesktopError(
            "transport",
            event,
            "listen_failed",
            messageFrom(cause, `Could not subscribe to desktop event: ${event}`),
            cause,
          );
        });

      return {
        ready,
        dispose() {
          if (!active) return;
          active = false;
          void stop?.();
        },
      };
    },
  };
}

function validateCommandResult(command: keyof DesktopCommands, value: unknown): void {
  switch (command) {
    case "acknowledge_alert":
      assertAlert(value, command);
      return;
    case "export_har":
      assertJsonValue(value, command);
      return;
    case "get_traffic_page":
      assertTrafficPage(value, command);
      return;
    case "get_rules":
      assertArray(value, command, assertRule);
      return;
    case "get_alert_count":
      assertNumber(value, command);
      return;
    case "get_alerts":
      assertArray(value, command, assertAlert);
      return;
    case "get_dashboard_url":
      assertString(value, command);
      return;
    case "get_db_stats":
      assertDbStats(value, command);
      return;
    case "get_keep_running":
    case "is_dashboard_running":
      assert(typeof value === "boolean", command, "must be a boolean");
      return;
    case "get_proxy_status":
      assert(typeof value === "boolean", command, "must be a boolean");
      return;
    case "prepare_device_onboarding":
      assertDeviceOnboarding(value, command);
      return;
    case "get_ws_frames":
      assertArray(value, command, assertWsFrame);
      return;
    case "list_filter_presets":
      assertArray(value, command, (preset, path) => {
        assertRecord(preset, path);
        assertString(preset.id, `${path}.id`);
        assertString(preset.name, `${path}.name`);
        assertString(preset.expr, `${path}.expr`);
      });
      return;
    case "list_rule_files":
      assertArray(value, command, assertString);
      return;
    case "load_history":
      assertArray(value, command, assertInterceptedRequest);
      return;
    case "match_host":
      if (value !== null) assertRuleAction(value, command);
      return;
    case "parse_filter":
      assertRecord(value, command);
      assert(typeof value.ok === "boolean", `${command}.ok`, "must be a boolean");
      assertNullableString(value.error, `${command}.error`);
      return;
    case "save_filter_preset":
      return;
    case "save_har_file":
      assertString(value, command);
      return;
    case "save_history":
      return;
    case "start_proxy":
    case "stop_proxy":
    case "start_dashboard":
    case "stop_dashboard":
      assertString(value, command);
      return;
  }
}

function assertDbStats(value: unknown, path: string): void {
  assertRecord(value, path);
  assertNumber(value.http_requests_count, `${path}.http_requests_count`);
  assertNumber(value.dns_queries_count, `${path}.dns_queries_count`);
  assertNumber(value.devices_count, `${path}.devices_count`);
  assertNumber(value.app_tags_count, `${path}.app_tags_count`);
}

function assertDeviceOnboarding(
  value: unknown,
  path: string,
): asserts value is DeviceOnboarding {
  assertRecord(value, path);
  assert(
    value.platform === "ios" || value.platform === "android",
    `${path}.platform`,
    "must be ios or android",
  );
  assertString(value.interface, `${path}.interface`);
  assertString(value.lan_ip, `${path}.lan_ip`);
  assertNumber(value.proxy_port, `${path}.proxy_port`);
  assertString(value.server_url, `${path}.server_url`);
  assertString(value.setup_url, `${path}.setup_url`);
  assertString(value.ca_url, `${path}.ca_url`);
  assertString(value.qr_svg, `${path}.qr_svg`);
}

function assertAlert(value: unknown, path: string): asserts value is Alert {
  assertRecord(value, path);
  assertNumber(value.id, `${path}.id`);
  assertNullableNumber(value.device_id, `${path}.device_id`);
  assert(
    value.severity === "Info" || value.severity === "Warning" || value.severity === "Critical",
    `${path}.severity`,
    "must be a known Alert Severity",
  );
  assert(
    value.alert_type === "NewDomain" ||
      value.alert_type === "NewIp" ||
      value.alert_type === "PrivacyExfil" ||
      value.alert_type === "AuthAnomaly" ||
      value.alert_type === "UntrustedCert",
    `${path}.alert_type`,
    "must be a known Alert Type",
  );
  assertString(value.details, `${path}.details`);
  assertString(value.created_at, `${path}.created_at`);
  assert(typeof value.acknowledged === "boolean", `${path}.acknowledged`, "must be a boolean");
}

function assertRule(value: unknown, path: string): asserts value is Rule {
  assertRecord(value, path);
  assert(
    ["DOMAIN", "DOMAIN-SUFFIX", "DOMAIN-KEYWORD", "IP-CIDR", "GEOIP", "RULE-SET"].includes(
      value.pattern as string,
    ),
    `${path}.pattern`,
    "must be a known rule pattern",
  );
  assertString(value.value, `${path}.value`);
  assertRuleAction(value.action, `${path}.action`);
  assertString(value.name, `${path}.name`);
  assertNumber(value.priority, `${path}.priority`);
  assert(
    Number.isInteger(value.priority) && value.priority >= 0 && value.priority <= 255,
    `${path}.priority`,
    "must be an integer from 0 to 255",
  );
  assert(typeof value.enabled === "boolean", `${path}.enabled`, "must be a boolean");
  assertString(value.comment, `${path}.comment`);
}

function assertRuleAction(value: unknown, path: string): asserts value is RuleAction {
  assertRecord(value, path);
  assertString(value.type, `${path}.type`);
  switch (value.type) {
    case "DIRECT":
    case "PROXY":
    case "REJECT":
      return;
    case "MAPREMOTE":
    case "MAPLOCAL":
      assertString(value.target, `${path}.target`);
      return;
    case "BREAKPOINT":
      assert(
        value.target === "REQUEST" || value.target === "RESPONSE" || value.target === "BOTH",
        `${path}.target`,
        "must be REQUEST, RESPONSE or BOTH",
      );
      return;
    default:
      assert(false, `${path}.type`, "must be a known rule action");
  }
}

function validateEventPayload(event: keyof DesktopEvents, value: unknown): void {
  switch (event) {
    case "capture-session:changed":
      assert(typeof value === "boolean", event, "must be a boolean");
      return;
    case "intercepted-request":
      assertInterceptedRequest(value, event);
      return;
    case "ws-frame:new":
      assertWsFrameEvent(value, event);
      return;
  }
}

function assertInterceptedRequest(value: unknown, path: string): asserts value is InterceptedRequest {
  assertRecord(value, path);
  assertString(value.id, `${path}.id`);
  assertString(value.timestamp, `${path}.timestamp`);
  assertString(value.method, `${path}.method`);
  assertString(value.host, `${path}.host`);
  assertString(value.path, `${path}.path`);
  assertNullableString(value.query_params, `${path}.query_params`);
  assertNullableNumber(value.status, `${path}.status`);
  assertNullableNumber(value.latency_ms, `${path}.latency_ms`);
  assertString(value.scheme, `${path}.scheme`);
  assertHeaderPairs(value.req_headers, `${path}.req_headers`);
  assertNullableString(value.req_body, `${path}.req_body`);
  assertHeaderPairs(value.resp_headers, `${path}.resp_headers`);
  assertNullableString(value.resp_body, `${path}.resp_body`);
  assertNullableNumber(value.resp_size, `${path}.resp_size`);
  assertNullableString(value.app_name, `${path}.app_name`);
  assertNullableString(value.app_icon, `${path}.app_icon`);
  assertNullableNumber(value.device_id, `${path}.device_id`);
  assertNullableString(value.device_name, `${path}.device_name`);
  assertNullableString(value.client_ip, `${path}.client_ip`);
  assertNullableString(value.upstream_ip, `${path}.upstream_ip`);
  assert(typeof value.is_websocket === "boolean", path, "is_websocket must be a boolean");
  if (value.ws_frames !== null) assertArray(value.ws_frames, `${path}.ws_frames`, assertWsFrame);
  assertNullableString(value.grpc_decoded, `${path}.grpc_decoded`);
  assertNullableString(value.graphql_op, `${path}.graphql_op`);
}

function assertWsFrame(value: unknown, path: string): asserts value is WsFrame {
  assertRecord(value, path);
  assertString(value.direction, `${path}.direction`);
  assertString(value.timestamp, `${path}.timestamp`);
  assertString(value.payload, `${path}.payload`);
  assertNumber(value.size, `${path}.size`);
  assertNumber(value.opcode, `${path}.opcode`);
  assert(typeof value.truncated === "boolean", path, "truncated must be a boolean");
}

function assertWsFrameEvent(value: unknown, path: string): asserts value is WsFrameEvent {
  assertRecord(value, path);
  assertString(value.request_id, `${path}.request_id`);
  assertWsFrame(value.frame, `${path}.frame`);
}

function assertTrafficPage(value: unknown, path: string): asserts value is TrafficPage {
  assertRecord(value, path);
  assertArray(value.records, `${path}.records`, assertInterceptedRequest);
  assertArray(value.normalized_records, `${path}.normalized_records`, assertNormalizedRecord);
  assertNumber(value.total, `${path}.total`);
  assertNumber(value.page, `${path}.page`);
  assertNumber(value.page_size, `${path}.page_size`);
  assert(typeof value.has_more === "boolean", path, "has_more must be a boolean");
}

function assertNormalizedRecord(value: unknown, path: string): void {
  assertRecord(value, path);
  assertNumber(value.id, `${path}.id`);
  assertString(value.timestamp, `${path}.timestamp`);
  assertString(value.method, `${path}.method`);
  assertString(value.path, `${path}.path`);
  assertJsonValue(value.query, `${path}.query`);
  assertJsonValue(value.request_headers, `${path}.request_headers`);
  assertJsonValue(value.request_body, `${path}.request_body`);
  assertNumber(value.response_status, `${path}.response_status`);
  assertJsonValue(value.response_headers, `${path}.response_headers`);
  assertJsonValue(value.response_body, `${path}.response_body`);
  assertNumber(value.timing_ms, `${path}.timing_ms`);
  assertNullableNumber(value.device_id, `${path}.device_id`);
}

function assertJsonValue(value: unknown, path: string): asserts value is JsonValue {
  if (value === null || typeof value === "boolean" || typeof value === "number" || typeof value === "string") {
    return;
  }
  if (Array.isArray(value)) {
    value.forEach((entry, index) => assertJsonValue(entry, `${path}[${index}]`));
    return;
  }
  assertRecord(value, path);
  Object.entries(value).forEach(([key, entry]) => assertJsonValue(entry, `${path}.${key}`));
}

function assertHeaderPairs(value: unknown, path: string): void {
  assertArray(value, path, (pair, pairPath) => {
    assert(Array.isArray(pair) && pair.length === 2, pairPath, "header must be a [name, value] pair");
    assertString(pair[0], `${pairPath}[0]`);
    assertString(pair[1], `${pairPath}[1]`);
  });
}

function assertArray(
  value: unknown,
  path: string,
  validate: (entry: unknown, path: string) => void,
): asserts value is unknown[] {
  assert(Array.isArray(value), path, "must be an array");
  value.forEach((entry, index) => validate(entry, `${path}[${index}]`));
}

function assertRecord(value: unknown, path: string): asserts value is Record<string, unknown> {
  assert(typeof value === "object" && value !== null && !Array.isArray(value), path, "must be an object");
}

function assertString(value: unknown, path: string): asserts value is string {
  assert(typeof value === "string", path, "must be a string");
}

function assertNumber(value: unknown, path: string): asserts value is number {
  assert(typeof value === "number" && Number.isFinite(value), path, "must be a finite number");
}

function assertNullableString(value: unknown, path: string): void {
  if (value !== null) assertString(value, path);
}

function assertNullableNumber(value: unknown, path: string): void {
  if (value !== null) assertNumber(value, path);
}

function assert(condition: boolean, operation: string, message: string): asserts condition {
  if (!condition) throw contractError(operation, "contract_violation", `${operation}: ${message}`);
}

function contractError(operation: string, code: string, message: string): DesktopError {
  return new DesktopError("contract", operation, code, message);
}

function messageFrom(cause: unknown, fallback: string): string {
  if (cause instanceof Error) return cause.message;
  if (typeof cause === "string") return cause;
  return fallback;
}

export const desktop = createDesktopContract(tauriAdapter);
