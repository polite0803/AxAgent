# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: gateway.spec.ts >> Gateway Management E2E >> should show gateway connection status
- Location: e2e\gateway.spec.ts:14:3

# Error details

```
Test timeout of 30000ms exceeded while running "beforeEach" hook.
```

```
Error: page.waitForSelector: Test timeout of 30000ms exceeded.
Call log:
  - waiting for locator('[data-testid="gateway-overview"]') to be visible

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
  1  | import { test, expect } from '@playwright/test';
  2  | 
  3  | test.describe('Gateway Management E2E', () => {
  4  |   test.beforeEach(async ({ page }) => {
  5  |     await page.goto('/');
> 6  |     await page.waitForSelector('[data-testid="gateway-overview"]', { timeout: 30000 });
     |                ^ Error: page.waitForSelector: Test timeout of 30000ms exceeded.
  7  |   });
  8  | 
  9  |   test('should display gateway overview page', async ({ page }) => {
  10 |     await expect(page.locator('[data-testid="gateway-overview"]')).toBeVisible();
  11 |     await expect(page.locator('text=Gateway')).toBeVisible();
  12 |   });
  13 | 
  14 |   test('should show gateway connection status', async ({ page }) => {
  15 |     const statusIndicator = page.locator('[data-testid="gateway-status"]');
  16 |     if (await statusIndicator.isVisible()) {
  17 |       await expect(statusIndicator).toBeVisible();
  18 |     }
  19 |   });
  20 | 
  21 |   test('should display active agents list', async ({ page }) => {
  22 |     const agentsList = page.locator('[data-testid="active-agents-list"]');
  23 |     if (await agentsList.isVisible()) {
  24 |       await expect(agentsList).toBeVisible();
  25 |     }
  26 |   });
  27 | 
  28 |   test('should navigate to gateway diagnostics', async ({ page }) => {
  29 |     const diagnosticsBtn = page.locator('[data-testid="gateway-diagnostics-btn"]');
  30 |     if (await diagnosticsBtn.isVisible()) {
  31 |       await diagnosticsBtn.click();
  32 |       await expect(page.locator('[data-testid="gateway-diagnostics"]')).toBeVisible();
  33 |     }
  34 |   });
  35 | 
  36 |   test('should display gateway metrics', async ({ page }) => {
  37 |     const metricsPanel = page.locator('[data-testid="gateway-metrics"]');
  38 |     if (await metricsPanel.isVisible()) {
  39 |       await expect(metricsPanel).toBeVisible();
  40 |     }
  41 |   });
  42 | });
  43 | 
  44 | test.describe('Gateway Templates E2E', () => {
  45 |   test.beforeEach(async ({ page }) => {
  46 |     await page.goto('/gateway/templates');
  47 |     await page.waitForTimeout(1000);
  48 |   });
  49 | 
  50 |   test('should display templates list', async ({ page }) => {
  51 |     const templatesList = page.locator('[data-testid="templates-list"]');
  52 |     if (await templatesList.isVisible()) {
  53 |       await expect(templatesList).toBeVisible();
  54 |     }
  55 |   });
  56 | 
  57 |   test('should create a new template', async ({ page }) => {
  58 |     const createBtn = page.locator('[data-testid="create-template-btn"]');
  59 |     if (await createBtn.isVisible()) {
  60 |       await createBtn.click();
  61 |       await expect(page.locator('[data-testid="template-form"]')).toBeVisible();
  62 |     }
  63 |   });
  64 | });
  65 | 
```