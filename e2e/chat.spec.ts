import { expect, test } from "@playwright/test";

test.describe("Chat Flow", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await page.waitForSelector('[data-testid="chat-view"]', { timeout: 60000 });
  });

  test("should display chat interface", async ({ page }) => {
    // Check that the chat view is visible
    await expect(page.locator('[data-testid="chat-view"]')).toBeVisible();

    // Check that the input area is present
    await expect(page.locator('[data-testid="message-input"]')).toBeVisible();
  });

  test("should create a new conversation and send a message", async ({ page }) => {
    // Click new conversation button
    const newConvBtn = page.locator('[data-testid="new-conversation-btn"]');
    if (await newConvBtn.isVisible()) {
      await newConvBtn.click();
    }

    // Type a message
    const input = page.locator('[data-testid="message-input"]');
    await input.fill("Hello, this is a test message");

    // Send the message
    const sendBtn = page.locator('[data-testid="send-btn"]');
    await sendBtn.click();

    // Wait for response (or at least for the message to appear)
    await page.waitForTimeout(2000);

    // Verify the message appears in the chat
    await expect(page.locator("text=Hello, this is a test message")).toBeVisible();
  });

  test("should navigate to settings page", async ({ page }) => {
    // Click settings icon in sidebar
    const settingsBtn = page.locator('[data-testid="settings-nav-btn"]');
    await settingsBtn.click();

    // Verify we're on settings page
    await expect(page).toHaveURL(/.*settings.*/);
    await expect(page.locator('[data-testid="settings-panel"]')).toBeVisible();
  });
});

test.describe("Settings", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/settings");
    await page.waitForSelector('[data-testid="settings-panel"]', { timeout: 30000 });
  });

  test("should display settings sections", async ({ page }) => {
    // Check that settings sidebar is visible
    await expect(page.locator('[data-testid="settings-sidebar"]')).toBeVisible();
  });

  test("should save theme preference", async ({ page }) => {
    // Navigate to appearance settings
    const appearanceBtn = page.locator("text=Appearance");
    if (await appearanceBtn.isVisible()) {
      await appearanceBtn.click();
    }

    // Toggle dark mode
    const darkModeToggle = page.locator('[data-testid="dark-mode-toggle"]');
    if (await darkModeToggle.isVisible()) {
      await darkModeToggle.click();

      // Wait for save
      await page.waitForTimeout(1000);
    }
  });
});
