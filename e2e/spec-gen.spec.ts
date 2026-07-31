import { test, expect } from "@playwright/test";
import { mockTauriCommands } from "./fixtures/tauri-mock";

const SPEC_RESULT = {
  openapi: { OpenApi: "openapi: 3.1.0\npaths:\n  /health:\n    get: {}\n" },
  asyncapi: null,
  coverage: {
    total_requests: 1,
    covered_in_openapi: 1,
    covered_in_asyncapi: 0,
    uncovered_paths: [],
    coverage_rate: 1,
  },
  replay: null,
  generated_at: "2026-07-31T00:00:00Z",
  source: "Heuristic",
};

/**
 * Spec-generation panel E2E.
 *
 * The Rust side has its own fixture-driven test (see
 * `proxybot-core/tests/specgen_fixture.rs`) that pins the SM-4
 * coverage gate end-to-end on the heuristic. This Playwright suite
 * focuses on the React surface: the panel mounts, runs through its
 * happy path, and the replay button responds.
 *
 * Tauri commands are mocked so the suite is deterministic and never reads a
 * developer's local `proxybot.db`.
 */
test.describe("Spec generation panel", () => {
  test.beforeEach(async ({ page }) => {
    await mockTauriCommands(page, {
      get_ai_stats: { stats: [] },
      set_active_session: null,
      get_traffic_records: [],
      generate_spec: SPEC_RESULT,
    });
    await page.goto("/ai");
    await page.getByRole("button", { name: "API Inference", exact: true }).click();
  });

  test("renders source badge after running generate", async ({ page }) => {
    await expect(page.getByText("OpenAPI / AsyncAPI 生成")).toBeVisible();

    const sessionInput = page.getByPlaceholder(/Session/i);
    await sessionInput.fill("session_001");

    await page.getByRole("button", { name: /生成规范/ }).click();

    // The source badge labels the result variant. With no API key
    // configured (CI default) we always end up on `Heuristic`; with
    // a key set the run might land on `Llm` or `Hybrid`. Accept any.
    await expect(page.getByText(/^(Llm|Heuristic|Hybrid)$/)).toBeVisible({
      timeout: 30_000,
    });
  });

  test("replay button enables once a spec exists", async ({ page }) => {
    // Replay should be disabled before generation.
    const replayBtn = page.getByRole("button", { name: /重放验证/ });
    await expect(replayBtn).toBeDisabled();

    await page.getByPlaceholder(/Session/i).fill("session_001");
    await page.getByRole("button", { name: /生成规范/ }).click();
    await expect(page.getByText(/^(Llm|Heuristic|Hybrid)$/)).toBeVisible({
      timeout: 30_000,
    });

    // Now the replay button should be enabled (we don't actually
    // click it — replay starts a mock server which Playwright would
    // need port hygiene for; just pin the gate).
    await expect(replayBtn).toBeEnabled();
  });
});
