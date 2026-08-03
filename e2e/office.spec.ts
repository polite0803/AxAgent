import { expect, test } from "@playwright/test";

/**
 * Office（像素办公室）冒烟测试 — 浏览器模式（browserMock）。
 *
 * 覆盖链路：进入办公室 Tab → 创建办公室 → 添加成员 → 群聊 dispatch →
 * 验证事件流（routing / agent_status / agent_message / complete）实时展示。
 */
async function dismissAllModals(page: import("@playwright/test").Page) {
  // 循环关闭所有可能出现的 Modal，防止延迟渲染的弹窗漏掉
  for (let i = 0; i < 3; i++) {
    let dismissed = false;
    try {
      await page.getByTestId("onboarding-skip").click({ timeout: 1500 });
      dismissed = true;
    } catch {}
    try {
      await page.locator(".ant-modal-close").first().click({ timeout: 1000 });
      dismissed = true;
    } catch {}
    // 关闭带 OK/确认按钮的 Modal
    try {
      const okBtn = page.locator(".ant-modal-footer .ant-btn-primary").first();
      if (await okBtn.isVisible({ timeout: 500 })) {
        await okBtn.click();
        dismissed = true;
      }
    } catch {}
    if (!dismissed) { break; }
    await page.waitForTimeout(300);
  }
  // 等待所有 Modal 消失
  await page.locator(".ant-modal-wrap").first().waitFor({ state: "hidden", timeout: 5000 }).catch(() => {});
  await page.waitForTimeout(300);
}

test.describe("Office (Pixel Office)", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/dashboard");
    await page.waitForSelector("body");
    await dismissAllModals(page);
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

    // 用 role=dialog 精准定位 Modal，避免 .last() 误选其他弹窗
    const modal = page.locator('[role="dialog"]').filter({ hasText: "创建办公室" }).first();
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
    const createModal = page.locator('[role="dialog"]').filter({ hasText: "创建办公室" }).first();
    await createModal.waitFor({ timeout: 5000 });
    await createModal.locator("input").first().fill("E2E 办公室");
    await createModal.getByRole("button", { name: "创建办公室" }).click();
    await expect(page.getByText("E2E 办公室").first()).toBeVisible({ timeout: 10000 });

    // ── 添加成员 ──
    const addMemberBtn = page.getByRole("button", { name: "添加成员" }).first();
    await addMemberBtn.click();
    // 直接等待输入框出现（modal.confirm 命令式弹窗）
    const displayNameInput = page.getByTestId("office-member-display-name");
    await displayNameInput.waitFor({ timeout: 10000 });
    await displayNameInput.fill("文案助手");
    await page.getByTestId("office-member-agent-slug").fill("copywriter");
    await page.getByTestId("office-member-role").fill("撰写产品文案");
    // modal.confirm 的 OK 按钮在 footer 中，文本同为"添加成员"
    // 用 .last() 选第二个（第一个是打开弹窗的按钮）
    const confirmBtn = page.getByRole("button", { name: "添加成员" }).last();
    await confirmBtn.click();
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
