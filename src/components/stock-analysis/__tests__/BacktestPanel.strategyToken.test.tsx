import { render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    // 不返回 options 对象,只回 key(避免把 {count: N} 渲染为 React 子节点)
    t: (key: string) => key,
  }),
}));

const timeAnchorState: Record<string, unknown> = {
  asOfDate: null,
  mode: "live",
};
vi.mock("@/stores/feature/timeAnchorStore", () => ({
  useTimeAnchorStore: (selector: (s: typeof timeAnchorState) => unknown) => selector(timeAnchorState),
}));

const invokeMock = vi.fn();
vi.mock("@/lib/invoke", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
  listen: vi.fn(),
  isTauri: () => false,
}));

import { BacktestPanel } from "../BacktestPanel";

beforeEach(() => {
  invokeMock.mockReset();
});

afterEach(() => {
  timeAnchorState.asOfDate = null;
  timeAnchorState.mode = "live";
});

describe("BacktestPanel — R2-Bug-I: 策略回测请求级 token 取消", () => {
  it("快速双击策略回测按钮时,只展示最后一次调用的结果(忽略乱序的早期响应)", async () => {
    // 准备两个可控的 promise
    let resolveFirst!: (v: unknown) => void;
    let resolveSecond!: (v: unknown) => void;
    const firstCall = new Promise<unknown>((r) => {
      resolveFirst = r;
    });
    const secondCall = new Promise<unknown>((r) => {
      resolveSecond = r;
    });

    let strategyCallIndex = 0;
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === "backtest_reco_strategies") {
        strategyCallIndex += 1;
        return strategyCallIndex === 1 ? firstCall : secondCall;
      }
      return null;
    });

    const { container } = render(<BacktestPanel />);

    // 等首次 backtest_all_history 触发完成
    await waitFor(() => expect(invokeMock).toHaveBeenCalled());
    invokeMock.mockClear();
    strategyCallIndex = 0;

    // 找到策略回测按钮并双击
    // 说明: antd Button 在 loading=true 时把点击事件透传到同一个 onClick,
    // 但内部 DOM 会被替换(spinner),所以每次点击前重新查询。
    const findRunBtn = () => {
      const btns = Array.from(container.querySelectorAll("button"));
      const btn = btns.find((b) => {
        const t = b.textContent || "";
        return t.includes("stockAnalysis.backtest.strategyRun")
          || t.includes("stockAnalysis.backtest.strategyRunning");
      });
      if (!btn) { throw new Error("策略回测按钮未找到"); }
      return btn as HTMLElement;
    };
    // 先在 React 状态更新之间点击两次(dispatchEvent 同步触发,不等待 React 提交)
    const btn1 = findRunBtn();
    const btn2 = findRunBtn();
    btn1.click();
    btn2.click();

    // 确认 2 次 invoke("backtest_reco_strategies") 都触发了
    await waitFor(() => {
      const strategyCalls = invokeMock.mock.calls.filter((c) => c[0] === "backtest_reco_strategies");
      expect(strategyCalls.length).toBe(2);
    });

    // 第二个 promise 先 resolve(模拟"后来的先回来")
    resolveSecond({
      positive: { label: "正向(2nd)", stockCount: 99, strategies: {} },
      negative: { label: "负向(2nd)", stockCount: 88, strategies: {} },
      skipped: [],
    });
    await waitFor(() => {
      expect(container.textContent).toContain("正向(2nd)");
    });

    // 然后第一个 promise 再 resolve(此时应被 token 取消,UI 不应回退)
    resolveFirst({
      positive: { label: "正向(1st)", stockCount: 1, strategies: {} },
      negative: { label: "负向(1st)", stockCount: 2, strategies: {} },
      skipped: [],
    });
    // 等一帧让 React 有机会错误地应用第一个的结果
    await new Promise((r) => setTimeout(r, 50));

    // 关键断言:UI 仍展示第二个调用的结果,没有回退到第一个
    expect(container.textContent).toContain("正向(2nd)");
    expect(container.textContent).not.toContain("正向(1st)");
  });
});
