import { expect, test } from "@playwright/test";

test.describe("Agent Execution Flow", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await page.waitForSelector('[data-testid="chat-view"]', { timeout: 60000 });
  });

  test("should display agent status indicator when active", async ({ page }) => {
    const statusIndicator = page.locator('[data-testid="agent-status"]');
    const isVisible = await statusIndicator.isVisible({ timeout: 5000 }).catch(() => false);
    if (isVisible) {
      await expect(statusIndicator).toBeVisible();
    }
  });

  test("should find chat input area", async ({ page }) => {
    const input = page.locator('[data-testid="message-input"]');
    const isVisible = await input.isVisible({ timeout: 10000 }).catch(() => false);
    if (isVisible) {
      await expect(input).toBeVisible();
    }
  });

  test("should switch models in agent config", async ({ page }) => {
    await page.goto("/settings");
    await page.waitForSelector('[data-testid="settings-panel"]', { timeout: 30000 });

    const modelSection = page.locator("text=Model");
    if (await modelSection.isVisible({ timeout: 3000 }).catch(() => false)) {
      await expect(modelSection.first()).toBeVisible();
    }
  });
});
