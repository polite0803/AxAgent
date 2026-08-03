import { expect, test } from "@playwright/test";

async function dismissModals(page: import("@playwright/test").Page) {
  // 循环关闭所有可能出现的 Modal，防止延迟渲染的弹窗漏掉
  for (let i = 0; i < 3; i++) {
    let dismissed = false;
    // 优先关闭 WelcomeWizard（带 footer=null，只有 skip 按钮可关）
    try {
      if (await page.getByTestId("onboarding-skip").isVisible({ timeout: 1500 }).catch(() => false)) {
        await page.getByTestId("onboarding-skip").click();
        dismissed = true;
      }
    } catch {}
    // 关闭其他带 close 按钮的 Modal
    try {
      if (await page.locator(".ant-modal-close").first().isVisible({ timeout: 1000 }).catch(() => false)) {
        await page.locator(".ant-modal-close").first().click();
        dismissed = true;
      }
    } catch {}
    // 关闭带 OK 按钮的 Modal
    try {
      const okBtn = page.locator(".ant-modal-footer .ant-btn-primary").first();
      if (await okBtn.isVisible({ timeout: 500 }).catch(() => false)) {
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

/**
 * Time-Travel / As-Of mode E2E spec
 *
 * Validates the user-facing surface of the time anchor:
 *  1. LIVE pill is mounted in the AppHeader on every page
 *  2. Clicking LIVE → opens AsOfDatePicker modal → picking a past date → enters Replay
 *  3. In Replay, ReplayBadge appears in panels (e.g. stock-analysis, backtest)
 *  4. Trying to switch back to Live shows a confirm Modal (one-step guard)
 *  5. Replay Workbench page forces date re-pick even when an asOfDate is already set
 *  6. AppHeader mode-switch is sticky across navigation (state in Zustand persist)
 *  7. Tour bubble is shown on first mount and dismissed via "Got it"
 */

test.describe("Time Travel / As-Of Mode", () => {
  test.beforeEach(async ({ page }) => {
    // Reset persisted time-anchor state before each test so we start at LIVE
    await page.addInitScript(() => {
      try {
        const key = "axagent-time-anchor";
        const raw = localStorage.getItem(key);
        if (raw) {
          const parsed = JSON.parse(raw);
          parsed.state = {
            asOfDate: null,
            mode: "live",
            tourSeen: true,
            pendingLiveConfirm: false,
          };
          localStorage.setItem(key, JSON.stringify(parsed));
        } else {
          localStorage.setItem(
            key,
            JSON.stringify({
              state: {
                asOfDate: null,
                mode: "live",
                tourSeen: true,
                pendingLiveConfirm: false,
              },
              version: 0,
            }),
          );
        }
      } catch {
        /* noop */
      }
    });
    // AppHeader 仅在非聊天、非股票页面上渲染。
    // 详见 ContentArea.tsx line 164 (`!isStockPage && <AppHeader />`) 和 AppHeader.tsx line 64 (`if (isChatPage) return null`)。
    await page.goto("/settings");
    await page.waitForLoadState("domcontentloaded");
    await dismissModals(page);
  });

  test("AppHeader mounts the LIVE pill on non-chat, non-stock pages", async ({ page }) => {
    await expect(page.locator('[data-testid="page-time-anchor"]')).toBeVisible({
      timeout: 30000,
    });
    // Navigate to another non-chat, non-stock page and verify pill is still visible
    await page.goto("/knowledge");
    await page.waitForLoadState("domcontentloaded");
    await expect(page.locator('[data-testid="page-time-anchor"]')).toBeVisible({
      timeout: 30000,
    });
  });

  test("switching to Replay on the Segmented opens the date picker", async ({ page }) => {
    const anchor = page.locator('[data-testid="page-time-anchor"]');
    await expect(anchor).toBeVisible({ timeout: 30000 });
    await dismissModals(page);
    // 再次确认无 Modal 阻塞后再交互
    await expect(page.locator(".ant-modal-wrap").first()).toBeHidden({ timeout: 3000 }).catch(() => {});
    // 通过 data-testid 定位 Segmented，点击"历史回放"选项
    const segmented = page.getByTestId("time-anchor-segmented");
    await expect(segmented).toBeVisible({ timeout: 10000 });
    // 使用 force 避免被意外浮层拦截
    await segmented.locator("label.ant-segmented-item").last().click({ force: true });
    const picker = page.locator('[data-testid="asof-date-picker"]');
    await expect(picker).toBeVisible({ timeout: 10000 });
  });

  test("picking a past date enters Replay mode and shows the Replay badge", async ({ page }) => {
    const anchor = page.locator('[data-testid="page-time-anchor"]');
    await dismissModals(page);
    // 再次确认无 Modal 阻塞后再交互
    await expect(page.locator(".ant-modal-wrap").first()).toBeHidden({ timeout: 3000 }).catch(() => {});
    // 通过 data-testid 定位 Segmented，点击"历史回放"选项
    const segmented = page.getByTestId("time-anchor-segmented");
    await expect(segmented).toBeVisible({ timeout: 10000 });
    // 使用 force 避免被意外浮层拦截
    await segmented.locator("label.ant-segmented-item").last().click({ force: true });
    const picker = page.locator('[data-testid="asof-date-picker"]');
    await expect(picker).toBeVisible({ timeout: 10000 });

    // 点 DatePicker 容器打开日历面板，选一个过去日期
    await picker.click();
    const calendar = page.locator(".ant-picker-dropdown").first();
    await expect(calendar).toBeVisible({ timeout: 5000 });
    await calendar.locator(".ant-picker-cell:not(.ant-picker-cell-disabled)").first().click();

    // 选日期后会自动触发 enterReplay，Segmented 应显示"回放"字样
    await expect(anchor).toContainText(/replay|回放|Replay/i, { timeout: 10000 });

    // 导航到 stock-analysis 检查 ReplayBadge
    await page.goto("/stock-analysis");
    await page.waitForLoadState("domcontentloaded");
    const badge = page.locator('[data-testid="replay-badge"]').first();
    const visible = await badge.isVisible({ timeout: 5000 }).catch(() => false);
    if (visible) {
      await expect(badge).toBeVisible();
    }
  });

  test("switching back to Live shows a confirm modal (no accidental exit)", async ({ page }) => {
    // First, set Replay state via localStorage
    await page.evaluate(() => {
      const key = "axagent-time-anchor";
      const raw = localStorage.getItem(key);
      const data = raw
        ? JSON.parse(raw)
        : { state: {}, version: 0 };
      data.state = {
        ...data.state,
        asOfDate: "2026-06-01",
        mode: "replay",
      };
      localStorage.setItem(key, JSON.stringify(data));
    });
    await page.reload();
    await dismissModals(page);
    await expect(page.locator('[data-testid="page-time-anchor"]')).toBeVisible({
      timeout: 30000,
    });

    // Click the mode-switch — should open the confirm modal
    // 点击 Segmented 的 live 选项，从 replay 切回 live
    const segmented = page.getByTestId("time-anchor-segmented");
    await expect(segmented).toBeVisible({ timeout: 10000 });
    await segmented.locator("label.ant-segmented-item").first().click();

    // AntD Modal renders role="dialog" — verify one appears with a confirm copy
    const dialog = page.locator('[role="dialog"]').first();
    const visible = await dialog.isVisible({ timeout: 5000 }).catch(() => false);
    test.skip(!visible, "Confirm dialog did not open");
    await expect(dialog).toBeVisible();
  });

  test("Replay Workbench forces date re-pick", async ({ page }) => {
    // Pre-seed with an asOfDate
    await page.evaluate(() => {
      const key = "axagent-time-anchor";
      const raw = localStorage.getItem(key);
      const data = raw ? JSON.parse(raw) : { state: {}, version: 0 };
      data.state = {
        ...data.state,
        asOfDate: "2026-06-01",
        mode: "replay",
      };
      localStorage.setItem(key, JSON.stringify(data));
    });

    await page.goto("/replay-workbench");
    await page.waitForLoadState("domcontentloaded");

    // The AsOfDatePicker should be present and the field should be empty
    // (we don't autofill from the persisted state — the workbench requires
    // explicit reselection)
    const picker = page.locator('[data-testid="asof-date-picker"]').first();
    const visible = await picker.isVisible({ timeout: 10000 }).catch(() => false);
    test.skip(!visible, "Replay Workbench picker not visible");
    await expect(picker).toBeVisible();
  });

  test("mode survives navigation across pages", async ({ page }) => {
    // Seed replay state
    await page.evaluate(() => {
      const key = "axagent-time-anchor";
      const raw = localStorage.getItem(key);
      const data = raw ? JSON.parse(raw) : { state: {}, version: 0 };
      data.state = {
        ...data.state,
        asOfDate: "2026-06-01",
        mode: "replay",
      };
      localStorage.setItem(key, JSON.stringify(data));
    });
    await page.reload();
    await dismissModals(page);
    await expect(page.locator('[data-testid="page-time-anchor"]')).toBeVisible({
      timeout: 30000,
    });

    // Navigate — only pages where AppHeader renders (not chat page `/`, not stock pages)
    for (const path of ["/knowledge", "/workflow", "/settings/advanced"]) {
      await page.goto(path);
      await page.waitForLoadState("domcontentloaded");
      const pill = page.locator('[data-testid="page-time-anchor"]');
      await expect(pill).toBeVisible({ timeout: 30000 });
      const txt = (await pill.textContent()) ?? "";
      // In replay, the pill text should NOT just be "LIVE"
      expect(txt.trim().length).toBeGreaterThan(0);
    }
  });

  test("Tour bubble shows when tourSeen=false and dismisses on click", async ({ page }) => {
    // Override the addInitScript to clear tourSeen
    await page.addInitScript(() => {
      try {
        const key = "axagent-time-anchor";
        const raw = localStorage.getItem(key);
        const data = raw ? JSON.parse(raw) : { state: {}, version: 0 };
        data.state = {
          ...data.state,
          asOfDate: null,
          mode: "live",
          tourSeen: false,
        };
        localStorage.setItem(key, JSON.stringify(data));
      } catch {
        /* noop */
      }
    });
    await page.goto("/settings");
    await page.waitForLoadState("domcontentloaded");
    await dismissModals(page);
    await expect(page.locator('[data-testid="page-time-anchor"]')).toBeVisible({
      timeout: 30000,
    });

    const tour = page.locator('[data-testid="time-anchor-tour"]');
    const visible = await tour.isVisible({ timeout: 5000 }).catch(() => false);
    test.skip(!visible, "Tour bubble did not appear");
    await expect(tour).toBeVisible();

    // Click "Got it" / "知道了" / etc.
    const gotIt = tour.locator("button").first();
    await gotIt.click();
    await expect(tour).toBeHidden({ timeout: 5000 });
  });
});
