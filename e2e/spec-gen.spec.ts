import { test, expect } from "@playwright/test";

/**
 * Spec-generation panel E2E.
 *
 * The Rust side has its own fixture-driven test (see
 * `proxybot-core/tests/specgen_fixture.rs`) that pins the SM-4
 * coverage gate end-to-end on the heuristic. This Playwright suite
 * focuses on the React surface: the panel mounts, runs through its
 * happy path, and the replay button responds.
 *
 * Note: this requires a dev session with captured traffic on
 * `proxybot.db`. CI gates run against the seeded fixture session
 * the dev server reads in test mode (see `e2e/fixtures/`); locally
 * just generate some traffic before running.
 */
test.describe("Spec generation panel", () => {
  test("renders source badge after running generate", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: /AI/i }).click();
    await expect(page.getByText("OpenAPI / AsyncAPI 生成")).toBeVisible();

    // sessionId field lives in ApiInferenceTab. Set it to anything
    // matching the fixture-seeded session; the empty-string
    // fallback also works for untagged traffic.
    const sessionInput = page.getByPlaceholder(/Session/i);
    await sessionInput.fill("");

    await page.getByRole("button", { name: /生成规范/ }).click();

    // The source badge labels the result variant. With no API key
    // configured (CI default) we always end up on `Heuristic`; with
    // a key set the run might land on `Llm` or `Hybrid`. Accept any.
    await expect(page.getByText(/^(Llm|Heuristic|Hybrid)$/)).toBeVisible({
      timeout: 30_000,
    });
  });

  test("replay button enables once a spec exists", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: /AI/i }).click();

    // Replay should be disabled before generation.
    const replayBtn = page.getByRole("button", { name: /重放验证/ });
    await expect(replayBtn).toBeDisabled();

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
