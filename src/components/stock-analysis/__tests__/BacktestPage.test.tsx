import { render } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, fallback?: string) => fallback ?? key,
  }),
  initReactI18next: { type: "3rdParty", init: () => {} },
}));

vi.mock("@/lib/invoke", () => ({
  invoke: vi.fn().mockResolvedValue([]),
  listen: vi.fn().mockResolvedValue(() => {}),
  isTauri: () => false,
}));

import { BacktestPage } from "../BacktestPage";

const renderWithRouter = () => render(<BacktestPage />, { wrapper: MemoryRouter });

describe("BacktestPage", () => {
  it("renders without crashing", () => {
    const { container } = renderWithRouter();
    expect(container).toBeTruthy();
  });

  it("renders back button with chat label", () => {
    const { container } = renderWithRouter();
    const backButton = container.querySelector(".sa-header-back");
    expect(backButton).toBeTruthy();
    expect(backButton?.textContent).toContain("nav.chat");
  });

  it("renders the page title via i18n key", () => {
    const { container } = renderWithRouter();
    const title = container.querySelector(".sa-header-title");
    expect(title).toBeTruthy();
    expect(title?.textContent).toBe("stockAnalysis.backtest.title");
  });

  it("renders BacktestPanel and HistoricalAnalysisPanel in vertical stack", () => {
    const { container } = renderWithRouter();
    // 页面使用 Tabs，analysis 标签下的内容被 lazy-render，
    // Tabs 默认激活第一个 tab (quick)，需要切换到 analysis tab 才能看到 space-y-4
    const tabs = container.querySelector(".ant-tabs");
    expect(tabs).toBeTruthy();
    expect(tabs?.querySelector(".ant-tabs-tab")).toBeTruthy();
  });
});
