# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: workflow-editor.spec.ts >> Workflow Editor E2E Tests >> should load workflow editor page
- Location: e2e\workflow-editor.spec.ts:9:3

# Error details

```
Error: expect(locator).toBeVisible() failed

Locator: locator('text=工作流')
Expected: visible
Timeout: 10000ms
Error: element(s) not found

Call log:
  - Expect "toBeVisible" with timeout 10000ms
  - waiting for locator('text=工作流')

```

# Page snapshot

```yaml
- generic [ref=e4]:
  - generic [ref=e5]:
    - generic [ref=e6]:
      - img "AxAgent" [ref=e7]
      - generic [ref=e8]: AxAgent
    - generic [ref=e10]:
      - button [ref=e11] [cursor=pointer]:
        - img [ref=e12]
      - button [ref=e16] [cursor=pointer]:
        - img [ref=e17]
      - button [ref=e19] [cursor=pointer]:
        - img [ref=e20]
      - button [ref=e23] [cursor=pointer]:
        - img [ref=e24]
      - button [ref=e27] [cursor=pointer]:
        - img [ref=e28]
      - button [ref=e33] [cursor=pointer]:
        - img [ref=e34]
      - button [ref=e37] [cursor=pointer]:
        - img [ref=e38]
  - generic [ref=e41]:
    - complementary [ref=e42]:
      - generic [ref=e44]:
        - navigation [ref=e45]:
          - button [ref=e46]:
            - img [ref=e47]
          - button [ref=e49]:
            - img [ref=e50]
          - button [ref=e53]:
            - img [ref=e54]
          - button [ref=e56]:
            - img [ref=e57]
          - button [ref=e65]:
            - img [ref=e66]
          - button [ref=e69]:
            - img [ref=e70]
          - button [ref=e74]:
            - img [ref=e75]
        - button [ref=e77]:
          - img [ref=e79] [cursor=pointer]
    - main [ref=e82]:
      - generic [ref=e83]:
        - generic [ref=e85]:
          - generic [ref=e86]:
            - generic [ref=e87]:
              - button [ref=e88] [cursor=pointer]:
                - img [ref=e90]
              - button [ref=e93] [cursor=pointer]:
                - img [ref=e95]
              - button [ref=e98] [cursor=pointer]:
                - img [ref=e100]
              - button [ref=e102] [cursor=pointer]:
                - img [ref=e104]
            - button [ref=e107] [cursor=pointer]:
              - img [ref=e109]
          - generic [ref=e115]:
            - img "No data" [ref=e117]
            - generic [ref=e123]: 暂无对话
        - generic [ref=e125]:
          - generic [ref=e126]:
            - generic [ref=e127]: 开始新的对话
            - generic [ref=e128] [cursor=pointer]:
              - generic "OpenAI" [ref=e129]:
                - img "OpenAI" [ref=e130]
              - generic [ref=e132]: OpenAI
              - generic [ref=e133]: gpt-4o
          - generic [ref=e135]:
            - heading "👋 下午好，今天想聊聊什么呢？" [level=3] [ref=e136]
            - generic [ref=e138]:
              - generic [ref=e139]:
                - img [ref=e141]
                - heading "编程（调试/审查）" [level=6] [ref=e145]
              - generic [ref=e146]:
                - img [ref=e148]
                - heading "创意写作" [level=6] [ref=e151]
              - generic [ref=e152]:
                - img [ref=e154]
                - heading "翻译助手" [level=6] [ref=e159]
              - generic [ref=e160]:
                - img [ref=e162]
                - heading "写作文档" [level=6] [ref=e166]
              - generic [ref=e167]:
                - img [ref=e169]
                - heading "搜索研究" [level=6] [ref=e173]
              - generic [ref=e174]:
                - img [ref=e176]
                - heading "数据分析" [level=6] [ref=e178]
              - generic [ref=e179]:
                - img [ref=e181]
                - heading "投资分析" [level=6] [ref=e185]
              - generic [ref=e186]:
                - img [ref=e188]
                - heading "自媒体运维" [level=6] [ref=e195]
          - generic [ref=e197]:
            - generic [ref=e198]:
              - img [ref=e200]
              - textbox "输入消息..." [ref=e208]
              - generic [ref=e209]:
                - generic [ref=e210]:
                  - button [ref=e211] [cursor=pointer]:
                    - img [ref=e213]
                  - button [ref=e216] [cursor=pointer]:
                    - img [ref=e218]
                  - button [ref=e221] [cursor=pointer]:
                    - img [ref=e223]
                  - button [ref=e226] [cursor=pointer]:
                    - img [ref=e228]
                  - button [ref=e231] [cursor=pointer]:
                    - img [ref=e233]
                  - button [ref=e241] [cursor=pointer]:
                    - img [ref=e243]
                  - button [disabled] [ref=e250]:
                    - generic:
                      - img
                  - button [disabled] [ref=e251]:
                    - generic:
                      - img
                  - button [disabled] [ref=e252]:
                    - generic:
                      - img
                  - button [ref=e253] [cursor=pointer]:
                    - img [ref=e255]
                - button [disabled] [ref=e257]:
                  - generic:
                    - img
            - generic [ref=e258]:
              - button "问答" [ref=e260] [cursor=pointer]:
                - img [ref=e262]
                - generic [ref=e264]: 问答
              - img [ref=e266] [cursor=pointer]
```

