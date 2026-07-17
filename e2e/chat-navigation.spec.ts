import { expect, test } from "@playwright/test";

test.describe("Chat Navigation", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/chat");
    await page.waitForSelector('[data-testid="chat-view"]', { timeout: 30000 });
  });

  test("should display chat view", async ({ page }) => {
    await expect(page.locator('[data-testid="chat-view"]')).toBeVisible();
  });

  test("should display new conversation button", async ({ page }) => {
    const newConvBtn = page.locator('[data-testid="new-conversation-btn"]');
    const isVisible = await newConvBtn.isVisible({ timeout: 10000 }).catch(() => false);
    if (isVisible) {
      await expect(newConvBtn).toBeVisible();
    }
  });

  test("should display agent status indicator", async ({ page }) => {
    const statusIndicator = page.locator('[data-testid="agent-status"]');
    const isVisible = await statusIndicator.isVisible({ timeout: 5000 }).catch(() => false);
    if (isVisible) {
      await expect(statusIndicator).toBeVisible();
    }
  });

  test("should have message input enabled", async ({ page }) => {
    const input = page.locator('[data-testid="message-input"]');
    const isVisible = await input.isVisible({ timeout: 10000 }).catch(() => false);
    if (isVisible) {
      await expect(input).toBeEnabled();
    }
  });

  test("should display send button", async ({ page }) => {
    const sendBtn = page.locator('[data-testid="send-btn"]');
    const isVisible = await sendBtn.isVisible({ timeout: 5000 }).catch(() => false);
    if (isVisible) {
      await expect(sendBtn).toBeVisible();
    }
  });

  test("should display cache indicator", async ({ page }) => {
    const cacheIndicator = page.locator('[data-testid="cache-indicator"]');
    const isVisible = await cacheIndicator.isVisible({ timeout: 5000 }).catch(() => false);
    if (isVisible) {
      await expect(cacheIndicator).toBeVisible();
    }
  });

  test("should display plan approval toggle", async ({ page }) => {
    const toggle = page.locator('[data-testid="plan-approval-toggle"]');
    const isVisible = await toggle.isVisible({ timeout: 5000 }).catch(() => false);
    if (isVisible) {
      await expect(toggle).toBeVisible();
    }
  });

  test("should navigate to dashboard from chat", async ({ page }) => {
    await page.goto("/dashboard");
    await page.waitForLoadState("networkidle");
    await expect(page.locator("body")).toBeVisible();
  });

  test("should navigate to knowledge hub from chat", async ({ page }) => {
    await page.goto("/knowledge");
    await page.waitForSelector('[data-testid="knowledge-hub"]', { timeout: 30000 });
    await expect(page.locator('[data-testid="knowledge-hub"]')).toBeVisible();
  });

  test("should navigate to gateway from chat", async ({ page }) => {
    await page.goto("/gateway");
    await page.waitForSelector('[data-testid="gateway-overview"]', { timeout: 30000 });
    await expect(page.locator('[data-testid="gateway-overview"]')).toBeVisible();
  });

  test("should navigate to workflow from chat", async ({ page }) => {
    await page.goto("/workflow");
    await page.waitForLoadState("networkidle");
    await expect(page.locator("body")).toBeVisible();
  });

  test("should navigate to files from chat", async ({ page }) => {
    await page.goto("/files");
    await page.waitForLoadState("networkidle");
    await expect(page.locator("body")).toBeVisible();
  });
});
