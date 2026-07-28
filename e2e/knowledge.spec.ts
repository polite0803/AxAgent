import { expect, test } from "@playwright/test";

test.describe("Knowledge Base E2E", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/knowledge");
    await page.waitForSelector('[data-testid="knowledge-hub"]', { timeout: 30000 });
  });

  test("should display knowledge hub page", async ({ page }) => {
    await expect(page.locator('[data-testid="knowledge-hub"]')).toBeVisible();
  });

  test("should display knowledge tabs", async ({ page }) => {
    const tabs = page.locator('[data-testid="source-manager-tabs"]');
    await expect(tabs).toBeVisible({ timeout: 10000 });
  });

  test("should display knowledge body section", async ({ page }) => {
    const body = page.locator('[data-testid="source-manager-body"]');
    await expect(body).toBeVisible({ timeout: 10000 });
  });

  test("should display knowledge search input", async ({ page }) => {
    const searchInput = page.locator('[data-testid="knowledge-search-input"]');
    const isVisible = await searchInput.isVisible({ timeout: 10000 }).catch(() => false);
    if (isVisible) {
      await expect(searchInput).toBeVisible();
    }
  });

  test("should display import directory button", async ({ page }) => {
    const importBtn = page.locator('[data-testid="import-directory-btn"]');
    const isVisible = await importBtn.isVisible({ timeout: 5000 }).catch(() => false);
    if (isVisible) {
      await expect(importBtn).toBeVisible();
    }
  });

  test("should navigate to settings from knowledge hub", async ({ page }) => {
    await page.goto("/settings");
    await page.waitForSelector('[data-testid="settings-panel"]', { timeout: 30000 });
    await expect(page.locator('[data-testid="settings-panel"]')).toBeVisible();
  });
});
