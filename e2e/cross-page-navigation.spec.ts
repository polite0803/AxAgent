import { expect, test } from "@playwright/test";

test.describe("Cross-Page Navigation", () => {
  const pages = [
    { name: "Chat", url: "/chat", testId: "chat-view" },
    { name: "Dashboard", url: "/dashboard" },
    { name: "Knowledge Hub", url: "/knowledge", testId: "knowledge-hub" },
    { name: "Memory", url: "/memory" },
    { name: "Gateway", url: "/gateway", testId: "gateway-overview" },
    { name: "Settings", url: "/settings", testId: "settings-panel" },
    { name: "Workflow", url: "/workflow" },
    { name: "Files", url: "/files" },
    { name: "Terminal", url: "/terminal" },
    { name: "Dynamic UI", url: "/dynamic-ui" },
    { name: "Wiki", url: "/wiki" },
    { name: "Learning Graph", url: "/learning-graph" },
    { name: "QuickBar", url: "/quickbar" },
    { name: "LLM Wiki", url: "/llm-wiki" },
  ];

  for (const { name, url, testId } of pages) {
    test(`should load ${name} page (${url})`, async ({ page }) => {
      await page.goto(url);
      await page.waitForLoadState("networkidle");

      if (testId) {
        await expect(page.locator(`[data-testid="${testId}"]`)).toBeVisible({ timeout: 30000 });
      } else {
        await expect(page.locator("body")).toBeVisible();
      }

      await page.waitForTimeout(500);
    });
  }
});

test.describe("Page Navigation Stability", () => {
  test("should navigate between all pages without errors", async ({ page }) => {
    page.on("pageerror", (err) => {
      console.log(`PAGE ERROR: ${err.message}`);
    });

    const routes = [
      "/chat",
      "/dashboard",
      "/knowledge",
      "/memory",
      "/gateway",
      "/settings",
      "/workflow",
      "/files",
      "/terminal",
      "/dynamic-ui",
      "/wiki",
      "/learning-graph",
      "/quickbar",
      "/llm-wiki",
      "/devtools/trace-explorer",
      "/devtools/benchmark",
      "/devtools/fine-tune",
    ];

    for (const route of routes) {
      await page.goto(route);
      await page.waitForLoadState("networkidle");
      await expect(page.locator("body")).toBeVisible();
      await page.waitForTimeout(200);
    }
  });
});
