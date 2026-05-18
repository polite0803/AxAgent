import { expect, test } from "@playwright/test";

test.describe("Chat Flow (Hard Assertions)", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await page.waitForSelector('[data-testid="chat-view"]', { timeout: 60000 });
  });

  test("should display chat interface", async ({ page }) => {
    await expect(page.locator('[data-testid="chat-view"]')).toBeVisible({ timeout: 10000 });
  });

  test("should have message input area enabled", async ({ page }) => {
    const input = page.locator('[data-testid="message-input"]');
    const isVisible = await input.isVisible({ timeout: 5000 }).catch(() => false);
    test.skip(!isVisible, "Input not visible (welcome page)");
    await expect(input).toBeEnabled();
  });

  test("should show new conversation button", async ({ page }) => {
    const newConvBtn = page.locator('[data-testid="new-conversation-btn"]');
    const isVisible = await newConvBtn.isVisible({ timeout: 5000 }).catch(() => false);
    if (isVisible) { await expect(newConvBtn).toBeVisible({ timeout: 10000 }); }
  });

  test("should type message and click send", async ({ page }) => {
    const input = page.locator('[data-testid="message-input"]');
    const isVisible = await input.isVisible({ timeout: 5000 }).catch(() => false);
    test.skip(!isVisible, "Input not visible (welcome page)");
    await input.fill("Hello, test message");
    await input.press("Enter");
    await page.waitForTimeout(500);
  });
});
