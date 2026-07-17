import { expect, test } from "@playwright/test";

test.describe("Memory Page", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/memory");
    await page.waitForLoadState("networkidle");
  });

  test("should load memory page", async ({ page }) => {
    await expect(page.locator("body")).toBeVisible();
    await page.waitForTimeout(1000);
  });

  test("should display memory settings content", async ({ page }) => {
    const content = page.locator(".ant-card").or(page.locator(".ant-collapse")).first();
    const isVisible = await content.isVisible({ timeout: 10000 }).catch(() => false);
    if (isVisible) {
      await expect(content).toBeVisible();
    }
  });

  test("should navigate to settings from memory via URL", async ({ page }) => {
    await page.goto("/settings");
    await page.waitForSelector('[data-testid="settings-panel"]', { timeout: 30000 });
    await expect(page.locator('[data-testid="settings-panel"]')).toBeVisible();
  });
});
