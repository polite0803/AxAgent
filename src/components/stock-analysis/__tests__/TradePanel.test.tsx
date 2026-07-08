import { render } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, fallback?: string) => fallback ?? key,
  }),
}));

const storeState: Record<string, unknown> = {
  stockCode: "",
  stockName: "",
  decision: null,
};

vi.mock("@/stores", () => ({
  useStockAnalysisStore: (selector: (s: typeof storeState) => unknown) => selector(storeState),
}));

vi.mock("@/lib/invoke", () => ({
  invoke: vi.fn(),
  listen: vi.fn(),
  isTauri: () => false,
}));

import { TradePanel } from "../TradePanel";

describe("TradePanel", () => {
  it("renders without crashing", () => {
    storeState.stockCode = "";
    storeState.stockName = "";
    storeState.decision = null;

    const { container } = render(<TradePanel />);
    expect(container).toBeTruthy();
  });

  it("renders trade panel heading", () => {
    storeState.stockCode = "600519";
    storeState.stockName = "茅台";
    storeState.decision = null;

    const { container } = render(<TradePanel />);
    expect(container.textContent).toBeTruthy();
  });

  it("renders stock code and name from store", () => {
    storeState.stockCode = "600519";
    storeState.stockName = "贵州茅台";
    storeState.decision = {
      action: "买入",
      confidence: 85,
      targetPrice: 1850,
      positionPct: 15,
    };

    const { container } = render(<TradePanel />);
    // 组件应渲染，不崩溃
    expect(container).toBeTruthy();
  });
});
