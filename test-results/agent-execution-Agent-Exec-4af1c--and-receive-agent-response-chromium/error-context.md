# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: agent-execution.spec.ts >> Agent Execution Flow >> should send message and receive agent response
- Location: e2e\agent-execution.spec.ts:16:3

# Error details

```
Error: expect(received).toBeGreaterThan(expected)

Expected: > 0
Received:   0
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
      - button [ref=e20] [cursor=pointer]:
        - img [ref=e21]
      - button [ref=e24] [cursor=pointer]:
        - img [ref=e25]
      - button [ref=e29] [cursor=pointer]:
        - img [ref=e30]
  - generic [ref=e33]:
    - generic [ref=e35]:
      - navigation [ref=e36]:
        - generic [ref=e37] [cursor=pointer]:
          - img [ref=e40]
          - generic: 对话
        - generic [ref=e42] [cursor=pointer]:
          - img [ref=e44]
          - generic: 技能
        - generic [ref=e47] [cursor=pointer]:
          - img [ref=e49]
          - generic: 市场
        - generic [ref=e53] [cursor=pointer]:
          - img [ref=e55]
          - generic: 提示词
        - generic [ref=e58] [cursor=pointer]:
          - img [ref=e60]
          - generic: 知识库
        - generic [ref=e62] [cursor=pointer]:
          - img [ref=e64]
          - generic: 记忆
        - generic [ref=e72] [cursor=pointer]:
          - img [ref=e74]
          - generic: 连接
        - generic [ref=e77] [cursor=pointer]:
          - img [ref=e79]
          - generic: 网关
        - generic [ref=e83] [cursor=pointer]:
          - img [ref=e85]
          - generic: 文件
      - generic [ref=e87] [cursor=pointer]:
        - img [ref=e89]
        - generic: 个人信息
    - main [ref=e92]:
      - generic [ref=e94]:
        - generic [ref=e96]:
          - generic [ref=e97]:
            - generic [ref=e98]:
              - button [ref=e99] [cursor=pointer]:
                - img [ref=e101]
              - button [ref=e104] [cursor=pointer]:
                - img [ref=e106]
              - button [ref=e109] [cursor=pointer]:
                - img [ref=e111]
              - button [ref=e113] [cursor=pointer]:
                - img [ref=e115]
            - button [ref=e118] [cursor=pointer]:
              - img [ref=e120]
          - generic [ref=e124]:
            - list [ref=e125]:
              - listitem [ref=e126]:
                - generic [ref=e128]: 今天
                - list [ref=e130]:
                  - listitem "新建对话" [ref=e131] [cursor=pointer]:
                    - generic "OpenAI" [ref=e133]:
                      - img "OpenAI" [ref=e134]
                    - generic [ref=e136]: 新建对话
                    - img "ellipsis" [ref=e138]:
                      - img [ref=e139]
            - status [ref=e141]
        - generic [ref=e142]:
          - generic [ref=e143]:
            - generic [ref=e145] [cursor=pointer]:
              - generic "OpenAI" [ref=e147]:
                - img "OpenAI" [ref=e148]
              - generic [ref=e150]: 新建对话
              - img [ref=e152]
            - img [ref=e156] [cursor=pointer]
          - generic [ref=e158]:
            - generic [ref=e159]:
              - generic "OpenAI" [ref=e160]:
                - img "OpenAI" [ref=e161]
              - generic [ref=e163] [cursor=pointer]:
                - text: 新建对话
                - img [ref=e164]
              - generic [ref=e167]:
                - generic [ref=e168]:
                  - generic: 选择场景
                  - combobox [disabled] [ref=e169]
                - img "down" [ref=e171]:
                  - img [ref=e172]
              - generic [ref=e174] [cursor=pointer]:
                - generic "OpenAI" [ref=e175]:
                  - img "OpenAI" [ref=e176]
                - generic [ref=e178]: OpenAI
                - generic [ref=e179]: gpt-4o
              - button [ref=e180] [cursor=pointer]:
                - img [ref=e182]
              - button [ref=e183] [cursor=pointer]:
                - img [ref=e185]
            - generic [ref=e194]:
              - generic [ref=e195]:
                - img [ref=e198]
                - generic [ref=e201]:
                  - generic [ref=e204]:
                    - generic [ref=e205]: 你
                    - generic [ref=e206]: 05:53
                  - generic [ref=e207]: What is 2+2?
                  - generic [ref=e210]:
                    - img [ref=e213] [cursor=pointer]
                    - img [ref=e218] [cursor=pointer]
                    - img [ref=e223] [cursor=pointer]
                    - img [ref=e227] [cursor=pointer]
              - generic [ref=e230]:
                - generic "OpenAI" [ref=e232]:
                  - img "OpenAI" [ref=e233]
                - generic [ref=e235]:
                  - generic [ref=e238]:
                    - generic [ref=e239]: OpenAI
                    - generic [ref=e240]: gpt-4o
                    - generic [ref=e241]: 05:53
                  - generic [ref=e244]:
                    - paragraph [ref=e247]:
                      - generic [ref=e248]: 收到你的消息：「What is 2+2?」
                    - paragraph [ref=e251]:
                      - generic [ref=e252]: 当前为浏览器预览模式，无法调用真实 AI 接口。此模式用于 UI 开发和体验测试。
                    - paragraph [ref=e255]:
                      - generic [ref=e256]: 如需 AI 回复，请使用
                      - code [ref=e257]: cargo tauri dev
                      - generic [ref=e258]: 启动完整应用。
                  - generic [ref=e264]:
                    - img [ref=e267] [cursor=pointer]
                    - img [ref=e272] [cursor=pointer]
                    - img [ref=e277] [cursor=pointer]
                    - img [ref=e282] [cursor=pointer]
                    - img [ref=e286] [cursor=pointer]
                    - img [ref=e291] [cursor=pointer]
            - generic [ref=e295]:
              - generic [ref=e296]:
                - img [ref=e298]
                - textbox "输入消息..." [ref=e306]
                - generic [ref=e307]:
                  - generic [ref=e308]:
                    - button [ref=e309] [cursor=pointer]:
                      - img [ref=e311]
                    - button [ref=e314] [cursor=pointer]:
                      - img [ref=e316]
                    - button [ref=e319] [cursor=pointer]:
                      - img [ref=e321]
                    - button [ref=e324] [cursor=pointer]:
                      - img [ref=e326]
                    - button [ref=e329] [cursor=pointer]:
                      - img [ref=e331]
                    - button [ref=e339] [cursor=pointer]:
                      - img [ref=e341]
                    - button [ref=e348] [cursor=pointer]:
                      - img [ref=e350]
                    - button [ref=e352] [cursor=pointer]:
                      - img [ref=e354]
                    - button [ref=e360] [cursor=pointer]:
                      - img [ref=e362]
                    - button [ref=e365] [cursor=pointer]:
                      - img [ref=e367]
                    - button [ref=e368] [cursor=pointer]:
                      - img [ref=e370]
                  - button [disabled] [ref=e373]:
                    - generic:
                      - img
              - generic [ref=e375]:
                - generic [ref=e376]: 2 条上下文
                - img [ref=e377] [cursor=pointer]
```

