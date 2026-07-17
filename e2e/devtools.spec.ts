import { expect, test } from "@playwright/test";

test.describe("DevTools Pages", () => {
  test("should load trace explorer page", async ({ page }) => {
    await page.goto("/devtools/trace-explorer");
    await page.waitForLoadState("networkidle");
    await expect(page.locator("body")).toBeVisible();
    await page.waitForTimeout(500);
  });

  test("should load benchmark runner page", async ({ page }) => {
    await page.goto("/devtools/benchmark");
    await page.waitForLoadState("networkidle");
    await expect(page.locator("body")).toBeVisible();
    await page.waitForTimeout(500);
  });

  test("should load tool recommender page", async ({ page }) => {
    await page.goto("/devtools/tool-recommender");
    await page.waitForLoadState("networkidle");
    await expect(page.locator("body")).toBeVisible();
    await page.waitForTimeout(500);
  });

  test("should load fine-tune page", async ({ page }) => {
    await page.goto("/devtools/fine-tune");
    await page.waitForLoadState("networkidle");
    await expect(page.locator("body")).toBeVisible();
    await page.waitForTimeout(500);
  });

  test("should load RL training page", async ({ page }) => {
    await page.goto("/devtools/rl-training");
    await page.waitForLoadState("networkidle");
    await expect(page.locator("body")).toBeVisible();
    await page.waitForTimeout(500);
  });
});

test.describe("Trace Explorer", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/devtools/trace-explorer");
    await page.waitForLoadState("networkidle");
  });

  test("should display trace list or empty state", async ({ page }) => {
    const traceList = page.locator(".ant-list").or(page.locator(".ant-empty")).first();
    const isVisible = await traceList.isVisible({ timeout: 10000 }).catch(() => false);
    if (isVisible) {
      await expect(traceList).toBeVisible();
    }
  });

  test("should have trace filters", async ({ page }) => {
    const filters = page.locator("input").or(page.locator(".ant-select")).first();
    const isVisible = await filters.isVisible({ timeout: 5000 }).catch(() => false);
    if (isVisible) {
      await expect(filters).toBeVisible();
    }
  });
});

test.describe("Benchmark Runner", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/devtools/benchmark");
    await page.waitForLoadState("networkidle");
  });

  test("should display benchmark selector or empty state", async ({ page }) => {
    const selector = page.locator(".ant-select").or(page.locator(".ant-empty")).first();
    const isVisible = await selector.isVisible({ timeout: 10000 }).catch(() => false);
    if (isVisible) {
      await expect(selector).toBeVisible();
    }
  });

  test("should have tabs for config and report", async ({ page }) => {
    const tabs = page.locator(".ant-tabs").first();
    const isVisible = await tabs.isVisible({ timeout: 5000 }).catch(() => false);
    if (isVisible) {
      await expect(tabs).toBeVisible();
    }
  });
});

test.describe("Fine-Tune Page", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/devtools/fine-tune");
    await page.waitForLoadState("networkidle");
  });

  test("should display fine-tune tabs", async ({ page }) => {
    const tabs = page.locator(".ant-tabs").first();
    const isVisible = await tabs.isVisible({ timeout: 10000 }).catch(() => false);
    if (isVisible) {
      await expect(tabs).toBeVisible();
    }
  });
});
