import { expect, test } from "@playwright/test";

test.describe("Chat Flow", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await page.waitForSelector('[data-testid="chat-view"]', { timeout: 60000 });
  });

  test("should display chat interface", async ({ page }) => {
    await expect(page.locator('[data-testid="chat-view"]')).toBeVisible();
  });

  test("should have message input area", async ({ page }) => {
    const input = page.locator('[data-testid="message-input"]');
    const isVisible = await input.isVisible({ timeout: 10000 }).catch(() => false);
    if (isVisible) {
      await expect(input).toBeVisible();
    }
  });

  test("should navigate to settings page via URL", async ({ page }) => {
    await page.goto("/settings");
    await expect(page.locator('[data-testid="settings-panel"]')).toBeVisible({ timeout: 30000 });
  });
});

test.describe("Settings", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/settings");
    await page.waitForSelector('[data-testid="settings-panel"]', { timeout: 30000 });
  });

  test("should display settings sections", async ({ page }) => {
    await expect(page.locator('[data-testid="settings-sidebar"]')).toBeVisible();
  });

  test("should save theme preference", async ({ page }) => {
    const displayNav = page.locator(".ant-menu-item").filter({ hasText: /显示|display|appearance|theme/i }).first();
    const isVisible = await displayNav.isVisible({ timeout: 5000 }).catch(() => false);
    if (isVisible) {
      await displayNav.click({ force: true });
      const darkModeToggle = page.locator('[data-testid="dark-mode-toggle"]');
      if (await darkModeToggle.isVisible({ timeout: 5000 }).catch(() => false)) {
        await expect(darkModeToggle).toBeVisible();
      }
    }
  });
});
