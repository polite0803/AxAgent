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
    // S-25: 硬断言 — 输入区域是核心 UI 元素
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

  test("should show dark mode toggle in settings", async ({ page }) => {
    // S-25: 硬断言 — 导航到设置页面验证暗色模式切换存在
    const darkModeToggle = page.locator('[data-testid="dark-mode-toggle"]');
    await expect(darkModeToggle).toBeVisible({ timeout: 15000 });
  });
});
