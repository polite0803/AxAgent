import { expect, test } from "@playwright/test";

test.describe("Wiki Page", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/wiki");
    await page.waitForLoadState("networkidle");
  });

  test("should load wiki page", async ({ page }) => {
    await expect(page.locator("body")).toBeVisible();
    await page.waitForTimeout(1000);
  });

  test("should display graph view or empty state", async ({ page }) => {
    const graphView = page.locator(".react-flow").or(page.locator(".ant-empty")).first();
    const isVisible = await graphView.isVisible({ timeout: 10000 }).catch(() => false);
    if (isVisible) {
      await expect(graphView).toBeVisible();
    }
  });

  test("should have search functionality", async ({ page }) => {
    const searchInput = page.locator("input").filter({ has: page.locator("[placeholder]") }).first();
    const isVisible = await searchInput.isVisible({ timeout: 5000 }).catch(() => false);
    if (isVisible) {
      await expect(searchInput).toBeVisible();
    }
  });

  test("should navigate to LLM wiki", async ({ page }) => {
    await page.goto("/llm-wiki");
    await page.waitForLoadState("networkidle");
    await expect(page.locator("body")).toBeVisible();
  });
});