# Test source

```ts
  1   | import { test, expect } from '@playwright/test';
  2   | 
  3   | test.describe('Workflow Editor E2E Tests', () => {
  4   |   test.beforeEach(async ({ page }) => {
  5   |     await page.goto('/#/workflow');
  6   |     await page.waitForLoadState('networkidle');
  7   |   });
  8   | 
  9   |   test('should load workflow editor page', async ({ page }) => {
> 10  |     await expect(page.locator('text=工作流')).toBeVisible({ timeout: 10000 });
      |                                            ^ Error: expect(locator).toBeVisible() failed
  11  |   });
  12  | 
  13  |   test('should display template list', async ({ page }) => {
  14  |     await page.waitForSelector('text=模板列表', { timeout: 10000 }).catch(() => {});
  15  |     const templateList = page.locator('[class*="template"]').first();
  16  |     await expect(templateList).toBeVisible({ timeout: 5000 }).catch(() => {});
  17  |   });
  18  | 
  19  |   test('should open new template dialog', async ({ page }) => {
  20  |     const newButton = page.locator('button:has-text("新建")').first();
  21  |     if (await newButton.isVisible()) {
  22  |       await newButton.click();
  23  |       await expect(page.locator('text=创建工作流')).toBeVisible({ timeout: 5000 });
  24  |     }
  25  |   });
  26  | 
  27  |   test('should open AI panel', async ({ page }) => {
  28  |     const aiButton = page.locator('text=AI 助手').first();
  29  |     if (await aiButton.isVisible()) {
  30  |       await aiButton.click();
  31  |       await expect(page.locator('text=生成工作流')).toBeVisible({ timeout: 5000 });
  32  |       await expect(page.locator('text=优化 Prompt')).toBeVisible({ timeout: 5000 });
  33  |       await expect(page.locator('text=推荐节点')).toBeVisible({ timeout: 5000 });
  34  |     }
  35  |   });
  36  | 
  37  |   test('should switch between AI panel tabs', async ({ page }) => {
  38  |     const aiButton = page.locator('text=AI 助手').first();
  39  |     if (await aiButton.isVisible()) {
  40  |       await aiButton.click();
  41  |       await page.waitForTimeout(500);
  42  | 
  43  |       const optimizeTab = page.locator('text=优化 Prompt');
  44  |       if (await optimizeTab.isVisible()) {
  45  |         await optimizeTab.click();
  46  |         await expect(page.locator('text=输入要优化的 Agent Prompt')).toBeVisible({ timeout: 5000 });
  47  |       }
  48  |     }
  49  |   });
  50  | 
  51  |   test('should toggle sidebar', async ({ page }) => {
  52  |     const sidebar = page.locator('[class*="sidebar"]').first();
  53  |     if (await sidebar.isVisible()) {
  54  |       const toggleButton = page.locator('button[class*="toggle"]').first();
  55  |       if (await toggleButton.isVisible()) {
  56  |         await toggleButton.click();
  57  |         await page.waitForTimeout(300);
  58  |       }
  59  |     }
  60  |   });
  61  | 
  62  |   test('should display node palette', async ({ page }) => {
  63  |     const nodePalette = page.locator('text=节点').first();
  64  |     if (await nodePalette.isVisible()) {
  65  |       await expect(nodePalette).toBeVisible({ timeout: 5000 });
  66  |     }
  67  |   });
  68  | 
  69  |   test('should show import/export modal', async ({ page }) => {
  70  |     const importExportButton = page.locator('text=导入/导出').first();
  71  |     if (await importExportButton.isVisible()) {
  72  |       await importExportButton.click();
  73  |       await expect(page.locator('text=导入/导出模板')).toBeVisible({ timeout: 5000 });
  74  |       await expect(page.locator('text=导出模板')).toBeVisible({ timeout: 5000 });
  75  |       await expect(page.locator('text=导入')).toBeVisible({ timeout: 5000 });
  76  |     }
  77  |   });
  78  | 
  79  |   test('should filter templates by search', async ({ page }) => {
  80  |     const searchInput = page.locator('input[placeholder*="搜索"]').first();
  81  |     if (await searchInput.isVisible()) {
  82  |       await searchInput.fill('test');
  83  |       await page.waitForTimeout(500);
  84  |     }
  85  |   });
  86  | 
  87  |   test('should display validation errors', async ({ page }) => {
  88  |     const validateButton = page.locator('text=验证').first();
  89  |     if (await validateButton.isVisible()) {
  90  |       await validateButton.click();
  91  |       await page.waitForTimeout(500);
  92  |     }
  93  |   });
  94  | 
  95  |   test('should show save indicator when dirty', async ({ page }) => {
  96  |     const saveIndicator = page.locator('text=未保存').first();
  97  |     if (await saveIndicator.isVisible()) {
  98  |       await expect(saveIndicator).toBeVisible({ timeout: 5000 });
  99  |     }
  100 |   });
  101 | 
  102 |   test('should handle keyboard shortcuts', async ({ page }) => {
  103 |     await page.keyboard.press('Control+s');
  104 |     await page.waitForTimeout(300);
  105 | 
  106 |     await page.keyboard.press('Control+z');
  107 |     await page.waitForTimeout(300);
  108 | 
  109 |     await page.keyboard.press('Delete');
  110 |     await page.waitForTimeout(300);
```