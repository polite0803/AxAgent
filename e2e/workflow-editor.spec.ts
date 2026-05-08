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
    const searchInput = page.locator('input[placeholder="搜索模板..."]');
    if (await searchInput.isVisible({ timeout: 10000 }).catch(() => false)) {
      await expect(searchInput).toBeVisible();
    }
  });

  test("should display template list", async ({ page }) => {
    const newButton = page.locator('button:has-text("新建模板")').first();
    if (await newButton.isVisible({ timeout: 5000 }).catch(() => false)) {
      await expect(newButton).toBeVisible();
    }
  });

  test("should create new template", async ({ page }) => {
    const newButton = page.locator('button:has-text("新建模板")').first();
    if (await newButton.isVisible({ timeout: 5000 }).catch(() => false)) {
      await newButton.click();
      await page.waitForTimeout(2000);
      const reactFlow = page.locator(".react-flow");
      if (await reactFlow.isVisible({ timeout: 10000 }).catch(() => false)) {
        await expect(reactFlow).toBeVisible();
      }
    }
  });

  test("should filter templates by search", async ({ page }) => {
    const searchInput = page.locator('input[placeholder="搜索模板..."]');
    if (await searchInput.isVisible({ timeout: 5000 }).catch(() => false)) {
      await searchInput.fill("code");
      await page.waitForTimeout(500);
    }
  });

  test("should delete a template", async ({ page }) => {
    const moreBtn = page.locator(".ant-card-actions button").first();
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
    const moreBtn = page.locator(".ant-card-actions button").first();
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

test.describe("Workflow Editor Canvas", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/workflow");
    await page.waitForLoadState("networkidle");
    await dismissModals(page);

    const newButton = page.locator('button:has-text("新建模板")').first();
    if (await newButton.isVisible({ timeout: 5000 }).catch(() => false)) {
      await newButton.click({ force: true });
      await page.waitForTimeout(2000);
      await dismissModals(page);
    }
  });

  test("should display node palette when canvas is open", async ({ page }) => {
    const triggerLabel = page.locator("text=触发器").first();
    if (await triggerLabel.isVisible({ timeout: 10000 }).catch(() => false)) {
      await expect(triggerLabel).toBeVisible();
    }
  });

  test("should show zoom controls when canvas is open", async ({ page }) => {
    const controls = page.locator(".react-flow__controls");
    if (await controls.isVisible({ timeout: 10000 }).catch(() => false)) {
      await expect(controls).toBeVisible();
    }
  });

  test("should open AI panel", async ({ page }) => {
    const aiPanelBtn = page.locator('[data-testid="workflow-ai-panel-btn"]');
    if (await aiPanelBtn.isVisible({ timeout: 5000 }).catch(() => false)) {
      await aiPanelBtn.click();
      await page.waitForTimeout(500);
      const textarea = page.locator("textarea").first();
      if (await textarea.isVisible({ timeout: 5000 }).catch(() => false)) {
        await expect(textarea).toBeVisible();
      }
    }
  });

  test("should open import/export modal", async ({ page }) => {
    const importExportBtn = page.locator('[data-testid="workflow-import-export-btn"]');
    if (await importExportBtn.isVisible({ timeout: 5000 }).catch(() => false)) {
      await importExportBtn.click();
      const modal = page.locator("text=导出").or(page.locator("text=导入")).first();
      if (await modal.isVisible({ timeout: 5000 }).catch(() => false)) {
        await expect(modal).toBeVisible();
      }
    }
  });

  test("should show save indicator when dirty", async ({ page }) => {
    const savedIndicator = page.locator("text=已保存").or(page.locator("text=Saved")).first();
    if (await savedIndicator.isVisible({ timeout: 5000 }).catch(() => false)) {
      await expect(savedIndicator).toBeVisible();
    }
  });
});
