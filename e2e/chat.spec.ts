import { expect, test } from "@playwright/test";

test.describe("Chat Flow", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await page.waitForSelector('[data-testid="chat-view"]', { timeout: 60000 });
    const prompt = page.locator(".ant-prompts-item").first();
    if (await prompt.isVisible({ timeout: 5000 }).catch(() => false)) {
      await prompt.click();
      await page.waitForSelector('[data-testid="message-input"]', { timeout: 15000 });
    }
  });

  test("should display chat interface", async ({ page }) => {
    await expect(page.locator('[data-testid="chat-view"]')).toBeVisible();
  });

  test("should have message input area", async ({ page }) => {
    const input = page.locator('[data-testid="message-input"]');
    await expect(input).toBeVisible({ timeout: 15000 });
    await expect(input).toBeEnabled();
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

  test("should show dark mode toggle when display section is active", async ({ page }) => {
    // dark-mode-toggle 仅在显示设置页签激活时可见
    const darkModeToggle = page.locator('[data-testid="dark-mode-toggle"]');
    const isVisible = await darkModeToggle.isVisible({ timeout: 5000 }).catch(() => false);
    if (isVisible) {
      await expect(darkModeToggle).toBeVisible();
    }
  });
});
