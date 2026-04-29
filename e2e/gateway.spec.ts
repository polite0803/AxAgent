import { expect, test } from "@playwright/test";

test.describe("Gateway Management E2E", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/gateway");
    await page.waitForSelector('[data-testid="gateway-overview"]', { timeout: 60000 });
  });

  test("should display gateway overview page", async ({ page }) => {
    await expect(page.locator('[data-testid="gateway-overview"]')).toBeVisible();
  });

  test("should show gateway connection status", async ({ page }) => {
    await expect(page.locator('[data-testid="gateway-status"]')).toBeVisible();
  });

  test("should display gateway metrics", async ({ page }) => {
    await expect(page.locator('[data-testid="gateway-metrics"]')).toBeVisible();
  });

  test.skip("should navigate to gateway diagnostics", async ({ page }) => {
    // Click the "日志" tab — tabs content is lazily rendered by antd
    const diagnosticsTab = page.locator('.ant-tabs-tab').filter({ hasText: '日志' }).first();
    await diagnosticsTab.click();
    // Tab content is rendered inside .ant-tabs-content — verify it's not empty
    await expect(page.locator('.ant-tabs-tabpane-active')).toBeVisible({ timeout: 10000 });
  });
});
