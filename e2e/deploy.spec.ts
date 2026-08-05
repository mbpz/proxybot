import { test, expect } from "@playwright/test";

// Mirrors the pattern in e2e/all-pages.spec.ts:
// verifies page structure without requiring Tauri IPC mocks.

test.describe("Deploy Page", () => {
  test("page loads at /deploy", async ({ page }) => {
    await page.goto("/deploy");
    // Sidebar should render (avoids strict-mode collision with the
    // "proxybot_deployment" text inside the deploy form).
    await expect(page.locator("aside")).toBeVisible({ timeout: 5000 });
    const body = page.locator("body");
    await expect(body).toBeVisible();
  });

  test("shows Deploy panel title", async ({ page }) => {
    await page.goto("/deploy");
    await expect(page.locator(".panel-title", { hasText: "Deploy" })).toBeVisible();
  });

  test("has session ID input", async ({ page }) => {
    await page.goto("/deploy");
    const input = page.locator('input[placeholder="e.g. 2026-06-04-001"]');
    await expect(input).toBeVisible();
  });

  test("has project name input with default", async ({ page }) => {
    await page.goto("/deploy");
    const input = page.locator('input[placeholder="proxybot_deployment"]');
    await expect(input).toBeVisible();
    await expect(input).toHaveValue("proxybot_deployment");
  });

  test("has output path display", async ({ page }) => {
    await page.goto("/deploy");
    await expect(page.locator("code", { hasText: ".proxybot/deployments" })).toBeVisible();
  });

  test("has Initialize git repo checkbox checked by default", async ({ page }) => {
    await page.goto("/deploy");
    // Label wraps the input in DeployForm.tsx, so getByLabel resolves cleanly.
    const checkbox = page.getByLabel("Initialize git repo on write");
    await expect(checkbox).toBeChecked();
  });

  // Tab-switching test omitted: the preview tabs require a generated bundle,
  // which depends on a successful Tauri invoke. Without IPC mocks we can't
  // produce a bundle in the browser, so the tabs render but are inert.

  test("Clear status button is visible in Deploy page header", async ({ page }) => {
    await page.goto("/deploy");
    const btn = page.getByRole("button", { name: /Clear status/i });
    await expect(btn).toBeVisible();
  });

  test("Generate button is disabled when session ID is empty", async ({ page }) => {
    await page.goto("/deploy");
    const btn = page.getByRole("button", { name: "Generate Preview" });
    await expect(btn).toBeDisabled();
  });

  test("Generate button enables when session ID is filled", async ({ page }) => {
    await page.goto("/deploy");
    await page.fill('input[placeholder="e.g. 2026-06-04-001"]', "test-session");
    const btn = page.getByRole("button", { name: "Generate Preview" });
    await expect(btn).toBeEnabled();
  });

  test("shows empty state when no bundle generated", async ({ page }) => {
    await page.goto("/deploy");
    await expect(page.getByText("No preview yet")).toBeVisible();
  });

  test("Write to Disk button is disabled when no bundle", async ({ page }) => {
    await page.goto("/deploy");
    const btn = page.getByRole("button", { name: "Write to Disk" });
    await expect(btn).toBeDisabled();
  });

  test("Re-init Git button is disabled when no bundle path", async ({ page }) => {
    await page.goto("/deploy");
    const btn = page.getByRole("button", { name: "Re-init Git" });
    await expect(btn).toBeDisabled();
  });

  test("Deploy is excluded from the default sidebar", async ({ page }) => {
    await page.goto("/");
    await expect(page.locator("aside").getByRole("link", { name: "Deploy" })).toHaveCount(0);
  });

  test("Deploy remains reachable by deep link", async ({ page }) => {
    await page.goto("/deploy");
    await expect(page).toHaveURL("/deploy");
    await expect(page.locator(".panel-title", { hasText: "Deploy" })).toBeVisible();
  });
});
