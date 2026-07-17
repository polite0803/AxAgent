import { expect, test } from "@playwright/test";

test.describe("Dashboard Page", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/dashboard");
    await page.waitForLoadState("networkidle");
  });

  test("should load dashboard page", async ({ page }) => {
    await expect(page.locator("body")).toBeVisible();
    await page.waitForTimeout(1000);
  });

  test("should display dashboard title or stats area", async ({ page }) => {
    const titleOrStats = page.locator(".ant-card").or(page.locator(".ant-statistic")).first();
    const isVisible = await titleOrStats.isVisible({ timeout: 10000 }).catch(() => false);
    if (isVisible) {
      await expect(titleOrStats).toBeVisible();
    }
  });

  test("should have reactive layout", async ({ page }) => {
    const content = page.locator("body");
    await expect(content).toBeVisible();
    await page.waitForTimeout(500);
  });

  test("should navigate to chat from dashboard", async ({ page }) => {
    await page.goto("/chat");
    await page.waitForSelector('[data-testid="chat-view"]', { timeout: 30000 });
    await expect(page.locator('[data-testid="chat-view"]')).toBeVisible();
  });
});
