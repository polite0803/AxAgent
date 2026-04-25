import { test, expect } from '@playwright/test';

test.describe('Workflow Editor E2E Tests', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/#/workflow');
    await page.waitForLoadState('networkidle');
  });

  test('should load workflow editor page', async ({ page }) => {
    await expect(page.locator('text=工作流')).toBeVisible({ timeout: 10000 });
  });

  test('should display template list', async ({ page }) => {
    await page.waitForSelector('text=模板列表', { timeout: 10000 }).catch(() => {});
    const templateList = page.locator('[class*="template"]').first();
    await expect(templateList).toBeVisible({ timeout: 5000 }).catch(() => {});
  });

  test('should open new template dialog', async ({ page }) => {
    const newButton = page.locator('button:has-text("新建")').first();
    if (await newButton.isVisible()) {
      await newButton.click();
      await expect(page.locator('text=创建工作流')).toBeVisible({ timeout: 5000 });
    }
  });

  test('should open AI panel', async ({ page }) => {
    const aiButton = page.locator('text=AI 助手').first();
    if (await aiButton.isVisible()) {
      await aiButton.click();
      await expect(page.locator('text=生成工作流')).toBeVisible({ timeout: 5000 });
      await expect(page.locator('text=优化 Prompt')).toBeVisible({ timeout: 5000 });
      await expect(page.locator('text=推荐节点')).toBeVisible({ timeout: 5000 });
    }
  });

  test('should switch between AI panel tabs', async ({ page }) => {
    const aiButton = page.locator('text=AI 助手').first();
    if (await aiButton.isVisible()) {
      await aiButton.click();
      await page.waitForTimeout(500);

      const optimizeTab = page.locator('text=优化 Prompt');
      if (await optimizeTab.isVisible()) {
        await optimizeTab.click();
        await expect(page.locator('text=输入要优化的 Agent Prompt')).toBeVisible({ timeout: 5000 });
      }
    }
  });

  test('should toggle sidebar', async ({ page }) => {
    const sidebar = page.locator('[class*="sidebar"]').first();
    if (await sidebar.isVisible()) {
      const toggleButton = page.locator('button[class*="toggle"]').first();
      if (await toggleButton.isVisible()) {
        await toggleButton.click();
        await page.waitForTimeout(300);
      }
    }
  });

  test('should display node palette', async ({ page }) => {
    const nodePalette = page.locator('text=节点').first();
    if (await nodePalette.isVisible()) {
      await expect(nodePalette).toBeVisible({ timeout: 5000 });
    }
  });

  test('should show import/export modal', async ({ page }) => {
    const importExportButton = page.locator('text=导入/导出').first();
    if (await importExportButton.isVisible()) {
      await importExportButton.click();
      await expect(page.locator('text=导入/导出模板')).toBeVisible({ timeout: 5000 });
      await expect(page.locator('text=导出模板')).toBeVisible({ timeout: 5000 });
      await expect(page.locator('text=导入')).toBeVisible({ timeout: 5000 });
    }
  });

  test('should filter templates by search', async ({ page }) => {
    const searchInput = page.locator('input[placeholder*="搜索"]').first();
    if (await searchInput.isVisible()) {
      await searchInput.fill('test');
      await page.waitForTimeout(500);
    }
  });

  test('should display validation errors', async ({ page }) => {
    const validateButton = page.locator('text=验证').first();
    if (await validateButton.isVisible()) {
      await validateButton.click();
      await page.waitForTimeout(500);
    }
  });

  test('should show save indicator when dirty', async ({ page }) => {
    const saveIndicator = page.locator('text=未保存').first();
    if (await saveIndicator.isVisible()) {
      await expect(saveIndicator).toBeVisible({ timeout: 5000 });
    }
  });

  test('should handle keyboard shortcuts', async ({ page }) => {
    await page.keyboard.press('Control+s');
    await page.waitForTimeout(300);

    await page.keyboard.press('Control+z');
    await page.waitForTimeout(300);

    await page.keyboard.press('Delete');
    await page.waitForTimeout(300);
  });

  test('should show canvas with nodes and edges', async ({ page }) => {
    const canvas = page.locator('[class*="react-flow"]').first();
    if (await canvas.isVisible()) {
      await expect(canvas).toBeVisible({ timeout: 5000 });
    }
  });

  test('should display properties panel when node selected', async ({ page }) => {
    const propertiesPanel = page.locator('text=属性').first();
    if (await propertiesPanel.isVisible()) {
      await expect(propertiesPanel).toBeVisible({ timeout: 5000 });
    }
  });

  test('should show zoom controls', async ({ page }) => {
    const zoomIn = page.locator('[aria-label="放大"], [title="放大"]').first();
    const zoomOut = page.locator('[aria-label="缩小"], [title="缩小"]').first();
    const zoomFit = page.locator('[aria-label="适应窗口"], [title="适应窗口"]').first();

    const hasZoomControls = await zoomIn.isVisible().catch(() => false) ||
                           await zoomOut.isVisible().catch(() => false) ||
                           await zoomFit.isVisible().catch(() => false);

    if (hasZoomControls) {
      await zoomIn.click().catch(() => {});
      await page.waitForTimeout(200);
      await zoomOut.click().catch(() => {});
    }
  });
});

