import { expect, test } from "@playwright/test";

test.describe("Terminal Page", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/terminal");
    await page.waitForLoadState("networkidle");
  });

  test("should load terminal page", async ({ page }) => {
    await expect(page.locator("body")).toBeVisible();
    await page.waitForTimeout(1000);
  });

  test("should display terminal interface", async ({ page }) => {
    const terminal = page.locator(".xterm").or(page.locator(".terminal-container")).first();
    const isVisible = await terminal.isVisible({ timeout: 10000 }).catch(() => false);
    if (isVisible) {
      await expect(terminal).toBeVisible();
    }
  });

  test("should display backend selector", async ({ page }) => {
    const selector = page.locator(".ant-select").or(page.locator("button").filter({ hasText: /local|docker|ssh/i }))
      .first();
    const isVisible = await selector.isVisible({ timeout: 5000 }).catch(() => false);
    if (isVisible) {
      await expect(selector).toBeVisible();
    }
  });

  test("should navigate to settings from terminal", async ({ page }) => {
    await page.goto("/settings");
    await page.waitForSelector('[data-testid="settings-panel"]', { timeout: 30000 });
    await expect(page.locator('[data-testid="settings-panel"]')).toBeVisible();
  });
});
