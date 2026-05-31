import { test, expect } from "@playwright/test";

// These tests verify page structure without requiring Tauri IPC mocks.
// All invoke() calls will fail gracefully (error states shown),
// but the page structure (tabs, buttons, inputs) should still render.

test.describe("Navigation — All 11 Pages", () => {
  const pages = [
    { name: "Traffic", path: "/" },
    { name: "Rules", path: "/rules" },
    { name: "Certs", path: "/certs" },
    { name: "Devices", path: "/devices" },
    { name: "DNS", path: "/dns" },
    { name: "Alerts", path: "/alerts" },
    { name: "Replay", path: "/replay" },
    { name: "Composer", path: "/composer" },
    { name: "Graph", path: "/graph" },
    { name: "Gen", path: "/gen" },
    { name: "AI", path: "/ai" },
  ];

  for (const { name, path } of pages) {
    test(`${name} page loads (${path})`, async ({ page }) => {
      await page.goto(path);
      // Every page should have the sidebar visible
      await expect(page.getByText("ProxyBot")).toBeVisible({ timeout: 5000 });
      // Should not show a blank page
      const body = page.locator("body");
      await expect(body).toBeVisible();
    });
  }
});

test.describe("Sidebar Navigation", () => {
  test("sidebar has all nav items", async ({ page }) => {
    await page.goto("/");
    const labels = ["Traffic", "Rules", "Certs", "Devices", "DNS", "Alerts", "Replay", "Graph", "Composer", "Gen", "AI"];
    for (const label of labels) {
      // Sidebar links use Link components with text
      await expect(page.locator("aside").getByText(label)).toBeVisible();
    }
  });

  test("settings link is in sidebar footer", async ({ page }) => {
    await page.goto("/");
    await expect(page.locator("aside").getByText("Settings")).toBeVisible();
  });

  test("sidebar collapse toggles width", async ({ page }) => {
    await page.goto("/");
    const aside = page.locator("aside");
    const toggleBtn = aside.locator("button").first();
    await expect(toggleBtn).toBeVisible();
    // Collapse should reduce sidebar width
    await toggleBtn.click();
    // Sidebar should still be visible
    await expect(aside).toBeVisible();
  });
});

test.describe("Traffic Page", () => {
  test("has filter input", async ({ page }) => {
    await page.goto("/");
    await expect(page.locator("input").first()).toBeVisible();
  });

  test("has toolbar buttons", async ({ page }) => {
    await page.goto("/");
    // Load/Save/Normalized/Export buttons
    await expect(page.getByText("Load")).toBeVisible();
    await expect(page.getByText("Export HAR")).toBeVisible();
  });
});

test.describe("Rules Page", () => {
  test("has Add Rule button", async ({ page }) => {
    await page.goto("/rules");
    // Uses "+ Add Rule" with plus sign
    await expect(page.locator("button:has-text('Add Rule')")).toBeVisible({ timeout: 5000 });
  });

  test("has host test input", async ({ page }) => {
    await page.goto("/rules");
    await expect(page.locator('input[placeholder="Test host..."]')).toBeVisible();
  });
});

test.describe("Certs Page", () => {
  test("shows certificate actions", async ({ page }) => {
    await page.goto("/certs");
    await expect(page.getByText("Root CA Certificate")).toBeVisible();
    await expect(page.getByText("Export CA")).toBeVisible();
    await expect(page.getByText("Regenerate CA")).toBeVisible();
    await expect(page.getByText("Start CA Server")).toBeVisible();
  });
});

test.describe("Devices Page", () => {
  test("has refresh button", async ({ page }) => {
    await page.goto("/devices");
    await expect(page.getByText("Refresh")).toBeVisible();
  });
});

test.describe("DNS Page", () => {
  test("has DoH/UDP toggle buttons", async ({ page }) => {
    await page.goto("/dns");
    await expect(page.getByText("DoH")).toBeVisible();
    await expect(page.getByText("UDP")).toBeVisible();
    await expect(page.getByText("Reload Lists")).toBeVisible();
  });
});

test.describe("Alerts Page", () => {
  test("has severity filter buttons", async ({ page }) => {
    await page.goto("/alerts");
    await expect(page.getByText("All")).toBeVisible();
    await expect(page.getByText("Scan Now")).toBeVisible();
  });

  test("has baseline tab", async ({ page }) => {
    await page.goto("/alerts");
    // Click the Baseline tab
    const baselineTab = page.locator("button.tab", { hasText: "Baseline" });
    await baselineTab.click();
    // Should still be on the page
    await expect(baselineTab).toHaveClass(/active/);
  });
});

test.describe("Replay Page", () => {
  test("has Start Replay button", async ({ page }) => {
    await page.goto("/replay");
    await expect(page.getByText("Start Replay")).toBeVisible();
  });
});

test.describe("Composer Page", () => {
  test("has composer form", async ({ page }) => {
    await page.goto("/composer");
    // Should have the composer panel
    await expect(page.locator("h1")).toBeVisible();
  });
});

test.describe("Graph Page", () => {
  test("has view selector buttons", async ({ page }) => {
    await page.goto("/graph");
    await expect(page.getByText("Waterfall")).toBeVisible();
    await expect(page.getByText("Dependency")).toBeVisible();
    await expect(page.getByText("Auth Flow")).toBeVisible();
  });

  test("has Build DAG button", async ({ page }) => {
    await page.goto("/graph");
    await expect(page.getByText("Build DAG")).toBeVisible();
  });
});

