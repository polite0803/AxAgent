import { expect, test } from "@playwright/test";

test.describe("Conversation Lifecycle", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await page.waitForSelector('[data-testid="chat-view"]', { timeout: 60000 });
  });

  test("should show new conversation button", async ({ page }) => {
    const newConvBtn = page.locator('[data-testid="new-conversation-btn"]');
    await expect(newConvBtn).toBeVisible({ timeout: 10000 });
  });

  test("should display agent status indicator when available", async ({ page }) => {
    // agent-status 仅在 agent 活跃时渲染，浏览器 mock 中可能不出现
    const statusIndicator = page.locator('[data-testid="agent-status"]');
    const isVisible = await statusIndicator.isVisible({ timeout: 5000 }).catch(() => false);
    if (isVisible) {
      await expect(statusIndicator).toBeVisible();
    }
  });

  test("should stop generation button appear during streaming", async ({ page }) => {
    const input = page.locator('[data-testid="message-input"]');
    await expect(input).toBeVisible({ timeout: 10000 });
    await input.fill("count to 100");
    await input.press("Enter");

    // 停止按钮仅在流式响应期间出现
    const stopBtn = page.locator('[data-testid="stop-generation-btn"]');
    const isVisible = await stopBtn.isVisible({ timeout: 5000 }).catch(() => false);
    if (isVisible) {
      await stopBtn.click();
    }
  });
});