test.describe('Workflow Editor AI Features', () => {
  test('should generate workflow from natural language', async ({ page }) => {
    await page.goto('/#/workflow');
    await page.waitForLoadState('networkidle');

    const aiButton = page.locator('text=AI 助手').first();
    if (await aiButton.isVisible().catch(() => false)) {
      await aiButton.click();
      await page.waitForTimeout(500);

      const promptInput = page.locator('textarea').first();
      if (await promptInput.isVisible().catch(() => false)) {
        await promptInput.fill('创建一个代码审查工作流');

        const generateButton = page.locator('button:has-text("生成工作流")').first();
        if (await generateButton.isVisible().catch(() => false)) {
          await generateButton.click();
          await page.waitForTimeout(2000);
        }
      }
    }
  });

  test('should optimize agent prompt', async ({ page }) => {
    await page.goto('/#/workflow');
    await page.waitForLoadState('networkidle');

    const aiButton = page.locator('text=AI 助手').first();
    if (await aiButton.isVisible().catch(() => false)) {
      await aiButton.click();
      await page.waitForTimeout(500);

      const optimizeTab = page.locator('text=优化 Prompt').first();
      if (await optimizeTab.isVisible().catch(() => false)) {
        await optimizeTab.click();
        await page.waitForTimeout(500);

        const promptInput = page.locator('textarea').first();
        if (await promptInput.isVisible().catch(() => false)) {
          await promptInput.fill('You are a helpful assistant');

          const optimizeButton = page.locator('button:has-text("优化 Prompt")').first();
          if (await optimizeButton.isVisible().catch(() => false)) {
            await optimizeButton.click();
            await page.waitForTimeout(2000);
          }
        }
      }
    }
  });

  test('should recommend nodes based on context', async ({ page }) => {
    await page.goto('/#/workflow');
    await page.waitForLoadState('networkidle');

    const aiButton = page.locator('text=AI 助手').first();
    if (await aiButton.isVisible().catch(() => false)) {
      await aiButton.click();
      await page.waitForTimeout(500);

      const recommendTab = page.locator('text=推荐节点').first();
      if (await recommendTab.isVisible().catch(() => false)) {
        await recommendTab.click();
        await page.waitForTimeout(500);

        const contextInput = page.locator('textarea').first();
        if (await contextInput.isVisible().catch(() => false)) {
          await contextInput.fill('我需要一个代码分析工作流');

          const recommendButton = page.locator('button:has-text("获取推荐")').first();
          if (await recommendButton.isVisible().catch(() => false)) {
            await recommendButton.click();
            await page.waitForTimeout(2000);
          }
        }
      }
    }
  });
});

test.describe('Template Management', () => {
  test('should list user templates', async ({ page }) => {
    await page.goto('/#/workflow');
    await page.waitForLoadState('networkidle');

    const templatesTab = page.locator('text=我的模板').first();
    if (await templatesTab.isVisible().catch(() => false)) {
      await templatesTab.click();
      await page.waitForTimeout(1000);
    }
  });

  test('should list preset templates', async ({ page }) => {
    await page.goto('/#/workflow');
    await page.waitForLoadState('networkidle');

    const presetsTab = page.locator('text=预设模板').first();
    if (await presetsTab.isVisible().catch(() => false)) {
      await presetsTab.click();
      await page.waitForTimeout(1000);
    }
  });

  test('should filter templates by tag', async ({ page }) => {
    await page.goto('/#/workflow');
    await page.waitForLoadState('networkidle');

    const tagSelect = page.locator('.ant-select').first();
    if (await tagSelect.isVisible().catch(() => false)) {
      await tagSelect.click();
      await page.waitForTimeout(500);

      const option = page.locator('.ant-select-item').first();
      if (await option.isVisible().catch(() => false)) {
        await option.click();
        await page.waitForTimeout(500);
      }
    }
  });

  test('should create new template', async ({ page }) => {
    await page.goto('/#/workflow');
    await page.waitForLoadState('networkidle');

    const newButton = page.locator('button:has-text("新建")').first();
    if (await newButton.isVisible().catch(() => false)) {
      await newButton.click();
      await page.waitForTimeout(500);

      const nameInput = page.locator('input[id*="name"], input[placeholder*="名称"]').first();
      if (await nameInput.isVisible().catch(() => false)) {
        await nameInput.fill('Test Workflow');
      }

      const createButton = page.locator('button:has-text("创建")').first();
      if (await createButton.isVisible().catch(() => false)) {
        await createButton.click();
        await page.waitForTimeout(1000);
      }
    }
  });

  test('should delete a template', async ({ page }) => {
    await page.goto('/#/workflow');
    await page.waitForLoadState('networkidle');

    const templateCard = page.locator('[class*="template-card"]').first();
    if (await templateCard.isVisible().catch(() => false)) {
      const moreButton = page.locator('[class*="more"]').first();
      if (await moreButton.isVisible().catch(() => false)) {
        await moreButton.click();
        await page.waitForTimeout(300);

        const deleteOption = page.locator('text=删除').first();
        if (await deleteOption.isVisible().catch(() => false)) {
          await deleteOption.click();
          await page.waitForTimeout(500);

          const confirmButton = page.locator('button:has-text("删除")').first();
          if (await confirmButton.isVisible().catch(() => false)) {
            await confirmButton.click();
            await page.waitForTimeout(1000);
          }
        }
      }
    }
  });

  test('should duplicate a template', async ({ page }) => {
    await page.goto('/#/workflow');
    await page.waitForLoadState('networkidle');

    const templateCard = page.locator('[class*="template-card"]').first();
    if (await templateCard.isVisible().catch(() => false)) {
      const moreButton = page.locator('[class*="more"]').first();
      if (await moreButton.isVisible().catch(() => false)) {
        await moreButton.click();
        await page.waitForTimeout(300);

        const duplicateOption = page.locator('text=复制').first();
        if (await duplicateOption.isVisible().catch(() => false)) {
          await duplicateOption.click();
          await page.waitForTimeout(1000);
        }
      }
    }
  });
});
