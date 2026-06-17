import { test, expect } from "@playwright/test";

test.describe("Spec generation panel", () => {
  test("renders and shows source badge", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: /AI/i }).click();
    await expect(page.getByText("OpenAPI / AsyncAPI 生成")).toBeVisible();
    await page.getByRole("button", { name: /生成规范/ }).click();
    await expect(page.getByText(/Llm|Heuristic|Hybrid/)).toBeVisible({ timeout: 30_000 });
  });
});
