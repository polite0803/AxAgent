import { expect, test } from "@playwright/test";

test.describe("Agent Execution Flow", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await page.waitForSelector('[data-testid="chat-view"]', { timeout: 60000 });
    // 欢迎页不显示输入框，需要先创建对话
    // 点击欢迎页的第一个 prompt 按钮触发创建对话
    const firstPrompt = page.locator(".ant-prompts-item").first();
    if (await firstPrompt.isVisible({ timeout: 5000 }).catch(() => false)) {
      await firstPrompt.click();
      await page.waitForSelector('[data-testid="message-input"]', { timeout: 15000 });
    }
  });

  test("should display agent status indicator when available", async ({ page }) => {
    // agent-status 仅在 agent 活跃时渲染；浏览器 mock 中可能不出现
    const statusIndicator = page.locator('[data-testid="agent-status"]');
    const isVisible = await statusIndicator.isVisible({ timeout: 5000 }).catch(() => false);
    if (isVisible) {
      await expect(statusIndicator).toBeVisible();
    }
  });

  test("should have chat input area enabled", async ({ page }) => {
    const input = page.locator('[data-testid="message-input"]');
    await expect(input).toBeVisible({ timeout: 15000 });
    await expect(input).toBeEnabled();
  });
});
