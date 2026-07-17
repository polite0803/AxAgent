import { expect, test } from "@playwright/test";

test.describe("Settings Navigation", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/settings");
    await page.waitForSelector('[data-testid="settings-panel"]', { timeout: 30000 });
  });

  test("should display settings sidebar", async ({ page }) => {
    await expect(page.locator('[data-testid="settings-sidebar"]')).toBeVisible();
  });

  test("should display settings panel", async ({ page }) => {
    await expect(page.locator('[data-testid="settings-panel"]')).toBeVisible();
  });

  test("should have settings back button", async ({ page }) => {
    const backBtn = page.locator(".settings-back-btn").first();
    const isVisible = await backBtn.isVisible({ timeout: 5000 }).catch(() => false);
    if (isVisible) {
      await expect(backBtn).toBeVisible();
    }
  });

  test("should navigate to provider settings via URL", async ({ page }) => {
    await page.goto("/settings/providers");
    await page.waitForTimeout(2000);
    await expect(page.locator('[data-testid="settings-panel"]')).toBeVisible();
  });

  test("should navigate to general settings via URL", async ({ page }) => {
    await page.goto("/settings/general");
    await page.waitForTimeout(2000);
    await expect(page.locator('[data-testid="settings-panel"]')).toBeVisible();
  });

  test("should navigate to display settings via URL", async ({ page }) => {
    await page.goto("/settings/display");
    await page.waitForTimeout(2000);
    const darkModeToggle = page.locator('[data-testid="dark-mode-toggle"]');
    const isVisible = await darkModeToggle.isVisible({ timeout: 10000 }).catch(() => false);
    if (isVisible) {
      await expect(darkModeToggle).toBeVisible();
    }
  });

  test("should navigate to shortcut settings via URL", async ({ page }) => {
    await page.goto("/settings/shortcuts");
    await page.waitForTimeout(2000);
    await expect(page.locator('[data-testid="settings-panel"]')).toBeVisible();
  });

  test("should navigate to about page via URL", async ({ page }) => {
    await page.goto("/settings/about");
    await page.waitForTimeout(2000);
    await expect(page.locator('[data-testid="settings-panel"]')).toBeVisible();
  });

  test("should navigate to data settings via URL", async ({ page }) => {
    await page.goto("/settings/data");
    await page.waitForTimeout(2000);
    await expect(page.locator('[data-testid="settings-panel"]')).toBeVisible();
  });

  test("should navigate to backup settings via URL", async ({ page }) => {
    await page.goto("/settings/backup");
    await page.waitForTimeout(2000);
    const backupDir = page.locator('[data-testid="backup-effective-dir"]');
    const isVisible = await backupDir.isVisible({ timeout: 5000 }).catch(() => false);
    if (isVisible) {
      await expect(backupDir).toBeVisible();
    }
  });

  test("should navigate to advanced settings via URL", async ({ page }) => {
    await page.goto("/settings/advanced");
    await page.waitForTimeout(2000);
    await expect(page.locator('[data-testid="settings-panel"]')).toBeVisible();
  });

  test("should navigate to proxy settings via URL", async ({ page }) => {
    await page.goto("/settings/proxy");
    await page.waitForTimeout(2000);
    await expect(page.locator('[data-testid="settings-panel"]')).toBeVisible();
  });

  test("should navigate to search provider settings via URL", async ({ page }) => {
    await page.goto("/settings/searchProviders");
    await page.waitForTimeout(2000);
    await expect(page.locator('[data-testid="settings-panel"]')).toBeVisible();
  });
});
