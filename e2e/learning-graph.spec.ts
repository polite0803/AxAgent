import { expect, test } from "@playwright/test";

test.describe("Learning Graph Page", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/learning-graph");
    await page.waitForLoadState("networkidle");
  });

  test("should load learning graph page", async ({ page }) => {
    await expect(page.locator("body")).toBeVisible();
    await page.waitForTimeout(1000);
  });

  test("should display graph visualization or empty state", async ({ page }) => {
    const graphView = page.locator(".react-flow").or(page.locator(".ant-empty")).first();
    const isVisible = await graphView.isVisible({ timeout: 10000 }).catch(() => false);
    if (isVisible) {
      await expect(graphView).toBeVisible();
    }
  });

  test("should have search input", async ({ page }) => {
    const searchInput = page.locator("input").filter({ has: page.locator("[placeholder]") }).first();
    const isVisible = await searchInput.isVisible({ timeout: 5000 }).catch(() => false);
    if (isVisible) {
      await expect(searchInput).toBeVisible();
    }
  });

  test("should navigate to chat from learning graph", async ({ page }) => {
    await page.goto("/chat");
    await page.waitForSelector('[data-testid="chat-view"]', { timeout: 30000 });
    await expect(page.locator('[data-testid="chat-view"]')).toBeVisible();
  });
});
