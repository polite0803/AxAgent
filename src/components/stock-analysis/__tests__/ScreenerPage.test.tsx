import { render } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, opts?: unknown) => {
      if (typeof opts === "string") { return opts; }
      if (opts && typeof opts === "object" && "defaultValue" in opts) {
        return String((opts as { defaultValue: string }).defaultValue);
      }
      return key;
    },
    // RecommendationPanel accesses i18n.language in useCallback deps.
    // Provide a stub so the panel can mount under the test mock.
    i18n: { language: "en-US" },
  }),
  initReactI18next: { type: "3rdParty", init: () => {} },
}));

vi.mock("@/lib/invoke", () => ({
  invoke: vi.fn().mockResolvedValue([]),
  listen: vi.fn().mockResolvedValue(() => {}),
  isTauri: () => false,
}));

import { ScreenerPage } from "../ScreenerPage";

const renderWithRouter = () => render(<ScreenerPage />, { wrapper: MemoryRouter });

describe("ScreenerPage", () => {
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
    expect(title?.textContent).toBe("screener.title");
  });

  it("renders a single Tabs container with smart-reco as default", () => {
    const { container } = renderWithRouter();
    const tabs = container.querySelector(".ant-tabs");
    expect(tabs).toBeTruthy();
    // Only count the page-level Tabs header (not the nested period Tabs
    // inside RecommendationPanel, which has 3 short/mid/long entries).
    const navHeaders = tabs?.querySelectorAll(":scope > .ant-tabs-nav .ant-tabs-tab");
    expect(navHeaders?.length).toBe(3);
    // Default active tab should be the smart-reco tab.
    const activeHeader = tabs?.querySelector(".ant-tabs-nav .ant-tabs-tab-active");
    expect(activeHeader?.textContent).toContain("screener.tab.smartReco");
  });

  it("renders all three tab labels via i18n", () => {
    const { container } = renderWithRouter();
    expect(container.textContent).toContain("screener.tab.smartReco");
    expect(container.textContent).toContain("screener.tab.myFilter");
    expect(container.textContent).toContain("screener.tab.serenity");
  });

  it("renders 3 accordion items for HotStocks / LimitUp / DragonTiger", () => {
    const { container } = renderWithRouter();
    // antd Collapse 渲染为 .ant-collapse,包含 N 个 .ant-collapse-item
    const collapse = container.querySelector(".ant-collapse");
    expect(collapse).toBeTruthy();
    const items = collapse?.querySelectorAll(".ant-collapse-item");
    expect(items?.length).toBe(3);
  });

  it("defaults to all accordion items collapsed", () => {
    const { container } = renderWithRouter();
    const expanded = container.querySelectorAll(".ant-collapse-item-active");
    expect(expanded.length).toBe(0);
  });
});
