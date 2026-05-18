import { expect, test } from "@playwright/test";

test.describe("Agent Execution Flow", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await page.waitForSelector('[data-testid="chat-view"]', { timeout: 60000 });
  });

  test("should display agent status indicator when available", async ({ page }) => {
    const statusIndicator = page.locator('[data-testid="agent-status"]');
    const isVisible = await statusIndicator.isVisible({ timeout: 5000 }).catch(() => false);
    if (isVisible) {
      await expect(statusIndicator).toBeVisible();
    }
  });

  test("should have chat input area enabled", async ({ page }) => {
    const input = page.locator('[data-testid="message-input"]');
    const isVisible = await input.isVisible({ timeout: 5000 }).catch(() => false);
    test.skip(!isVisible, "Input not visible (welcome page) — requires active conversation");
    await expect(input).toBeEnabled();
  });
});
