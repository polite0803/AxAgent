# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: chat.spec.ts >> Settings >> should display settings sections
- Location: e2e\chat.spec.ts:57:3

# Error details

```
Test timeout of 30000ms exceeded while running "beforeEach" hook.
```

```
Error: page.waitForSelector: Test timeout of 30000ms exceeded.
Call log:
  - waiting for locator('[data-testid="settings-panel"]') to be visible

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
  - main [ref=e43]:
    - generic [ref=e44]:
      - generic [ref=e46]:
        - generic [ref=e47] [cursor=pointer]:
          - img [ref=e48]
          - generic [ref=e50]: 返回
          - generic [ref=e51]: Esc
        - menu [ref=e53]:
          - menuitem "通用设置" [ref=e54] [cursor=pointer]:
            - img [ref=e55]
            - generic [ref=e58]: 通用设置
          - menuitem "显示设置" [ref=e59] [cursor=pointer]:
            - img [ref=e60]
            - generic [ref=e66]: 显示设置
          - menuitem "服务商管理" [ref=e67] [cursor=pointer]:
            - img [ref=e68]
            - generic [ref=e70]: 服务商管理
          - menuitem "对话设置" [ref=e71] [cursor=pointer]:
            - img [ref=e72]
            - generic [ref=e74]: 对话设置
          - menuitem "默认模型" [ref=e75] [cursor=pointer]:
            - img [ref=e76]
            - generic [ref=e79]: 默认模型
          - menuitem "搜索提供商" [ref=e80] [cursor=pointer]:
            - img [ref=e81]
            - generic [ref=e84]: 搜索提供商
          - menuitem "工具管理" [ref=e85] [cursor=pointer]:
            - img [ref=e86]
            - generic [ref=e88]: 工具管理
          - menuitem "代理设置" [ref=e89] [cursor=pointer]:
            - img [ref=e90]
            - generic [ref=e93]: 代理设置
          - menuitem "快捷键" [ref=e94] [cursor=pointer]:
            - img [ref=e95]
            - generic [ref=e97]: 快捷键
          - menuitem "数据管理" [ref=e98] [cursor=pointer]:
            - img [ref=e99]
            - generic [ref=e103]: 数据管理
          - menuitem "存储空间" [ref=e104] [cursor=pointer]:
            - img [ref=e105]
            - generic [ref=e107]: 存储空间
          - menuitem "定时任务" [ref=e108] [cursor=pointer]:
            - img [ref=e109]
            - generic [ref=e112]: 定时任务
          - menuitem "备份中心" [ref=e113] [cursor=pointer]:
            - img [ref=e114]
            - generic [ref=e117]: 备份中心
          - menuitem "工作流设置" [ref=e118] [cursor=pointer]:
            - img [ref=e119]
            - generic [ref=e123]: 工作流设置
          - menuitem "关于" [ref=e124] [cursor=pointer]:
            - img [ref=e125]
            - generic [ref=e127]: 关于
      - generic [ref=e129]:
        - generic [ref=e130]:
          - generic [ref=e132]: 语言
          - generic [ref=e135]:
            - generic [ref=e136]: 语言
            - button "🇨🇳 简体中文" [ref=e137] [cursor=pointer]:
              - generic [ref=e139]: 🇨🇳 简体中文
              - img [ref=e140]
        - generic [ref=e143]:
          - generic [ref=e145]: 启动
          - generic [ref=e147]:
            - generic [ref=e148]:
              - generic [ref=e149]: 开机自启动
              - switch [ref=e150] [cursor=pointer]
            - separator [ref=e153]
            - generic [ref=e154]:
              - generic [ref=e155]: 启动时显示窗口
              - switch [checked] [ref=e156] [cursor=pointer]
            - separator [ref=e159]
            - generic [ref=e160]:
              - generic [ref=e161]: 窗口置顶
              - switch [disabled] [ref=e162]
            - separator [ref=e165]
            - generic [ref=e166]:
              - generic [ref=e167]: 启动时最小化
              - switch [disabled] [ref=e168]
        - generic [ref=e171]:
          - generic [ref=e173]: 托盘
          - generic [ref=e176]:
            - generic [ref=e177]: 关闭时最小化到托盘
            - switch [checked] [ref=e178] [cursor=pointer]
```

# Test source

```ts
  1  | import { test, expect } from '@playwright/test';
  2  | 
  3  | test.describe('Chat Flow', () => {
  4  |   test.beforeEach(async ({ page }) => {
  5  |     await page.goto('/');
  6  |     // Wait for the app to load
  7  |     await page.waitForSelector('[data-testid="chat-view"]', { timeout: 30000 });
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
> 54 |     await page.waitForSelector('[data-testid="settings-panel"]', { timeout: 30000 });
     |                ^ Error: page.waitForSelector: Test timeout of 30000ms exceeded.
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