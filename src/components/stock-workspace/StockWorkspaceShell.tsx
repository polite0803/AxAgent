// SPDX-License-Identifier: AGPL-3.0-only

import { AgentStockUIHost } from "@/components/stock-analysis/AgentStockUIHost";
import { UserModeToggle } from "@/components/stock-workspace/_shared/UserModeToggle";
import { ContextSidebar } from "@/components/stock-workspace/ContextSidebar";
import { DecisionHeroBar } from "@/components/stock-workspace/DecisionHeroBar";
import { MobileTabBar } from "@/components/stock-workspace/MobileTabBar";
import { StockSwitcher } from "@/components/stock-workspace/StockSwitcher";
import { AnalysisView } from "@/components/stock-workspace/views/AnalysisView";
import { BacktestView } from "@/components/stock-workspace/views/BacktestView";
import { CompareView } from "@/components/stock-workspace/views/CompareView";
import { MonitorView } from "@/components/stock-workspace/views/MonitorView";
import { ReviewView } from "@/components/stock-workspace/views/ReviewView";
import { TradeView } from "@/components/stock-workspace/views/TradeView";
import { PageTimeAnchor } from "@/components/time-travel/PageTimeAnchor";
import { useUIStore, useWorkspaceStore } from "@/stores";
import type { WorkspaceView } from "@/stores";
import { ArrowLeft, BarChart3, Briefcase, Coins, FlaskConical, GitCompare, RotateCcw } from "lucide-react";
import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useParams, useSearchParams } from "react-router-dom";

/** 桌面端视图 Tab 配置（6 个视图） */
const DESKTOP_TABS: Array<{ key: WorkspaceView; icon: React.ReactNode }> = [
  { key: "analysis", icon: <BarChart3 size={15} /> },
  { key: "monitor", icon: <Briefcase size={15} /> },
  { key: "trade", icon: <Coins size={15} /> },
  { key: "backtest", icon: <FlaskConical size={15} /> },
  { key: "compare", icon: <GitCompare size={15} /> },
  { key: "review", icon: <RotateCcw size={15} /> },
];

/** 所有有效视图（用于 URL 参数校验） */
const VALID_VIEWS: WorkspaceView[] = [
  "analysis",
  "monitor",
  "trade",
  "backtest",
  "compare",
  "review",
  "more",
];

/**
 * 股票工作区壳层 — 单股票全生命周期的统一入口。
 *
 * 三栏布局（桌面端）：
 * - 左栏：StockSwitcher（股票切换器，默认折叠）
 * - 中栏：视图区（6 个视图 Tab）
 * - 右栏：ContextSidebar（上下文侧栏，默认折叠）
 *
 * 顶部永久可见 DecisionHeroBar，跨视图共享决策摘要。
 * 移动端：单栏 + 底部 Tab Bar（4 核心 + 更多）+ 抽屉。
 *
 * 路由：/invest?tab=workspace&stockCode=xxx  →  query 参数驱动当前股票
 * 兼容：/workspace/:stockCode（旧路由，已重定向到 /invest?tab=workspace&stockCode=xxx）
 */
