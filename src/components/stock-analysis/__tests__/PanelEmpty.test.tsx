import { render } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { PanelEmpty } from "../PanelEmpty";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, opts?: Record<string, unknown>) => {
      if (opts && "n" in opts) { return `${key}#${opts.n}`; }
      if (opts && "reason" in opts) { return `${key}#${opts.reason}`; }
      return key;
    },
  }),
}));

// timeAnchorStore mock with mutable mode
const anchorState: { mode: "live" | "replay" | "backtest_sweep"; degradationCount: number } = {
  mode: "live",
  degradationCount: 0,
};

vi.mock("@/stores/feature/timeAnchorStore", () => ({
  useTimeAnchorStore: (selector: (s: typeof anchorState) => unknown) => selector(anchorState),
}));

describe("PanelEmpty (缺陷 F 修复: replay 自动检测)", () => {
  it("live 模式 + noData → 渲染默认 Empty", () => {
    anchorState.mode = "live";
    anchorState.degradationCount = 0;
    const { container } = render(<PanelEmpty kind="noData" />);
    // 没有 replay-degraded testid
    expect(container.querySelector('[data-testid="panel-empty-replay-degraded"]')).toBeNull();
    // Empty 的 ant-empty-image 应存在
    expect(container.querySelector(".ant-empty")).not.toBeNull();
  });

  it("replay 模式 + noData → 自动升级为 replayDegraded Alert", () => {
    anchorState.mode = "replay";
    anchorState.degradationCount = 3;
    const { container } = render(<PanelEmpty kind="noData" />);
    // 触发 alert
    expect(container.querySelector('[data-testid="panel-empty-replay-degraded"]')).not.toBeNull();
    // description 应包含 count
    expect(container.textContent).toContain("replayDegradedWithCount#3");
  });

  it("backtest_sweep 模式 + noData → 同样升级", () => {
    anchorState.mode = "backtest_sweep";
    anchorState.degradationCount = 0;
    const { container } = render(<PanelEmpty kind="noData" />);
    expect(container.querySelector('[data-testid="panel-empty-replay-degraded"]')).not.toBeNull();
  });

  it("replay 模式 + 显式 kind=replayDegraded + reason → 渲染具体原因", () => {
    anchorState.mode = "live"; // 即便 mode 不是 replay,显式 kind 也生效
    const { container } = render(<PanelEmpty kind="replayDegraded" reason="money_flow 无日期参数" />);
    expect(container.querySelector('[data-testid="panel-empty-replay-degraded"]')).not.toBeNull();
    expect(container.textContent).toContain("replayDegradedWithReason#money_flow 无日期参数");
  });

  it("replay 模式 + degradationCount=0 → 渲染默认 replayDegraded 文案", () => {
    anchorState.mode = "replay";
    anchorState.degradationCount = 0;
    const { container } = render(<PanelEmpty kind="noData" />);
    expect(container.textContent).toContain("stockAnalysis.empty.replayDegraded");
  });
});
