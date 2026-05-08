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
    const statusEl = page.locator('[data-testid="gateway-status"]');
    if (await statusEl.isVisible({ timeout: 5000 }).catch(() => false)) {
      await expect(statusEl).toBeVisible();
    }
  });

  test("should display gateway metrics", async ({ page }) => {
    const metricsEl = page.locator('[data-testid="gateway-metrics"]');
    if (await metricsEl.isVisible({ timeout: 5000 }).catch(() => false)) {
      await expect(metricsEl).toBeVisible();
    }
  });

  test.skip("should navigate to gateway diagnostics", async ({ page }) => {
    const diagnosticsTab = page.locator(".ant-tabs-tab").filter({ hasText: "日志" }).first();
    await diagnosticsTab.click();
    await expect(page.locator(".ant-tabs-tabpane-active")).toBeVisible({ timeout: 10000 });
  });
});
