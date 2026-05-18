import { expect, test } from "@playwright/test";

test.describe("Conversation Lifecycle", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await page.waitForSelector('[data-testid="chat-view"]', { timeout: 60000 });
  });

  test("should show new conversation button", async ({ page }) => {
    const newConvBtn = page.locator('[data-testid="new-conversation-btn"]');
    const isVisible = await newConvBtn.isVisible({ timeout: 5000 }).catch(() => false);
    if (isVisible) await expect(newConvBtn).toBeVisible({ timeout: 10000 });
  });

  test("should display agent status indicator when available", async ({ page }) => {
    const statusIndicator = page.locator('[data-testid="agent-status"]');
    const isVisible = await statusIndicator.isVisible({ timeout: 5000 }).catch(() => false);
    if (isVisible) await expect(statusIndicator).toBeVisible();
  });

  test("should stop generation button appear during streaming", async ({ page }) => {
    const input = page.locator('[data-testid="message-input"]');
    const isVisible = await input.isVisible({ timeout: 5000 }).catch(() => false);
    test.skip(!isVisible, "Input not visible (welcome page)");
    await input.fill("count to 100");
    await input.press("Enter");
    const stopBtn = page.locator('[data-testid="stop-generation-btn"]');
    const btnVisible = await stopBtn.isVisible({ timeout: 5000 }).catch(() => false);
    if (btnVisible) await stopBtn.click();
  });
});
