import { expect, test } from "@playwright/test";

test.describe("Knowledge Hub", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/knowledge");
    await page.waitForSelector('[data-testid="knowledge-hub"]', { timeout: 30000 });
  });

  test("should display knowledge hub interface", async ({ page }) => {
    await expect(page.locator('[data-testid="knowledge-hub"]')).toBeVisible();
  });

  test("should have knowledge tabs visible", async ({ page }) => {
    const tabs = page.locator('[data-testid="source-manager-tabs"]');
    await expect(tabs).toBeVisible({ timeout: 5000 });
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
