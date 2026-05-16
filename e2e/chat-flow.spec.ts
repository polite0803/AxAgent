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
    await expect(input).toBeVisible({ timeout: 15000 });
    await expect(input).toBeEnabled();
  });

  test("should show new conversation button", async ({ page }) => {
    const newConvBtn = page.locator('[data-testid="new-conversation-btn"]');
    await expect(newConvBtn).toBeVisible({ timeout: 10000 });
  });

  test("should type message and click send", async ({ page }) => {
    const input = page.locator('[data-testid="message-input"]');
    await expect(input).toBeVisible({ timeout: 10000 });
    await input.fill("Hello, test message");
    await input.press("Enter");

    // 输入应提交（浏览器 mock 中可能有延迟，等待一下）
    await page.waitForTimeout(500);
  });
});
