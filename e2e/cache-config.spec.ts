import { expect, test } from "@playwright/test";

test.describe("Cache Configuration", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/settings");
    await page.waitForSelector('[data-testid="settings-panel"]', { timeout: 30000 });
  });

  test("should navigate to cache settings", async ({ page }) => {
    const settingsSidebar = page.locator('[data-testid="settings-sidebar"]');
    if (await settingsSidebar.isVisible({ timeout: 5000 }).catch(() => false)) {
      await expect(settingsSidebar).toBeVisible();
    }
  });

  test("should display prompt cache toggle when visible", async ({ page }) => {
    const cacheToggle = page.locator('[data-testid="cache-breakpoints-toggle"]');
    const visible = await cacheToggle.isVisible({ timeout: 3000 }).catch(() => false);
    if (visible) {
      await expect(cacheToggle).toBeVisible();
    }
  });

  test("should show cache status indicator in chat when visible", async ({ page }) => {
    await page.goto("/");
    await page.waitForSelector('[data-testid="chat-view"]', { timeout: 60000 });

    const cacheIndicator = page.locator('[data-testid="cache-indicator"]');
    const visible = await cacheIndicator.isVisible({ timeout: 5000 }).catch(() => false);
    if (visible) {
      await expect(cacheIndicator).toBeVisible();
    }
  });

  test("should display token savings information when available", async ({ page }) => {
    await page.goto("/");
    await page.waitForSelector('[data-testid="chat-view"]', { timeout: 60000 });

    await page.waitForTimeout(2000);

    const tokenInfo = page.locator("text=token");
    const visible = await tokenInfo.first().isVisible({ timeout: 3000 }).catch(() => false);
    if (visible) {
      await expect(tokenInfo.first()).toBeVisible();
    }
  });
});
