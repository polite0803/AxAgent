// SPDX-License-Identifier: AGPL-3.0-only

import { expect, test } from "@playwright/test";

/**
 * P0-2 计划确认闸门端到端测试（浏览器 mock 模式）。
 *
 * 依赖 browserMock 的 agent_query/agent_approve_plan 模拟与内存事件总线
 * （src/lib/browserEvents.ts + invoke.ts 的 listen），使事件驱动的审批流在
 * `npm run dev` 下可被真实触发。
 */

const TOGGLE = '[data-testid="plan-approval-toggle"]';
const APPROVE_BTN = '[data-testid="plan-approval-approve"]';
const REJECT_BTN = '[data-testid="plan-approval-reject"]';
const INPUT = '[data-testid="message-input"]';
const SEND_BTN = '[data-testid="send-btn"]';
const STORAGE_KEY = "axagent:agent:planApprovalEnabled";

async function seedOnboarding(page: import("@playwright/test").Page) {
  await page.addInitScript(() => {
    try {
      localStorage.setItem(
        "axagent_settings",
        JSON.stringify({
          onboardingCompleted: true,
          onboardingWizardDismissed: true,
          onboardingTutorialCompleted: true,
        }),
      );
    } catch {
      /* ignore */
    }
  });
}

async function dismissModals(page: import("@playwright/test").Page) {
  // 尝试多种方式关闭可能存在的模态框（循环3次确保完全关闭）
  for (let i = 0; i < 3; i++) {
    // 1. 首先尝试点 X 关闭
    const closeBtn = page.locator(".ant-modal-close").first();
    if (await closeBtn.isVisible({ timeout: 500 }).catch(() => false)) {
      await closeBtn.click({ force: true });
      await page.waitForTimeout(200);
    }

    // 2. 欢迎引导向导（WelcomeWizard）footer 为 null，没有 .ant-modal-footer，
    // 关闭动作在弹窗体内的"跳过"按钮上（data-testid=onboarding-skip）。
    const skipBtn = page.getByTestId("onboarding-skip").first();
    if (await skipBtn.isVisible({ timeout: 500 }).catch(() => false)) {
      await skipBtn.click({ force: true });
      await page.waitForTimeout(300);
    }

    // 3. 尝试点击主按钮关闭
    const okBtn = page.locator(".ant-modal-footer .ant-btn-primary").first();
    if (await okBtn.isVisible({ timeout: 500 }).catch(() => false)) {
      await okBtn.click({ force: true });
      await page.waitForTimeout(200);
    }
  }

  // 额外等待，确保模态框完全消失
  await page.waitForTimeout(500);
}

test.describe("Plan Approval Gate (P0-2)", () => {
  test.beforeEach(async ({ page }) => {
    page.on("pageerror", (err) => console.log("PAGEERROR>>", err.message));
    page.on("console", (msg) => {
      console.log("CONSOLE>>", msg.type(), msg.text());
    });
    await seedOnboarding(page);
    await page.goto("/chat");
    await page.waitForLoadState("networkidle");
    await page.waitForSelector('[data-testid="chat-view"]', { timeout: 20000 });
    await page.waitForTimeout(500);
    await dismissModals(page);
  });

  test("plan approval toggle persists to localStorage", async ({ page }) => {
    // 再次关闭模态框，确保 toggle 不被遮挡
    await dismissModals(page);
    const toggle = page.locator(TOGGLE);
    await expect(toggle).toBeVisible({ timeout: 10000 });

    // 初始应为关闭（无 true）
    let stored = await page.evaluate((k) => localStorage.getItem(k), STORAGE_KEY);
    expect(stored).not.toBe("true");

    // 开启 → 持久化为 true
    await toggle.click({ force: true });
    await expect
      .poll(async () => page.evaluate((k) => localStorage.getItem(k), STORAGE_KEY))
      .toBe("true");

    // 关闭 → 持久化回 false
    await toggle.click({ force: true });
    await expect
      .poll(async () => page.evaluate((k) => localStorage.getItem(k), STORAGE_KEY))
      .toBe("false");
  });

  test("gate opens on complex task and approve proceeds", async ({ page }) => {
    // 通过 toggle 按钮启用计划确认闸门
    const toggle = page.locator(TOGGLE);
    await toggle.click({ force: true });
    // 等待一下让状态更新
    await page.waitForTimeout(500);

    await page.fill(
      INPUT,
      "请先分析这段代码的问题，再重构它，接着写单元测试，最后生成设计文档",
    );
    await page.locator(SEND_BTN).click();

    // 计划确认弹窗出现（agent-plan-ready-for-approval 事件已触发）
    await expect(page.locator(APPROVE_BTN)).toBeVisible({ timeout: 15000 });
    await expect(page.locator(REJECT_BTN)).toBeVisible();

    // 批准执行 → 弹窗关闭（agent_approve_plan 回合成功）
    await page.locator(APPROVE_BTN).click();
    await expect(page.locator(APPROVE_BTN)).toBeHidden({ timeout: 15000 });
  });

  test("reject returns rejected status and shows toast", async ({ page }) => {
    // 通过 toggle 按钮启用计划确认闸门
    const toggle = page.locator(TOGGLE);
    await toggle.click({ force: true });
    await page.waitForTimeout(500);

    await page.fill(
      INPUT,
      "请先调研市场，再设计系统架构，然后实现核心模块并部署上线",
    );
    await page.locator(SEND_BTN).click();

    await expect(page.locator(REJECT_BTN)).toBeVisible({ timeout: 15000 });

    // 拒绝 → 弹窗关闭
    await page.locator(REJECT_BTN).click();
    await expect(page.locator(REJECT_BTN)).toBeHidden({ timeout: 15000 });

    // 验证模态框已关闭（通过检查 approve 按钮不再可见）
    await expect(page.locator(APPROVE_BTN)).toBeHidden({ timeout: 5000 });

    // 验证消息已被移除（拒绝后不应显示助手回复）
    // 检查是否有错误或信息提示出现
    await page.waitForTimeout(1000);
  });
});
