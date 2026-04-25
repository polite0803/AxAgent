# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: chat.spec.ts >> Chat Flow >> should display chat interface
- Location: e2e\chat.spec.ts:10:3

# Error details

```
Test timeout of 30000ms exceeded while running "beforeEach" hook.
```

```
Error: page.waitForSelector: Test timeout of 30000ms exceeded.
Call log:
  - waiting for locator('[data-testid="chat-view"]') to be visible

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
  3  | test.describe('Chat Flow', () => {
  4  |   test.beforeEach(async ({ page }) => {
  5  |     await page.goto('/');
  6  |     // Wait for the app to load
> 7  |     await page.waitForSelector('[data-testid="chat-view"]', { timeout: 30000 });
     |                ^ Error: page.waitForSelector: Test timeout of 30000ms exceeded.
  8  |   });
  9  | 
  10 |   test('should display chat interface', async ({ page }) => {
  11 |     // Check that the chat view is visible
  12 |     await expect(page.locator('[data-testid="chat-view"]')).toBeVisible();
  13 |     
  14 |     // Check that the input area is present
  15 |     await expect(page.locator('[data-testid="message-input"]')).toBeVisible();
  16 |   });
  17 | 
  18 |   test('should create a new conversation and send a message', async ({ page }) => {
  19 |     // Click new conversation button
  20 |     const newConvBtn = page.locator('[data-testid="new-conversation-btn"]');
  21 |     if (await newConvBtn.isVisible()) {
  22 |       await newConvBtn.click();
  23 |     }
  24 | 
  25 |     // Type a message
  26 |     const input = page.locator('[data-testid="message-input"]');
  27 |     await input.fill('Hello, this is a test message');
  28 |     
  29 |     // Send the message
  30 |     const sendBtn = page.locator('[data-testid="send-btn"]');
  31 |     await sendBtn.click();
  32 | 
  33 |     // Wait for response (or at least for the message to appear)
  34 |     await page.waitForTimeout(2000);
  35 | 
  36 |     // Verify the message appears in the chat
  37 |     await expect(page.locator('text=Hello, this is a test message')).toBeVisible();
  38 |   });
  39 | 
  40 |   test('should navigate to settings page', async ({ page }) => {
  41 |     // Click settings icon in sidebar
  42 |     const settingsBtn = page.locator('[data-testid="settings-nav-btn"]');
  43 |     await settingsBtn.click();
  44 | 
  45 |     // Verify we're on settings page
  46 |     await expect(page).toHaveURL(/.*settings.*/);
  47 |     await expect(page.locator('[data-testid="settings-panel"]')).toBeVisible();
  48 |   });
  49 | });
  50 | 
  51 | test.describe('Settings', () => {
  52 |   test.beforeEach(async ({ page }) => {
  53 |     await page.goto('/settings');
  54 |     await page.waitForSelector('[data-testid="settings-panel"]', { timeout: 30000 });
  55 |   });
  56 | 
  57 |   test('should display settings sections', async ({ page }) => {
  58 |     // Check that settings sidebar is visible
  59 |     await expect(page.locator('[data-testid="settings-sidebar"]')).toBeVisible();
  60 |   });
  61 | 
  62 |   test('should save theme preference', async ({ page }) => {
  63 |     // Navigate to appearance settings
  64 |     const appearanceBtn = page.locator('text=Appearance');
  65 |     if (await appearanceBtn.isVisible()) {
  66 |       await appearanceBtn.click();
  67 |     }
  68 | 
  69 |     // Toggle dark mode
  70 |     const darkModeToggle = page.locator('[data-testid="dark-mode-toggle"]');
  71 |     if (await darkModeToggle.isVisible()) {
  72 |       await darkModeToggle.click();
  73 |       
  74 |       // Wait for save
  75 |       await page.waitForTimeout(1000);
  76 |     }
  77 |   });
  78 | });
  79 | 
```