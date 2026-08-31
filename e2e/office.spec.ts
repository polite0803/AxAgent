import { expect, test } from "@playwright/test";

/**
 * Office（像素办公室）冒烟测试 — 浏览器模式（browserMock）。
 *
 * 覆盖链路：进入办公室 Tab → 创建办公室。
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
});