export function StockWorkspaceShell() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { stockCode: pathStockCode } = useParams<{ stockCode?: string }>();
  const [searchParams, setSearchParams] = useSearchParams();
  // stockCode 可来自路径参数（旧路由）或 query 参数（InvestHub tab）
  const urlStockCode = pathStockCode ?? searchParams.get("stockCode");
  const urlView = searchParams.get("view") as string | null;

  const deviceLayout = useUIStore((s) => s.deviceLayout);
  const isMobile = deviceLayout === "mobile";

  const currentStockCode = useWorkspaceStore((s) => s.currentStockCode);
  const currentStockName = useWorkspaceStore((s) => s.currentStockName);
  const currentView = useWorkspaceStore((s) => s.currentView);
  const setCurrentStock = useWorkspaceStore((s) => s.setCurrentStock);
  const setCurrentView = useWorkspaceStore((s) => s.setCurrentView);

  // ── URL → store 同步 ──
  // 路由参数 stockCode 驱动当前股票
  useEffect(() => {
    if (urlStockCode && urlStockCode !== currentStockCode) {
      // URL 带了股票代码，同步到 store（名称从最近列表查找或暂用代码）
      const recent = useWorkspaceStore.getState().recentStocks.find((s) => s.code === urlStockCode);
      setCurrentStock(urlStockCode, recent?.name ?? urlStockCode);
    }
  }, [urlStockCode, currentStockCode, setCurrentStock]);

  // URL ?view= 驱动当前视图
  useEffect(() => {
    if (urlView && urlView !== currentView) {
      if (VALID_VIEWS.includes(urlView as WorkspaceView)) {
        setCurrentView(urlView as WorkspaceView);
      }
    }
  }, [urlView, currentView, setCurrentView]);

  // ── 渲染当前视图 ──
  const renderView = () => {
    switch (currentView) {
      case "analysis":
        return <AnalysisView />;
      case "monitor":
        return <MonitorView />;
      case "trade":
        return <TradeView />;
      case "backtest":
        return <BacktestView />;
      case "compare":
        return <CompareView />;
      case "review":
        return <ReviewView />;
      case "more":
        // 移动端"更多"视图：展示回测 + 对比入口
        return (
          <div className="flex flex-col h-full p-4 gap-3">
            <div className="text-sm" style={{ color: "var(--muted)" }}>
              {t("workspace.view.moreHint")}
            </div>
            <button
              type="button"
              onClick={() => setCurrentView("backtest")}
              className="flex items-center gap-3 p-4 rounded-lg text-left transition-colors hover:opacity-80"
              style={{ background: "var(--surface)", border: "1px solid var(--border)" }}
            >
              <FlaskConical size={24} style={{ color: "var(--accent)" }} />
              <div>
                <div className="font-semibold">{t("workspace.view.backtest")}</div>
                <div className="text-sm" style={{ color: "var(--muted)" }}>
                  {t("workspace.view.backtestHint")}
                </div>
              </div>
            </button>
            <button
              type="button"
              onClick={() => setCurrentView("compare")}
              className="flex items-center gap-3 p-4 rounded-lg text-left transition-colors hover:opacity-80"
              style={{ background: "var(--surface)", border: "1px solid var(--border)" }}
            >
              <GitCompare size={24} style={{ color: "var(--accent)" }} />
              <div>
                <div className="font-semibold">{t("workspace.view.compare")}</div>
                <div className="text-sm" style={{ color: "var(--muted)" }}>
                  {t("workspace.view.compareHint")}
                </div>
              </div>
            </button>
          </div>
        );
      default:
        return null;
    }
  };

  // ── 桌面端视图切换器 ──
  const desktopTabBar = (
    <div
      className="flex items-center gap-1 px-3 py-1.5"
      style={{ borderBottom: "1px solid var(--border)", flexShrink: 0 }}
    >
      {DESKTOP_TABS.map((tab) => {
        const isActive = currentView === tab.key;
        return (
          <button
            key={tab.key}
            type="button"
            onClick={() => {
              setCurrentView(tab.key);
              // 同步 URL（避免 URL→store effect 反向覆盖）
              const next = new URLSearchParams(searchParams);
              next.set("view", tab.key);
              setSearchParams(next, { replace: true });
            }}
            className="flex items-center gap-1.5 px-3 py-1 rounded text-sm transition-colors"
            style={{
              background: isActive ? "var(--accent)" : "transparent",
              color: isActive ? "white" : "var(--muted)",
            }}
          >
            {tab.icon}
            {t(`workspace.view.${tab.key}`)}
          </button>
        );
      })}
    </div>
  );

  // ── 移动端布局 ──
  if (isMobile) {
    return (
      <div className="flex flex-col h-full min-h-0" style={{ overflow: "hidden" }}>
        {/* 顶部 Header */}
        <div
          className="flex items-center gap-2 px-3 py-2"
          style={{ borderBottom: "1px solid var(--border)", flexShrink: 0 }}
        >
          <button
            type="button"
            onClick={() => navigate("/")}
            className="p-1 rounded hover:opacity-70"
          >
            <ArrowLeft size={18} />
          </button>
          <span className="text-sm font-semibold flex-1 truncate">
            {currentStockName ?? t("workspace.title")}
          </span>
          <PageTimeAnchor />
        </div>

        {/* DecisionHeroBar */}
        <div className="px-2 py-1.5" style={{ flexShrink: 0 }}>
          <DecisionHeroBar />
        </div>

        {/* 视图区（唯一滚动入口） */}
        <div className="flex-1 min-h-0 overflow-y-auto overflow-x-hidden">
          {renderView()}
        </div>
        {/* [AxInvest] Agent 动态 UI 容器（移动端） */}
        <AgentStockUIHost />

        {/* 底部 Tab Bar */}
        <MobileTabBar />
      </div>
    );
  }

  // ── 桌面端/平板布局 ──
  return (
    <div className="ax-stock-workspace flex flex-col h-full min-h-0" style={{ overflow: "hidden" }}>
      {/* 顶部 Header */}
      <div
        className="flex items-center gap-3 px-3 py-2"
        style={{ borderBottom: "1px solid var(--border)", flexShrink: 0 }}
      >
        <button
          type="button"
          onClick={() => navigate("/")}
          className="flex items-center gap-1 p-1 rounded hover:opacity-70"
        >
          <ArrowLeft size={16} />
          <span className="text-sm">{t("nav.chat")}</span>
        </button>
        <span className="text-sm" style={{ color: "var(--muted)" }}>|</span>
        <h2 className="text-sm font-semibold flex-1">
          {t("workspace.title")}
          {currentStockName && (
            <span className="ml-2" style={{ color: "var(--muted)" }}>
              · {currentStockName} ({currentStockCode})
            </span>
          )}
        </h2>
        <PageTimeAnchor />
        <UserModeToggle />
      </div>

      {/* DecisionHeroBar — 永久可见，跨视图共享 */}
      <div className="px-3 py-2" style={{ flexShrink: 0, minWidth: 0, maxWidth: "100%" }}>
        <DecisionHeroBar />
      </div>

      {/* 三栏布局 */}
      <div className="flex-1 flex overflow-hidden min-h-0">
        {/* 左栏：股票切换器 */}
        <StockSwitcher />

        {/* 中栏：视图区 */}
        <div className="flex-1 flex flex-col overflow-hidden min-h-0">
          {desktopTabBar}
          {/* 视图容器：单视图在大内容时竖向滚动（唯一滚动入口） */}
          <div className="flex-1 min-h-0 overflow-y-auto overflow-x-hidden">
            {renderView()}
          </div>
          {/* [AxInvest] Agent 动态 UI 容器：跨视图常驻，只显示 targetId=stock-workspace 的渲染 */}
          <AgentStockUIHost />
        </div>

        {/* 右栏：上下文侧栏 */}
        <ContextSidebar />
      </div>
    </div>
  );
}
