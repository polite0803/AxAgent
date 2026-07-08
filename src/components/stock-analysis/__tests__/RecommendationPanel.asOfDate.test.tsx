import i18n from "@/i18n";
import { useTimeAnchorStore } from "@/stores/feature/timeAnchorStore";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { I18nextProvider } from "react-i18next";
import { MemoryRouter } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { RecommendationPanel } from "../RecommendationPanel";

const invokeMock = vi.fn();
vi.mock("@/lib/invoke", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
  listen: vi.fn().mockResolvedValue(() => {}),
  isTauri: () => false,
}));

function renderWithProviders() {
  return render(
    <MemoryRouter>
      <I18nextProvider i18n={i18n}>
        <RecommendationPanel />
      </I18nextProvider>
    </MemoryRouter>,
  );
}

/** 在 Card extra 中寻找"刷新"按钮（避免匹配到空状态文案中的"刷新"） */
function findRefreshButton() {
  const card = document.querySelector(".ant-card-extra");
  if (!card) { throw new Error("card extra not found"); }
  const btn = card.querySelector("button");
  if (!btn) { throw new Error("refresh button not found"); }
  return btn;
}

beforeEach(() => {
  invokeMock.mockReset();
  // 默认所有 invoke 都返回"无数据"响应,避免测试间相互污染
  invokeMock.mockResolvedValue({
    period: "short",
    picks: {},
    disabledStyles: [],
    generatedAt: Date.now(),
    rawSeedPoolSize: 0,
  });
  useTimeAnchorStore.setState({
    asOfDate: null,
    mode: "live",
    tourSeen: true,
    pendingLiveConfirm: false,
  });
});

afterEach(() => {
  useTimeAnchorStore.setState({ asOfDate: null, mode: "live" });
});

/**
 * 找到 invoke 第一次被以 `command` 名称调用的参数
 */
function findCall(command: string): unknown[] | undefined {
  return invokeMock.mock.calls.find((c) => c[0] === command);
}

