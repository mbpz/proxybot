import { defineConfig, devices } from "@playwright/test";

const browserExecutablePath = process.env.PLAYWRIGHT_BROWSER_EXECUTABLE_PATH;

export default defineConfig({
  testDir: "./e2e",
  // Browser-mock runs are wall-clock bound on the shared Vite dev server.
  // Keep a modest CI retry budget so unrelated tests are not reported as
  // regressions when the dev server stalls; real failures still surface.
  timeout: 20000,
  retries: process.env.CI ? 2 : 0,
  workers: 1,
  reporter: [["list"], ["html", { outputFolder: "e2e-report" }]],
  use: {
    baseURL: "http://localhost:1420",
    trace: "on-first-retry",
    screenshot: "only-on-failure",
  },
  projects: [
    {
      name: "chromium",
      use: {
        ...devices["Desktop Chrome"],
        ...(browserExecutablePath
          ? { launchOptions: { executablePath: browserExecutablePath } }
          : {}),
      },
    },
  ],
  webServer: {
    // Invoke the locked local Vite binary directly so E2E runs do not depend
    // on Corepack reaching the package registry during test startup.
    command: "node node_modules/vite/bin/vite.js",
    url: "http://localhost:1420",
    reuseExistingServer: true,
  },
});