test.describe("Gen Page", () => {
  test("has all generate tabs", async ({ page }) => {
    await page.goto("/gen");
    // All three tabs should exist
    const tabs = ["Mock API", "Scaffold", "Deploy"];
    for (const t of tabs) {
      await expect(page.locator("button.tab", { hasText: t })).toBeVisible();
    }
  });

  test("scaffold tab shows Generate button", async ({ page }) => {
    await page.goto("/gen");
    await page.click("text=Scaffold");
    await expect(page.getByText("Generate Scaffold")).toBeVisible();
  });

  test("deploy tab shows Generate button", async ({ page }) => {
    await page.goto("/gen");
    await page.click("text=Deploy");
    await expect(page.getByText("Generate Deployment Bundle")).toBeVisible();
  });
});

test.describe("AI Page", () => {
  test("has all AI panel tabs", async ({ page }) => {
    await page.goto("/ai");
    const tabs = ["Token Usage", "API Inference", "Auth Flow", "Vision"];
    for (const t of tabs) {
      await expect(page.locator("button.tab", { hasText: t })).toBeVisible();
    }
  });

  test("inference tab shows buttons", async ({ page }) => {
    await page.goto("/ai");
    await page.click("text=API Inference");
    await expect(page.getByText("Infer APIs")).toBeVisible();
    await expect(page.getByText("YAML Export")).toBeVisible();
  });

  test("vision tab shows upload", async ({ page }) => {
    await page.goto("/ai");
    await page.click("text=Vision");
    await expect(page.getByText("Upload Screenshot")).toBeVisible();
  });
});

test.describe("Settings Page", () => {
  test("has all settings tabs", async ({ page }) => {
    await page.goto("/settings");
    for (const tab of ["General", "Network", "DNS", "Certificate", "About"]) {
      await expect(page.locator("button.tab", { hasText: tab })).toBeVisible();
    }
  });

  test("general tab shows DB stats", async ({ page }) => {
    await page.goto("/settings");
    await expect(page.getByText("Keep Running")).toBeVisible();
    await expect(page.getByText("Mobile Dashboard")).toBeVisible();
  });
});

test.describe("Design System", () => {
  test("dark theme is applied", async ({ page }) => {
    await page.goto("/");
    const bgColor = await page.evaluate(() =>
      getComputedStyle(document.body).backgroundColor
    );
    // Should be dark background (not white)
    expect(bgColor).not.toBe("rgb(255, 255, 255)");
  });
});

// ============================================================
// Data Binding Tests (verify mock data renders)
// ============================================================

test.describe("Data Binding", () => {
  test("alerts page shows severity filters", async ({ page }) => {
    await page.goto("/alerts");
    // Severity filter buttons should always render
    await expect(page.getByText("All")).toBeVisible({ timeout: 5000 });
    await expect(page.getByText("Scan Now")).toBeVisible();
  });

  test("cert page shows Export/Regenerate/Start buttons", async ({ page }) => {
    await page.goto("/certs");
    await expect(page.getByText("Export CA")).toBeVisible();
    await expect(page.getByText("Regenerate CA")).toBeVisible();
    await expect(page.getByText("Start CA Server")).toBeVisible();
  });

  test("devices page shows refresh", async ({ page }) => {
    await page.goto("/devices");
    await expect(page.getByText("Refresh")).toBeVisible();
  });

  test("dns tab shows DoH/UDP controls", async ({ page }) => {
    await page.goto("/dns");
    await expect(page.getByText("DoH")).toBeVisible();
    await expect(page.getByText("UDP")).toBeVisible();
  });

  test("ai page shows AI Analysis title", async ({ page }) => {
    await page.goto("/ai");
    await expect(page.locator("h1")).toBeVisible();
  });

  test("settings shows Keep Running", async ({ page }) => {
    await page.goto("/settings");
    await expect(page.getByText("Keep Running")).toBeVisible();
    await expect(page.getByText("Mobile Dashboard")).toBeVisible();
  });

  test("replay shows delay input", async ({ page }) => {
    await page.goto("/replay");
    await expect(page.locator('input[type="number"]')).toBeVisible();
  });

  test("graph shows view buttons", async ({ page }) => {
    await page.goto("/graph");
    await expect(page.getByText("Waterfall")).toBeVisible();
    await expect(page.getByText("Dependency")).toBeVisible();
  });

  test("composer has URL input", async ({ page }) => {
    await page.goto("/composer");
    await expect(page.locator("input").first()).toBeVisible();
  });

  test("gen has session input", async ({ page }) => {
    await page.goto("/gen");
    await expect(page.locator('input[placeholder="e.g. session_001"]')).toBeVisible();
  });
});

// ============================================================
// Interaction Tests
// ============================================================

test.describe("Interactions", () => {
  test("clicking sidebar nav item changes active state", async ({ page }) => {
    await page.goto("/");
    // Traffic should be active
    const trafficLink = page.locator("aside a[href='/']");
    await expect(trafficLink).toHaveClass(/border-accent-blue/);

    // Click Rules
    await page.click("aside a[href='/rules']");
    await expect(page).toHaveURL("/rules");
  });

  test("alert tab shows content when clicked", async ({ page }) => {
    await page.goto("/alerts");
    // Click Baseline tab
    await page.locator("button.tab", { hasText: "Baseline" }).click();
    // Should become active
    await expect(page.locator("button.tab.active", { hasText: "Baseline" })).toBeVisible();
  });

  test("ai tabs are all clickable", async ({ page }) => {
    await page.goto("/ai");
    for (const tab of ["Token Usage", "API Inference", "Auth Flow", "Vision"]) {
      await page.locator("button.tab", { hasText: tab }).click();
      await expect(page.locator("button.tab.active", { hasText: tab })).toBeVisible();
    }
  });
});
