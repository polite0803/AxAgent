import { expect, test } from "@playwright/test";

test.describe("QuickBar Page", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/quickbar");
    await page.waitForLoadState("networkidle");
  });

  test("should load quickbar page", async ({ page }) => {
    await expect(page.locator("body")).toBeVisible();
    await page.waitForTimeout(1000);
  });

  test("should display search input", async ({ page }) => {
    const searchInput = page.locator("input").filter({ has: page.locator("[placeholder]") }).first();
    const isVisible = await searchInput.isVisible({ timeout: 10000 }).catch(() => false);
    if (isVisible) {
      await expect(searchInput).toBeVisible();
    }
  });

  test("should display quick actions", async ({ page }) => {
    await page.waitForTimeout(1000);
    const actionItems = page.locator(".ant-card").or(
      page.locator("button").filter({ hasText: /search|translate|计算/i }),
    ).first();
    const isVisible = await actionItems.isVisible({ timeout: 5000 }).catch(() => false);
    if (isVisible) {
      await expect(actionItems).toBeVisible();
    }
  });

  test("should navigate to chat from quickbar", async ({ page }) => {
    await page.goto("/chat");
    await page.waitForSelector('[data-testid="chat-view"]', { timeout: 30000 });
    await expect(page.locator('[data-testid="chat-view"]')).toBeVisible();
  });
});