describe("RecommendationPanel — as-of propagation", () => {
  it("live 模式: 挂载时调用 get_cached_recommendation(period),不带 asOfDate", async () => {
    renderWithProviders();
    // 等待特定的命令被调用,避免只等任意 call(可能只等到 backtest)
    await waitFor(() => expect(findCall("get_cached_recommendation")).toBeDefined());
    const call = findCall("get_cached_recommendation");
    expect(call).toBeDefined();
    // args[1] 是 { period } 对象,确认不含 asOfDate
    expect(call?.[1]).toEqual({ period: "short" });
  });

  it("replay 模式: 挂载时调用 recommend_stocks(period, asOfDate),带 asOfDate", async () => {
    useTimeAnchorStore.setState({ asOfDate: "2026-06-01", mode: "replay" });
    renderWithProviders();
    // replay 模式下 loadCache 走 fallback,直接调 recommend_stocks
    await waitFor(() => expect(findCall("recommend_stocks")).toBeDefined());
    const call = findCall("recommend_stocks");
    expect(call).toBeDefined();
    expect(call?.[1]).toMatchObject({ period: "short", asOfDate: "2026-06-01" });
  });

  it("live 模式: 不调用 recommend_stocks (用户没点刷新)", async () => {
    renderWithProviders();
    await waitFor(() => expect(invokeMock).toHaveBeenCalled());
    expect(findCall("recommend_stocks")).toBeUndefined();
  });

  it("replay 模式: 不调用 get_cached_recommendation (缓存永远是 live 产物)", async () => {
    useTimeAnchorStore.setState({ asOfDate: "2026-06-01", mode: "replay" });
    renderWithProviders();
    await waitFor(() => expect(invokeMock).toHaveBeenCalled());
    expect(findCall("get_cached_recommendation")).toBeUndefined();
  });

  it("shows a replay banner when in replay mode", async () => {
    useTimeAnchorStore.setState({ asOfDate: "2026-06-01", mode: "replay" });
    renderWithProviders();
    await waitFor(() => {
      expect(
        screen.getByText(
          i18n.t("timeTravel.recommendationBanner", { date: "2026-06-01" }),
        ),
      ).toBeInTheDocument();
    });
  });

  it("does NOT show a replay banner in live mode", async () => {
    renderWithProviders();
    await waitFor(() => expect(invokeMock).toHaveBeenCalled());
    expect(
      screen.queryByText(
        i18n.t("timeTravel.recommendationBanner", { date: "" }),
      ),
    ).toBeNull();
  });

  it("缓存命中时显示「缓存」灰底标签", async () => {
    // 给 get_cached_recommendation 返回有数据的响应
    const cachedTs = Date.now();
    invokeMock.mockReset();
    invokeMock.mockResolvedValue({
      period: "short",
      picks: {
        trend: [{
          stockCode: "000001",
          stockName: "平安银行",
          style: "trend",
          period: "short",
          price: 10,
          entryLow: 9,
          entryHigh: 11,
          stopLoss: 8.5,
          targetPrice: 12,
          positionPct: 10,
          holdingDays: 5,
          confidence: 80,
          reasons: ["测试"],
          riskNotes: [],
          secondaryStyles: [],
          synthetic: false,
        }],
      },
      disabledStyles: [],
      generatedAt: cachedTs,
      rawSeedPoolSize: 10,
      mode: "cached",
    });
    renderWithProviders();
    await waitFor(() => {
      expect(screen.getByTestId("reco-cached-badge")).toBeInTheDocument();
    });
  });

  it("缓存为空时显示 emptyNoCache 文案", async () => {
    // get_cached_recommendation 返回 null → 无缓存
    invokeMock.mockReset();
    invokeMock.mockResolvedValue(null);
    renderWithProviders();
    // 缓存为空 → loadCache 设 emptyKind=noData → 显示 emptyNoCache
    await waitFor(() => {
      expect(
        screen.getByText(i18n.t("stockAnalysis.recommendation.emptyNoCache")),
      ).toBeInTheDocument();
    });
  });

  it("live 模式下点击刷新按钮调用 recommend_stocks", async () => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValue(null);
    renderWithProviders();
    await waitFor(() => expect(invokeMock).toHaveBeenCalled());
    // 等待 loading 结束（antd 中 "刷新" 按钮含空格,用 card-extra 范围避免匹配空文案）
    await waitFor(() => {
      const btn = findRefreshButton();
      expect(btn).not.toHaveClass("ant-btn-loading");
    });
    invokeMock.mockClear();
    invokeMock.mockResolvedValue({
      period: "short",
      picks: {},
      disabledStyles: [],
      generatedAt: Date.now(),
      rawSeedPoolSize: 0,
    });
    fireEvent.click(findRefreshButton());
    await waitFor(() => expect(findCall("recommend_stocks")).toBeDefined());
    expect(findCall("recommend_stocks")?.[1]).toEqual({ period: "short", asOfDate: null });
  });

  it("切换 period 重新调用 get_cached_recommendation(newPeriod)", async () => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValue(null);
    renderWithProviders();
    await waitFor(() => expect(findCall("get_cached_recommendation")).toBeDefined());
    await waitFor(() => {
      const btn = findRefreshButton();
      expect(btn).not.toHaveClass("ant-btn-loading");
    });
    invokeMock.mockClear();
    invokeMock.mockResolvedValue(null);
    fireEvent.click(screen.getByText(i18n.t("stockAnalysis.recommendation.periodMid")));
    await waitFor(() => expect(findCall("get_cached_recommendation")).toBeDefined());
    expect(findCall("get_cached_recommendation")?.[1]).toEqual({ period: "mid" });
  });

  it("replay 模式下点击刷新按钮调用 recommend_stocks 带 asOfDate", async () => {
    useTimeAnchorStore.setState({ asOfDate: "2026-06-01", mode: "replay" });
    invokeMock.mockReset();
    invokeMock.mockResolvedValue({
      period: "short",
      picks: {},
      disabledStyles: [],
      generatedAt: Date.now(),
      rawSeedPoolSize: 0,
    });
    renderWithProviders();
    await waitFor(() => expect(findCall("recommend_stocks")).toBeDefined());
    await waitFor(() => {
      const btn = findRefreshButton();
      expect(btn).not.toHaveClass("ant-btn-loading");
    });
    invokeMock.mockClear();
    invokeMock.mockResolvedValue({
      period: "short",
      picks: {},
      disabledStyles: [],
      generatedAt: Date.now(),
      rawSeedPoolSize: 0,
    });
    fireEvent.click(findRefreshButton());
    await waitFor(() => expect(invokeMock).toHaveBeenCalled());
    expect(findCall("recommend_stocks")?.[1]).toMatchObject({ period: "short", asOfDate: "2026-06-01" });
  });
});
