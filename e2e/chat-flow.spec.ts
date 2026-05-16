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
    // S-25: 改为硬断言 — 输入区域是核心 UI，缺失一定表示回归
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

    // 验证输入已清空（消息已提交）
    await expect(input).toHaveValue("", { timeout: 5000 });
  });
});
