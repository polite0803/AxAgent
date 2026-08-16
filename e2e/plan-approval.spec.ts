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

test.describe("Plan Approval Gate (P0-2)", () => {
  test.beforeEach(async ({ page }) => {
    page.on("pageerror", (err) => console.log("PAGEERROR>>", err.message));
    page.on("console", (msg) => {
      if (msg.type() === "error") { console.log("CONSOLE_ERR>>", msg.text()); }
    });
    // 清空持久化开关 + 预置向导已完成（防止 WelcomeWizard Modal 阻挡交互）
    // 注意：browserMock 使用 axagent_settings 作为 key
    await page.addInitScript(() => {
      try {
        localStorage.removeItem("axagent:agent:planApprovalEnabled");
      } catch { /* ignore */ }
      try {
        // 设置 onboarding 状态（通过 axagent_settings key）
        localStorage.setItem(
          "axagent_settings",
          JSON.stringify({
            onboardingCompleted: true,
            onboardingWizardDismissed: true,
            onboardingTutorialCompleted: true,
          }),
        );
      } catch { /* ignore */ }
    });
    await page.goto("/chat");

    // 等待页面完全加载
    await page.waitForLoadState("networkidle");

    // 等待 chat-view 出现
    await page.waitForSelector('[data-testid="chat-view"]', { timeout: 20000 });

    // 额外等待，确保 onboarding 状态被正确加载
    await page.waitForTimeout(500);

    // 尝试关闭任何可能存在的模态框
    const skipBtn = page.getByTestId("onboarding-skip");
    if (await skipBtn.isVisible({ timeout: 2000 }).catch(() => false)) {
      await skipBtn.click({ force: true });
      await page.waitForTimeout(500);
    }
  });

  test("plan approval toggle persists to localStorage", async ({ page }) => {
    const toggle = page.locator(TOGGLE);
    await expect(toggle).toBeVisible({ timeout: 10000 });

    // 初始应为关闭（无 true）
    let stored = await page.evaluate((k) => localStorage.getItem(k), STORAGE_KEY);
    expect(stored).not.toBe("true");

    // 开启 → 持久化为 true
    await toggle.click();
    await expect
      .poll(async () => page.evaluate((k) => localStorage.getItem(k), STORAGE_KEY))
      .toBe("true");

    // 关闭 → 持久化回 false
    await toggle.click();
    await expect
      .poll(async () => page.evaluate((k) => localStorage.getItem(k), STORAGE_KEY))
      .toBe("false");
  });

  test("gate opens on complex task and approve proceeds", async ({ page }) => {
    await page.locator(TOGGLE).click();
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
    await page.locator(TOGGLE).click();
    await page.fill(
      INPUT,
      "请先调研市场，再设计系统架构，然后实现核心模块并部署上线",
    );
    await page.locator(SEND_BTN).click();

    await expect(page.locator(REJECT_BTN)).toBeVisible({ timeout: 15000 });

    // 拒绝 → 弹窗关闭
    await page.locator(REJECT_BTN).click();
    await expect(page.locator(REJECT_BTN)).toBeHidden({ timeout: 15000 });

    // 拒绝提示出现（zh-CN 默认语言）
    await expect(page.getByText("计划已被拒绝，本轮未执行")).toBeVisible({
      timeout: 10000,
    });
  });
});
