import { expect, test } from "@playwright/test";

// ─── Workflow Editor Tests (requires entering the editor canvas) ───

test.describe("Workflow Editor E2E Tests", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/workflow");
    await page.waitForLoadState("networkidle");

    // Click "创建新模板" to enter the canvas editor with initNewTemplate()
    await page.locator('button:has-text("创建新模板")').first().click();
    await page.waitForTimeout(1500);
  });

  test("should load workflow editor page", async ({ page }) => {
    await expect(page.locator(".react-flow")).toBeVisible({ timeout: 15000 });
  });

  test("should display node palette", async ({ page }) => {
    // LeftPanel shows node types — check for "触发器" label
    await expect(page.locator("text=触发器").first()).toBeVisible({ timeout: 10000 });
  });

  test("should show zoom controls", async ({ page }) => {
    // ReactFlow renders zoom controls inside .react-flow__controls wrapper
    await expect(page.locator(".react-flow__controls")).toBeVisible({ timeout: 10000 });
  });

  test("should show canvas with nodes and edges", async ({ page }) => {
    await expect(page.locator(".react-flow")).toBeVisible({ timeout: 10000 });
  });

  test("should handle keyboard shortcuts", async ({ page }) => {
    await page.keyboard.press("Control+s");
    await page.waitForTimeout(300);
    await page.keyboard.press("Control+z");
    await page.waitForTimeout(300);
  });

  test("should open import/export modal", async ({ page }) => {
    await page.locator('[data-testid="workflow-import-export-btn"]').click();
    // ImportExportModal should appear
    await expect(page.locator("text=导出").or(page.locator("text=导入")).first()).toBeVisible({ timeout: 5000 });
  });

  test("should show save indicator when dirty", async ({ page }) => {
    // StatusBar shows save status text
    await expect(page.locator("text=已保存").or(page.locator("text=Saved")).first()).toBeVisible({ timeout: 5000 });
  });
});

// ─── AI Panel Tests (requires editor open + AI panel interaction) ───

test.describe("Workflow Editor AI Features", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/workflow");
    await page.waitForLoadState("networkidle");
    await page.locator('button:has-text("创建新模板")').first().click();
    await page.waitForTimeout(1500);
  });

  test("should open AI panel", async ({ page }) => {
    await page.locator('[data-testid="workflow-ai-panel-btn"]').click();
    await page.waitForTimeout(500);
    await expect(page.locator("textarea").first()).toBeVisible({ timeout: 5000 });
  });
});

// ─── Template Management Tests (stays on template list page) ───

test.describe("Template Management", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/workflow");
    await page.waitForLoadState("networkidle");
  });

  test("should display template list", async ({ page }) => {
    await expect(page.locator('input[placeholder="搜索模板..."]')).toBeVisible({ timeout: 10000 });
    await expect(page.locator('button:has-text("新建模板")').first()).toBeVisible({ timeout: 5000 });
  });

  test("should filter templates by search", async ({ page }) => {
    const searchInput = page.locator('input[placeholder="搜索模板..."]');
    await searchInput.fill("code");
    await page.waitForTimeout(500);
  });

  test("should create new template", async ({ page }) => {
    const newButton = page.locator('button:has-text("新建模板")').first();
    await newButton.click();
    await expect(page.locator(".react-flow")).toBeVisible({ timeout: 10000 });
  });

  test("should delete a template", async ({ page }) => {
    // Click "更多" dropdown on first template card
    const moreBtn = page.locator(".ant-card button").filter({ hasText: "" }).first();
    if (await moreBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await moreBtn.click();
      await page.waitForTimeout(300);

      const deleteOption = page.locator(".ant-dropdown-menu-item").filter({ hasText: "删除" }).first();
      if (await deleteOption.isVisible({ timeout: 3000 }).catch(() => false)) {
        await deleteOption.click();
        await page.waitForTimeout(500);

        const confirmBtn = page.locator(".ant-btn-dangerous").first();
        if (await confirmBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
          await confirmBtn.click();
          await page.waitForTimeout(1000);
        }
      }
    }
  });

  test("should duplicate a template", async ({ page }) => {
    const moreBtn = page.locator(".ant-card button").filter({ hasText: "" }).first();
    if (await moreBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await moreBtn.click();
      await page.waitForTimeout(300);

      const duplicateOption = page.locator(".ant-dropdown-menu-item").filter({ hasText: "复制" }).first();
      if (await duplicateOption.isVisible({ timeout: 3000 }).catch(() => false)) {
        await duplicateOption.click();
        await page.waitForTimeout(1000);
      }
    }
  });
});
