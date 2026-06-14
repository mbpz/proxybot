import { test, expect, type Page } from "@playwright/test";
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
  get_alerts: [],
  get_alert_count: 0,
  get_traffic_baseline: null,
  get_graph_data: { requests: [] },
  get_replay_history: [],
  get_traffic_page: { records: [], total: 0, has_more: false },
};

/** A captured WebSocket upgrade — must have is_websocket: true so the
 *  WebSocket Frames tab is rendered in RequestDetail. */
const WS_REQUEST = {
  id: "ws-req-1",
  method: "GET",
  host: "echo.websocket.org",
  path: "/",
  status: 101,
  duration_ms: 33,
  timestamp: Math.floor(Date.now() / 1000),
  headers: {
    upgrade: "websocket",
    connection: "Upgrade",
    "sec-websocket-key": "dGhlIHNhbXBsZSBub25jZQ==",
    "sec-websocket-version": "13",
  },
  body: "",
  size: 0,
  is_websocket: true,
};

/** Inject a single captured request into the page via the Tauri event
 *  callback registered by the traffic page's listen("intercepted-request", …). */
async function injectRequest(page: Page, req: typeof WS_REQUEST) {
  await page.evaluate((r) => {
    const internals = window.__TAURI_INTERNALS__;
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

/** Inject a WS frame via the ws-frame:new event callback registered by
 *  WsFramesView's listen("ws-frame:new", …). */
async function injectWsFrame(
  page: Page,
  requestId: string,
  frame: {
    direction: "incoming" | "outgoing";
    timestamp: string;
    payload: string;
    size: number;
    opcode: number;
    truncated: boolean;
  },
) {
  await page.evaluate(
    ({ requestId, frame }) => {
      const internals = window.__TAURI_INTERNALS__;
      if (internals?.callbacks) {
        for (const [, cb] of internals.callbacks) {
          try {
            cb({ payload: { request_id: requestId, frame }, event: "ws-frame:new" });
          } catch {
            /* ignore */
          }
        }
      }
    },
    { requestId, frame },
  );
}

/** Open the Traffic page, inject the WS request, click the row, and
 *  switch to the WebSocket Frames tab. Caller must have already called
 *  mockTauriCommands with the desired get_ws_frames mock. */
async function openWsFramesTab(page: Page, req: typeof WS_REQUEST) {
  await page.goto("/");
  await page.waitForLoadState("networkidle");
  await injectRequest(page, req);
  // The captured row contains the host; click to open the detail panel.
  await page.getByText(req.host).first().click();
  // The "WebSocket Frames" tab only renders for is_websocket: true.
  await page.getByRole("button", { name: "WebSocket Frames" }).click();
}

test.describe("WS Frame Viewer", () => {
  test("ws_frames_view_shows_empty_for_non_ws_request", async ({ page }) => {
    // get_ws_frames returns an empty array — no frames were captured
    // for this WebSocket upgrade.
    await mockTauriCommands(page, {
      ...BASE_MOCKS,
      get_ws_frames: [],
    });
    await openWsFramesTab(page, WS_REQUEST);

    await expect(
      page.getByText("No WebSocket frames for this request."),
    ).toBeVisible();
  });

  test("ws_frames_view_shows_frames_after_ws_conversation", async ({ page }) => {
    // Mock get_ws_frames to return two captured frames (one outgoing,
    // one incoming) so the list populates without needing the backend.
    await mockTauriCommands(page, {
      ...BASE_MOCKS,
      get_ws_frames: [
        {
          direction: "outgoing",
          timestamp: "2026-06-14T10:00:00Z",
          payload: "hello",
          size: 5,
          opcode: 0x01,
          truncated: false,
        },
        {
          direction: "incoming",
          timestamp: "2026-06-14T10:00:01Z",
          payload: "world",
          size: 5,
          opcode: 0x01,
          truncated: false,
        },
      ],
    });
    await openWsFramesTab(page, WS_REQUEST);

    await expect(page.getByTestId("ws-frame-row")).toHaveCount(2);
    // Opcode column shows "Text" for both rows.
    await expect(page.getByText("Text").first()).toBeVisible();
    // Payload preview should include the truncated first message.
    await expect(page.getByText("hello").first()).toBeVisible();
    await expect(page.getByText("world").first()).toBeVisible();
  });

  test("ws_frames_view_text_hex_toggle", async ({ page }) => {
    await mockTauriCommands(page, {
      ...BASE_MOCKS,
      get_ws_frames: [
        {
          direction: "outgoing",
          timestamp: "2026-06-14T10:00:00Z",
          payload: "hello",
          size: 5,
          opcode: 0x01,
          truncated: false,
        },
      ],
    });
    await openWsFramesTab(page, WS_REQUEST);

    // Select the only frame — right pane shows FrameDetail.
    await page.getByTestId("ws-frame-row").first().click();
    // Default is text mode — payload rendered in a <pre>.
    await expect(page.getByText("hello").first()).toBeVisible();

    // Switch to hex — FrameDetail shows the HexDump output, which
    // contains an ASCII column for printable bytes ("hello").
    await page.getByRole("button", { name: "Hex", exact: true }).click();
    // HexDump renders bytes inside a <pre> with the ASCII column on
    // the right; "hello" should still be visible in the ASCII column.
    await expect(page.getByText("hello").first()).toBeVisible();
  });

  test("ws_frames_view_realtime_append", async ({ page }) => {
    // Starts empty; a ws-frame:new event for our requestId should
    // append a row without a manual refresh.
    await mockTauriCommands(page, {
      ...BASE_MOCKS,
      get_ws_frames: [],
    });
    await openWsFramesTab(page, WS_REQUEST);

    await expect(
      page.getByText("No WebSocket frames for this request."),
    ).toBeVisible();

    await injectWsFrame(page, WS_REQUEST.id, {
      direction: "outgoing",
      timestamp: "2026-06-14T10:00:02Z",
      payload: "ping",
      size: 4,
      opcode: 0x01,
      truncated: false,
    });

    // React StrictMode double-mounts effects in dev, so the listen()
    // callback may be registered twice and the event triggers both
    // listeners. We assert the row appears, not the exact count.
    await expect(page.getByText("ping").first()).toBeVisible();
    const rows = page.getByTestId("ws-frame-row");
    await expect(rows.first()).toBeVisible();
    expect(await rows.count()).toBeGreaterThanOrEqual(1);
  });
});