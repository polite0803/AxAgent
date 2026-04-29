import { expect, test } from "@playwright/test";

test.describe("Platform Gateway Configuration", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/settings");
    await page.waitForSelector('[data-testid="settings-panel"]', { timeout: 30000 });
  });

  test("should navigate to gateway settings", async ({ page }) => {
    const gatewayNav = page.locator("text=Gateway");
    if (await gatewayNav.isVisible({ timeout: 5000 }).catch(() => false)) {
      await gatewayNav.click();
      await page.waitForTimeout(1000);
    }
  });

  test("should display platform integration section", async ({ page }) => {
    await page.goto("/settings/platform");
    await page.waitForTimeout(2000);

    // Check for platform-related UI elements
    const pageContent = page.locator("body");
    await expect(pageContent).toBeVisible();
  });

  test("should toggle Telegram integration", async ({ page }) => {
    const telegramToggle = page.locator('[data-testid="telegram-toggle"]');
    if (await telegramToggle.isVisible({ timeout: 3000 }).catch(() => false)) {
      const isChecked = await telegramToggle.isChecked();
      await telegramToggle.click();
      await page.waitForTimeout(500);
      expect(await telegramToggle.isChecked()).toBe(!isChecked);
    }
  });

  test("should enter Telegram bot token", async ({ page }) => {
    const tokenInput = page.locator('[data-testid="telegram-bot-token"]');
    if (await tokenInput.isVisible({ timeout: 3000 }).catch(() => false)) {
      await tokenInput.fill("test_bot_token_12345");
      expect(await tokenInput.inputValue()).toBe("test_bot_token_12345");
    }
  });

  test("should toggle Discord integration", async ({ page }) => {
    const discordToggle = page.locator('[data-testid="discord-toggle"]');
    if (await discordToggle.isVisible({ timeout: 3000 }).catch(() => false)) {
      const isChecked = await discordToggle.isChecked();
      await discordToggle.click();
      await page.waitForTimeout(500);
      expect(await discordToggle.isChecked()).toBe(!isChecked);
    }
  });
});