# Test source

```ts
  1  | import { expect, test } from "@playwright/test";
  2  | 
  3  | test.describe("Agent Execution Flow", () => {
  4  |   test.beforeEach(async ({ page }) => {
  5  |     await page.goto("/");
  6  |     await page.waitForSelector('[data-testid="chat-view"]', { timeout: 60000 });
  7  |   });
  8  | 
  9  |   test("should display agent status indicator", async ({ page }) => {
  10 |     const statusIndicator = page.locator('[data-testid="agent-status"]');
  11 |     if (await statusIndicator.isVisible({ timeout: 5000 }).catch(() => false)) {
  12 |       await expect(statusIndicator).toBeVisible();
  13 |     }
  14 |   });
  15 | 
  16 |   test("should send message and receive agent response", async ({ page }) => {
  17 |     const newConvBtn = page.locator('[data-testid="new-conversation-btn"]');
  18 |     if (await newConvBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
  19 |       await newConvBtn.click();
  20 |     }
  21 | 
  22 |     const input = page.locator('[data-testid="message-input"]');
  23 |     await input.fill("What is 2+2?");
  24 |     const sendBtn = page.locator('[data-testid="send-btn"]');
  25 |     await sendBtn.click();
  26 | 
  27 |     await page.waitForTimeout(5000);
  28 | 
  29 |     const messages = page.locator('[data-testid="chat-message"]');
  30 |     const count = await messages.count();
> 31 |     expect(count).toBeGreaterThan(0);
     |                   ^ Error: expect(received).toBeGreaterThan(expected)
  32 |   });
  33 | 
  34 |   test("should handle tool call in conversation", async ({ page }) => {
  35 |     const newConvBtn = page.locator('[data-testid="new-conversation-btn"]');
  36 |     if (await newConvBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
  37 |       await newConvBtn.click();
  38 |     }
  39 | 
  40 |     const input = page.locator('[data-testid="message-input"]');
  41 |     await input.fill("Read the file README.md");
  42 |     const sendBtn = page.locator('[data-testid="send-btn"]');
  43 |     await sendBtn.click();
  44 | 
  45 |     await page.waitForTimeout(8000);
  46 | 
  47 |     // Check if tool call cards appear
  48 |     const toolCall = page.locator('[data-testid="tool-call-card"]');
  49 |     const toolCallVisible = await toolCall.isVisible({ timeout: 3000 }).catch(() => false);
  50 |     // Tool call may or may not appear depending on agent configuration
  51 |     expect(toolCallVisible || true).toBeTruthy();
  52 |   });
  53 | 
  54 |   test("should cancel agent execution", async ({ page }) => {
  55 |     const newConvBtn = page.locator('[data-testid="new-conversation-btn"]');
  56 |     if (await newConvBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
  57 |       await newConvBtn.click();
  58 |     }
  59 | 
  60 |     const input = page.locator('[data-testid="message-input"]');
  61 |     await input.fill("Write a 1000 word essay about AI");
  62 |     const sendBtn = page.locator('[data-testid="send-btn"]');
  63 |     await sendBtn.click();
  64 | 
  65 |     await page.waitForTimeout(2000);
  66 | 
  67 |     const stopBtn = page.locator('[data-testid="stop-generation-btn"]');
  68 |     if (await stopBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
  69 |       await stopBtn.click();
  70 |       await page.waitForTimeout(1000);
  71 |     }
  72 |   });
  73 | 
  74 |   test("should switch models in agent config", async ({ page }) => {
  75 |     // Navigate to settings
  76 |     await page.goto("/settings");
  77 |     await page.waitForSelector('[data-testid="settings-panel"]', { timeout: 30000 });
  78 | 
  79 |     // Look for model or provider settings
  80 |     const modelSection = page.locator("text=Model");
  81 |     if (await modelSection.isVisible({ timeout: 3000 }).catch(() => false)) {
  82 |       await expect(modelSection.first()).toBeVisible();
  83 |     }
  84 |   });
  85 | });
  86 | 
```