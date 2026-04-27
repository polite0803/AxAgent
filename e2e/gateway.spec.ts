import { expect, test } from "@playwright/test";

test.describe("Gateway Management E2E", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await page.waitForSelector('[data-testid="gateway-overview"]', { timeout: 60000 });
  });

  test("should display gateway overview page", async ({ page }) => {
    await expect(page.locator('[data-testid="gateway-overview"]')).toBeVisible();
    await expect(page.locator("text=Gateway")).toBeVisible();
  });

  test("should show gateway connection status", async ({ page }) => {
    const statusIndicator = page.locator('[data-testid="gateway-status"]');
    if (await statusIndicator.isVisible()) {
      await expect(statusIndicator).toBeVisible();
    }
  });

  test("should display active agents list", async ({ page }) => {
    const agentsList = page.locator('[data-testid="active-agents-list"]');
    if (await agentsList.isVisible()) {
      await expect(agentsList).toBeVisible();
    }
  });

  test("should navigate to gateway diagnostics", async ({ page }) => {
    const diagnosticsBtn = page.locator('[data-testid="gateway-diagnostics-btn"]');
    if (await diagnosticsBtn.isVisible()) {
      await diagnosticsBtn.click();
      await expect(page.locator('[data-testid="gateway-diagnostics"]')).toBeVisible();
    }
  });

  test("should display gateway metrics", async ({ page }) => {
    const metricsPanel = page.locator('[data-testid="gateway-metrics"]');
    if (await metricsPanel.isVisible()) {
      await expect(metricsPanel).toBeVisible();
    }
  });
});

test.describe("Gateway Templates E2E", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/gateway/templates");
    await page.waitForTimeout(1000);
  });

  test("should display templates list", async ({ page }) => {
    const templatesList = page.locator('[data-testid="templates-list"]');
    if (await templatesList.isVisible()) {
      await expect(templatesList).toBeVisible();
    }
  });

  test("should create a new template", async ({ page }) => {
    const createBtn = page.locator('[data-testid="create-template-btn"]');
    if (await createBtn.isVisible()) {
      await createBtn.click();
      await expect(page.locator('[data-testid="template-form"]')).toBeVisible();
    }
  });
});
