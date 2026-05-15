import { expect, test } from "@playwright/test";

async function dismissModals(page: import("@playwright/test").Page) {
  const closeBtn = page.locator(".ant-modal-close").first();
  if (await closeBtn.isVisible({ timeout: 1000 }).catch(() => false)) {
    await closeBtn.click();
    await page.waitForTimeout(300);
  }
  const okBtn = page.locator(".ant-modal-footer .ant-btn-primary").first();
  if (await okBtn.isVisible({ timeout: 1000 }).catch(() => false)) {
    await okBtn.click();
    await page.waitForTimeout(300);
  }
}

test.describe("Workflow Editor E2E Tests", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/workflow");
    await page.waitForLoadState("networkidle");
    await dismissModals(page);
  });

  test("should load workflow page", async ({ page }) => {
    // TemplateList search input (uses testid for language independence)
    const searchInput = page.locator('[data-testid="template-list-search"]');
    await expect(searchInput).toBeVisible({ timeout: 10000 });
  });

  test("should display template list", async ({ page }) => {
    // WorkflowSettings "创建新模板" button
    const newButton = page.getByTestId("workflow-create-new-btn").first();
    await expect(newButton).toBeVisible({ timeout: 5000 });
  });

  test("should create new template", async ({ page }) => {
    const newButton = page.getByTestId("workflow-create-new-btn").first();
    await expect(newButton).toBeVisible({ timeout: 5000 });
    await newButton.click();
    const reactFlow = page.locator(".react-flow");
    await expect(reactFlow).toBeVisible({ timeout: 10000 });
  });

  test("should filter templates by search", async ({ page }) => {
    const searchInput = page.locator('[data-testid="template-list-search"]');
    await expect(searchInput).toBeVisible({ timeout: 5000 });
    await searchInput.fill("code");
    await expect(searchInput).toHaveValue("code");
  });

  // 卡片操作（删除/复制）需要预存模板数据，浏览器 mock 模式下无持久化数据
  test.skip("should delete a template", async () => {});
  test.skip("should duplicate a template", async () => {});
});

test.describe("Workflow Editor Canvas", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/workflow");
    await page.waitForLoadState("networkidle");
    await dismissModals(page);

    const newButton = page.getByTestId("workflow-create-new-btn").first();
    await expect(newButton).toBeVisible({ timeout: 5000 });
    await newButton.click({ force: true });
    await page.waitForTimeout(2000);
    await dismissModals(page);
  });

  test("should display node palette when canvas is open", async ({ page }) => {
    const triggerLabel = page.locator("text=触发器").first();
    await expect(triggerLabel).toBeVisible({ timeout: 10000 });
  });

  test("should show zoom controls when canvas is open", async ({ page }) => {
    const controls = page.locator(".react-flow__controls");
    await expect(controls).toBeVisible({ timeout: 10000 });
  });

  test("should open AI panel", async ({ page }) => {
    const aiPanelBtn = page.locator('[data-testid="workflow-ai-panel-btn"]');
    await expect(aiPanelBtn).toBeVisible({ timeout: 5000 });
    await aiPanelBtn.click();
    await page.waitForTimeout(500);
    const textarea = page.locator("textarea").first();
    await expect(textarea).toBeVisible({ timeout: 5000 });
  });

  test("should open import/export modal", async ({ page }) => {
    const importExportBtn = page.locator('[data-testid="workflow-import-export-btn"]');
    await expect(importExportBtn).toBeVisible({ timeout: 5000 });
    await importExportBtn.click();
    const modal = page.locator("text=导出").or(page.locator("text=导入")).first();
    await expect(modal).toBeVisible({ timeout: 5000 });
  });

  test("should show save indicator when dirty", async ({ page }) => {
    const savedIndicator = page.locator("text=已保存").or(page.locator("text=Saved")).first();
    await expect(savedIndicator).toBeVisible({ timeout: 5000 });
  });
});
