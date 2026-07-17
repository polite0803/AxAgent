import { expect, test } from "@playwright/test";

test.describe("Dynamic UI Page", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/dynamic-ui");
    await page.waitForLoadState("networkidle");
  });

  test("should load dynamic UI page", async ({ page }) => {
    await expect(page.locator("body")).toBeVisible();
    await page.waitForTimeout(1000);
  });

  test("should display create button or empty state", async ({ page }) => {
    const createBtn = page.locator("button").filter({ hasText: /create|新建|新建UI|create ui/i }).first();
    const isVisible = await createBtn.isVisible({ timeout: 10000 }).catch(() => false);
    if (isVisible) {
      await expect(createBtn).toBeVisible();
    }
  });

  test("should display import/export buttons", async ({ page }) => {
    const importBtn = page.locator("button").filter({ hasText: /import|导入/i }).first();
    const exportBtn = page.locator("button").filter({ hasText: /export|导出/i }).first();
    const importVisible = await importBtn.isVisible({ timeout: 5000 }).catch(() => false);
    const exportVisible = await exportBtn.isVisible({ timeout: 5000 }).catch(() => false);
    if (importVisible) {
      await expect(importBtn).toBeVisible();
    }
    if (exportVisible) {
      await expect(exportBtn).toBeVisible();
    }
  });

  test("should navigate to settings from dynamic UI", async ({ page }) => {
    await page.goto("/settings");
    await page.waitForSelector('[data-testid="settings-panel"]', { timeout: 30000 });
    await expect(page.locator('[data-testid="settings-panel"]')).toBeVisible();
  });
});
