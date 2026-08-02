import { expect, test } from "@playwright/test";

/**
 * Office（像素办公室）冒烟测试 — 浏览器模式（browserMock）。
 *
 * 覆盖链路：进入办公室 Tab → 创建办公室 → 添加成员 → 群聊 dispatch →
 * 验证事件流（routing / agent_status / agent_message / complete）实时展示。
 */
test.describe("Office (Pixel Office)", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/dashboard");
    await page.waitForSelector("body");
    // 关闭可能出现的引导弹窗（interactive tutorial 等），locator 自动等待出现
    try {
      await page.locator(".ant-modal-close").first().click({ timeout: 5000 });
      await page.locator(".ant-modal-wrap").first().waitFor({ state: "hidden", timeout: 3000 });
    } catch {
      // 5 秒内未出现弹窗，正常
    }
    // 切换到「像素办公室」Tab
    const officeTab = page.locator('[role="tab"]', { hasText: "像素办公室" }).first();
    await officeTab.waitFor({ timeout: 15000 });
    await officeTab.click();
    // 等待 Office 画布容器出现（Phaser 初始化）
    await page.waitForSelector(".ant-tabs-tabpane canvas", { timeout: 15000 }).catch(() => {});
  });

  test("should create an office via modal", async ({ page }) => {
    const createBtn = page.getByRole("button", { name: "创建办公室" }).first();
    await createBtn.waitFor({ timeout: 15000 });
    await createBtn.click();

    // wizard 关闭后 DOM 残留可能仍匹配 .ant-modal，用 .last() 锁定最新（创建办公室）
    const modal = page.locator(".ant-modal-wrap").last();
    await modal.waitFor({ timeout: 5000 });
    const nameInput = modal.locator("input").first();
    await nameInput.fill("测试办公室");
    await modal.getByRole("button", { name: "创建办公室" }).click();

    // 创建成功后，顶部出现办公室名称
    await expect(page.getByText("测试办公室").first()).toBeVisible({ timeout: 10000 });
  });

  test("should add a member and dispatch a message", async ({ page }) => {
    // ── 前置：创建办公室 ──
    const createBtn = page.getByRole("button", { name: "创建办公室" }).first();
    await createBtn.waitFor({ timeout: 15000 });
    await createBtn.click();
    const modal = page.locator(".ant-modal-wrap").last();
    await modal.waitFor({ timeout: 5000 });
    await modal.locator("input").first().fill("E2E 办公室");
    await modal.getByRole("button", { name: "创建办公室" }).click();
    await expect(page.getByText("E2E 办公室").first()).toBeVisible({ timeout: 10000 });

    // ── 添加成员 ──
    const addMemberBtn = page.getByRole("button", { name: "添加成员" }).first();
    await addMemberBtn.click();
    const memberModal = page.locator(".ant-modal-wrap").last();
    await memberModal.waitFor({ timeout: 5000 });
    const inputs = memberModal.locator("input");
    // inputs[0]=显示名称, inputs[1]=slug, inputs[2]=角色
    await inputs.nth(0).fill("文案助手");
    await inputs.nth(1).fill("copywriter");
    await inputs.nth(2).fill("撰写产品文案");
    await memberModal.getByRole("button", { name: "添加成员" }).click();
    await expect(page.getByText("文案助手").first()).toBeVisible({ timeout: 10000 });

    // ── 群聊 dispatch：发送消息，验证事件流实时展示 ──
    // ChatPanel 输入框用 placeholder 定位（跨 antd 版本稳定）
    const chatTextarea = page.getByPlaceholder("输入群聊消息…");
    await chatTextarea.fill("帮我写一段产品介绍");
    await page.getByRole("button", { name: "发送" }).first().click();

    // 路由事件出现（routing 标签）
    await expect(page.getByText("路由", { exact: false }).first()).toBeVisible({ timeout: 15000 });
    // 事件流中出现成员回复（检查 agent_message 的内容）
    await expect(page.getByText(/帮我写一段产品介绍/).first()).toBeVisible({ timeout: 15000 });
    // 流结束标记 ✓
    await expect(page.getByText("✓").first()).toBeVisible({ timeout: 15000 });
  });
});
