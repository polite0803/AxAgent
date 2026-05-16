import { expect, test } from "@playwright/test";

test.describe("Agent Execution Flow", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await page.waitForSelector('[data-testid="chat-view"]', { timeout: 60000 });
  });

  test("should display agent status indicator", async ({ page }) => {
    // S-25: 硬断言 — agent 状态指示器应始终在聊天头部渲染
    const statusIndicator = page.locator('[data-testid="agent-status"]');
    await expect(statusIndicator).toBeVisible({ timeout: 10000 });
  });

  test("should have chat input area enabled", async ({ page }) => {
    // S-25: 硬断言 — 输入区域是核心 UI
    const input = page.locator('[data-testid="message-input"]');
    await expect(input).toBeVisible({ timeout: 15000 });
    await expect(input).toBeEnabled();
  });
});
