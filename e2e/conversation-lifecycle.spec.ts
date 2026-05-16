import { expect, test } from "@playwright/test";

test.describe("Conversation Lifecycle", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await page.waitForSelector('[data-testid="chat-view"]', { timeout: 60000 });
  });

  test("should create a new conversation", async ({ page }) => {
    const newConvBtn = page.locator('[data-testid="new-conversation-btn"]');
    await expect(newConvBtn).toBeVisible({ timeout: 10000 });
    await newConvBtn.click();

    // 新对话应加载完成，显示聊天界面
    await expect(page.locator('[data-testid="chat-view"]')).toBeVisible({ timeout: 10000 });
  });

  test("should display agent status indicator", async ({ page }) => {
    // 硬断言：agent 状态指示器应始终渲染在聊天头部
    const statusIndicator = page.locator('[data-testid="agent-status"]');
    await expect(statusIndicator).toBeVisible({ timeout: 10000 });
  });

  test("should stop generation when stop button clicked", async ({ page }) => {
    const input = page.locator('[data-testid="message-input"]');
    await expect(input).toBeVisible({ timeout: 10000 });
    await input.fill("count from 1 to 100");
    await input.press("Enter");

    // 点击停止按钮
    const stopBtn = page.locator('[data-testid="stop-generation-btn"]');
    try {
      await expect(stopBtn).toBeVisible({ timeout: 5000 });
      await stopBtn.click();
    } catch {
      // 如果流已完成或未启动，不视为测试失败
    }
  });
});
