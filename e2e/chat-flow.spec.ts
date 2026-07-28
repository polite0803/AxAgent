import { expect, test } from "@playwright/test";

test.describe("Knowledge Hub (Hard Assertions)", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/knowledge");
    await page.waitForSelector('[data-testid="knowledge-hub"]', { timeout: 30000 });
  });

  test("should display knowledge hub interface", async ({ page }) => {
    await expect(page.locator('[data-testid="knowledge-hub"]')).toBeVisible({ timeout: 10000 });
  });

  test("should have knowledge tabs visible", async ({ page }) => {
    await expect(page.locator('[data-testid="source-manager-tabs"]')).toBeVisible({ timeout: 5000 });
  });

  test("should show source manager body", async ({ page }) => {
    const body = page.locator('[data-testid="source-manager-body"]');
    await expect(body).toBeVisible({ timeout: 5000 });
  });
});
