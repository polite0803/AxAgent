import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";
import { DecisionBanner } from "../DecisionBanner";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, fallback?: string) => fallback ?? key,
  }),
}));

// Default mock: no decision
const storeState = {
  decision: null as {
    action: string;
    positionPct: number;
    reasoning: string;
    riskLevel: string;
    confidence: number;
    targetPrice?: number;
    stopLoss?: number;
  } | null,
  stockCode: "600519" as string | null,
  stockName: "茅台",
  startAnalysis: vi.fn(),
};

vi.mock("@/stores", () => ({
  useStockAnalysisStore: (selector: (s: typeof storeState) => unknown) => selector(storeState),
  useSettingsStore: (selector: (s: { settings: { theme_mode: string } }) => unknown) =>
    selector({ settings: { theme_mode: "system" } }),
}));

describe("DecisionBanner", () => {
  it("decision 为 null 时渲染'决策缺失'占位卡（不再 firstChild === null）", () => {
    storeState.decision = null;
    storeState.stockCode = "600519";
    const { container } = render(
      <MemoryRouter>
        <DecisionBanner />
      </MemoryRouter>,
    );
    expect(container.firstChild).not.toBeNull();
    expect(screen.getByTestId("decision-banner-missing")).toBeTruthy();
    expect(container.textContent).toContain("stockAnalysis.decisionMissing");
    expect(container.textContent).toContain("stockAnalysis.decisionMissingHint");
  });

  it("占位卡有 stockCode 时显示'重跑分析'按钮", () => {
    storeState.decision = null;
    storeState.stockCode = "600519";
    const { container } = render(
      <MemoryRouter>
        <DecisionBanner />
      </MemoryRouter>,
    );
    expect(screen.getByTestId("decision-banner-missing")).toBeTruthy();
    expect(container.textContent).toContain("stockAnalysis.reAnalyze");
  });

  it("占位卡无 stockCode 时显示'搜索股票'按钮(永远有入口)", () => {
    storeState.decision = null;
    storeState.stockCode = null;
    const { container } = render(
      <MemoryRouter>
        <DecisionBanner />
      </MemoryRouter>,
    );
    expect(screen.getByTestId("decision-banner-missing")).toBeTruthy();
    // 永远有按钮（不重跑就跳到搜索栏），不出现 dead-end
    expect(container.textContent).toContain("stockAnalysis.searchStock");
    expect(container.textContent).toContain("stockAnalysis.reAnalyzeNeedCodeHint");
  });

  it("renders decision info when decision exists", () => {
    storeState.decision = {
      action: "BUY",
      positionPct: 10.0,
      reasoning: "技术面突破",
      riskLevel: "中",
      confidence: 0.8,
      targetPrice: 1850.0,
      stopLoss: 1580.0,
    };
    storeState.stockCode = "600519";
    const { container } = render(
      <MemoryRouter>
        <DecisionBanner />
      </MemoryRouter>,
    );
    expect(container.firstChild).not.toBeNull();
    expect(container.textContent).toContain("stockAnalysis.actionBuy");
    expect(container.textContent).toContain("10%");
  });
});
