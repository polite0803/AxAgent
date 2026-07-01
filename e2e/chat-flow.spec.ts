import { expect, test } from "@playwright/test";

test.describe("Knowledge Hub (Hard Assertions)", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/knowledge");
    await page.waitForSelector('[data-testid="knowledge-hub"]', { timeout: 30000 });
  });

  test("should display knowledge hub interface", async ({ page }) => {
    await expect(page.locator('[data-testid="knowledge-hub"]')).toBeVisible({ timeout: 10000 });
  });

  test("should have knowledge header visible", async ({ page }) => {
    await expect(page.locator(".kb-header-title")).toBeVisible({ timeout: 5000 });
  });

  test("should show source manager section", async ({ page }) => {
    const sourceManager = page.locator(".kb-body");
    await expect(sourceManager).toBeVisible({ timeout: 5000 });
  });
});
