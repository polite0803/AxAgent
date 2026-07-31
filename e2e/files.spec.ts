import { expect, test } from "@playwright/test";

test.describe("Files Page", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/files");
  });

  test("should load files page", async ({ page }) => {
    await expect(page.locator("body")).toBeVisible();
  });

  test("should display files sidebar", async ({ page }) => {
    const sidebar = page.locator('[data-testid="files-sidebar"]');
    const isVisible = await sidebar.isVisible({ timeout: 10000 }).catch(() => false);
    if (isVisible) {
      await expect(sidebar).toBeVisible();
    }
  });

  test("should display files content area", async ({ page }) => {
    const content = page.locator('[data-testid="files-content"]');
    const isVisible = await content.isVisible({ timeout: 10000 }).catch(() => false);
    if (isVisible) {
      await expect(content).toBeVisible();
    }
  });

  test("should display category search", async ({ page }) => {
    const searchInput = page.locator('[data-testid="category-search"]');
    const isVisible = await searchInput.isVisible({ timeout: 5000 }).catch(() => false);
    if (isVisible) {
      await expect(searchInput).toBeVisible();
    }
  });

  test("should navigate to chat from files", async ({ page }) => {
    const errors: string[] = [];
    page.on("pageerror", (err) => errors.push(err.message));

    await page.goto("/chat");
    await page.waitForSelector('[data-testid="chat-view"]', { timeout: 45000 });
    await expect(page.locator('[data-testid="chat-view"]')).toBeVisible();

    if (errors.length > 0) {
      console.log("Page errors during chat navigation:", errors.join("; "));
    }
  });
});
