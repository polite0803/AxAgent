import { StockAnalysisSettings } from "@/components/settings/StockAnalysisSettings";
import { PageErrorBoundary } from "@/components/shared/ErrorBoundary";
import { PageTimeAnchor } from "@/components/time-travel/PageTimeAnchor";
import { extractLlmField } from "@/lib/agentOutput";
import { invoke } from "@/lib/invoke";
import { classifySentiment } from "@/lib/stock-analysis-utils";
import { useStockAnalysisStore, useUIStore } from "@/stores";
import { useTimeAnchorStore } from "@/stores/feature/timeAnchorStore";
import { Button, Collapse, Dropdown } from "antd";
import {
  ArrowLeftRight,
  Coins,
  LineChart,
  RotateCcw,
  Settings,
  Shield,
  Sparkles,
  SplitSquareHorizontal,
  Users,
  X,
} from "lucide-react";
import { type ReactNode, useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useParams, useSearchParams } from "react-router-dom";
import { AnalysisDebugPanel } from "./AnalysisDebugPanel";
import { AnalysisProgress } from "./AnalysisProgress";
import { AnalystReportGrid } from "./AnalystReportGrid";
import { AnnouncementsPanel } from "./AnnouncementsPanel";
import { ClsFlashPanel } from "./ClsFlashPanel";
import { ConceptBlocksPanel } from "./ConceptBlocksPanel";
import { DebatePanel } from "./DebatePanel";
import { DecisionBanner } from "./DecisionBanner";
import { ExperimentSidebar } from "./ExperimentSidebar";
import { ExperimentTrail } from "./ExperimentTrail";
import { resolveTimelineJump } from "./timelineJump";
import "./dual-view";
import { AnalysisHistoryButton } from "./AnalysisHistoryButton";
import { DualViewRenderer } from "./dual-view";
import { EventCalendarPanel } from "./EventCalendarPanel";

/** 决策双视角 Tab 内容：从 store 取 LLM vs 公式决策数据传入 DualViewRenderer */
function DecisionComparisonTabContent() {
  const store = useStockAnalysisStore();
  const dualViewData = {
    decisionAction: store.decision?.action,
    decisionPositionPct: store.decision?.positionPct,
    confidence: store.decision?.confidence,
    adjustedConfidence: store.decision?.adjustedConfidence ?? null,
    decisionReasoning: store.decision?.reasoning ?? null,
    llmDecisionAction: (extractLlmField(store.llmDecisionJson, "action") as string | null)
      ?? (extractLlmField(store.llmDecisionJson, "stance") as string | null)
      ?? null,
    llmDecisionPositionPct: (extractLlmField(store.llmDecisionJson, "positionPct") as number | null) ?? null,
    llmConfidence: (extractLlmField(store.llmDecisionJson, "confidence") as number | null) ?? null,
    llmDecisionReasoning: (extractLlmField(store.llmDecisionJson, "reasoning") as string | null)
      ?? (extractLlmField(store.llmDecisionJson, "summary") as string | null)
      ?? null,
    decisionAgreementScore: store.decisionAgreementScore,
    agreementBreakdown: store.decision?.agreementBreakdown ?? null,
  };
  return <DualViewRenderer id="decision-comparison" data={dualViewData} defaultMode="panel" />;
}
import { EvolutionDriftPanel } from "./EvolutionDriftPanel";
import { IndexQuotesPanel } from "./IndexQuotesPanel";
import { IndustryRankingPanel } from "./IndustryRankingPanel";
import { InvestDashboard } from "./InvestDashboard";
import { KLineChart } from "./KLineChart";
import { NorthBoundPanel } from "./NorthBoundPanel";
import { OptionPcrPanel } from "./OptionPcrPanel";
import { PositionsMiniPanel } from "./PositionsMiniPanel";
import { ReflectionPanel } from "./ReflectionPanel";
import { RiskMatrix } from "./RiskMatrix";
import { SectorHeatmapPanel } from "./SectorHeatmapPanel";
import { StockAnalysisPageContext } from "./StockAnalysisPageContext";
import { StockAnalysisSettingsModal } from "./StockAnalysisSettingsModal";
import { StockQuoteCard } from "./StockQuoteCard";
import { StockSearchBar } from "./StockSearchBar";
import { ValueAssessmentPanel } from "./ValueAssessmentPanel";

const PERIOD_MAP: Record<string, { period: string; limit: number }> = {
  "1m": { period: "daily", limit: 22 },
  "3m": { period: "daily", limit: 66 },
  "6m": { period: "daily", limit: 120 },
  "1y": { period: "daily", limit: 250 },
  "weekly": { period: "weekly", limit: 104 },
  "monthly": { period: "monthly", limit: 60 },
};

interface SheetPanel {
  key: string;
  label: string;
  element: ReactNode;
}

export function StockAnalysisPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { id } = useParams<{ id: string }>();
  const loadAnalysis = useStockAnalysisStore((s) => s.loadAnalysis);
  const status = useStockAnalysisStore((s) => s.status);
  const [showDebug, setShowDebug] = useState(false);
  const error = useStockAnalysisStore((s) => s.error);
  const failedNodes = useStockAnalysisStore((s) => s.failedNodes);
  const failedNodeErrors = useStockAnalysisStore((s) => s.failedNodeErrors);
  const stockCode = useStockAnalysisStore((s) => s.stockCode);
  const startAnalysis = useStockAnalysisStore((s) => s.startAnalysis);
  const getStockQuote = useStockAnalysisStore((s) => s.getStockQuote);
  const getStockKline = useStockAnalysisStore((s) => s.getStockKline);
  const klinePeriod = useStockAnalysisStore((s) => s.klinePeriod);

  const deviceLayout = useUIStore((s) => s.deviceLayout);
  const isMobile = deviceLayout === "mobile" || deviceLayout === "tablet";

  // 时间旅行: 监听全局 TimeAnchor,用于 sa-header 的 L2 视觉信号(回放半透紫色遮罩)
  const timeAnchorMode = useTimeAnchorStore((s) => s.mode);
  const isReplay = timeAnchorMode === "replay" || timeAnchorMode === "backtest_sweep";

  const [searchParams] = useSearchParams();
  const [activeTab, setActiveTab] = useState("market");
  const [sheetOpen, setSheetOpen] = useState(false);
  const [sheetTab, setSheetTab] = useState("trade");
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [settingsDefaultTab, setSettingsDefaultTab] = useState<string | undefined>(undefined);
  const [marketStatus, setMarketStatus] = useState("");
  const [expandedFailedNode, setExpandedFailedNode] = useState<string | null>(null);

  const openDataSourceSettings = useCallback(() => {
    setSettingsDefaultTab("data");
    setSettingsOpen(true);
  }, []);

  useEffect(() => {
    let cancelled = false;
    invoke<{ status: string }>("get_market_status").then((r) => {
      if (!cancelled) { setMarketStatus(r.status); }
    }).catch((e) => {
      if (!cancelled) { console.warn("[StockAnalysis] get_market_status failed:", e); }
    });
    return () => {
      cancelled = true;
    };
  }, []);

  // 注意：setupEventListener 改为仅在 startAnalysis 内调用。
  // 之前 L76-78 在页面挂载时也调用，startAnalysis 已 await setupEventListener，
  // 存在竞态——两个 listen 会同时进行，后一个 set({_unlisten}) 覆盖前一次，
  // 前一次的 3 个监听句柄变成孤儿（永远不会 unlisten）。
  // 现在挂载时不再注册，只在用户点击"开始分析"时由 startAnalysis 负责注册。

  useEffect(() => {
    let cancelled = false;
    const code = searchParams.get("code");
    if (code) {
      // 立即把 code 写入 searchKeyword，让 StockSearchBar 输入框显示股票代码
      useStockAnalysisStore.setState({ searchKeyword: code });
      getStockQuote(code).then(() => {
        if (cancelled) { return; }
        // getStockQuote 完成后，stockName 已写入 store，把 searchKeyword 更新为 "名称(code)" 格式
        const { stockCode, stockName } = useStockAnalysisStore.getState();
        if (stockCode) {
          useStockAnalysisStore.setState({
            searchKeyword: stockName ? `${stockName} (${stockCode})` : stockCode,
          });
        }
      });
      const kp = PERIOD_MAP[klinePeriod] ?? PERIOD_MAP["6m"];
      getStockKline(code, kp.period, kp.limit).then(() => {
        if (cancelled) { return; }
      });
    }
    return () => {
      cancelled = true;
    };
  }, [searchParams, getStockQuote, getStockKline, klinePeriod]);

  useEffect(() => {
    if (!id) { return; }
    // ── 同步预填：避免 loadAnalysis 异步期间(或失败时)占位卡显示"搜索股票"按钮 ──
    // 用户从 AnalysisHistoryButton 进入时,history 缓存必有该条,
    // 立即把 stockCode / stockName / analysisId 写进 store,
    // DecisionBanner 占位卡就能直接渲染"重跑分析"按钮,
    // 而不是默认空 stockCode 走"搜索股票"分支。
    const state = useStockAnalysisStore.getState();
    const cached = state.history.find((h) => h.id === id);
    useStockAnalysisStore.setState({
      analysisId: id,
      ...(cached
        ? { stockCode: cached.stockCode, stockName: cached.stockName }
        : {}),
    });

    let cancelled = false;
    loadAnalysis(id).then(() => {
      if (cancelled) { return; }
      const code = useStockAnalysisStore.getState().stockCode;
      if (code) {
        getStockQuote(code);
        const kp = PERIOD_MAP[useStockAnalysisStore.getState().klinePeriod] ?? PERIOD_MAP["6m"];
        getStockKline(code, kp.period, kp.limit);
      }
    }).catch((e) => {
      if (cancelled) { return; }
      // 修复:loadAnalysis 失败时把错误暴露到 store,占位卡渲染"重试"按钮
      // 而不是永远停在"搜索股票"分支(用户认为这违反"history 已有 stockCode"的预期)
      const msg = e instanceof Error ? e.message : String(e);
      console.error("[StockAnalysis] loadAnalysis failed:", msg);
      useStockAnalysisStore.setState({ error: msg });
    });
    return () => {
      cancelled = true;
    };
  }, [id, loadAnalysis, getStockQuote, getStockKline]);

  // 离开分析页时重置时间锚点为 live，避免影响其他页面（如选股/荐股）
  useEffect(() => {
    return () => {
      const ta = useTimeAnchorStore.getState();
      if (ta.mode !== "live") {
        ta.enterLive(false);
      }
    };
  }, []);

  // Decision Timeline 证据芯片 → 切换主 tab / 打开 sheet panel / 滚到决策 banner
  // (useRightPanel 派发的 timeline-jump 事件)
  // 格式: timelineJump = "<tabKey>:<panelKey>"
  //   tabKey  = "market" | "analyze" | "execute"   (抽象层，由 evidence 表定义)
  //   panelKey = evidence 指向的"具体面板"，可能落在：
  //     - 主区 tab（market/analysts/debate/value/risk/reflection/evolution）
  //     - 侧栏 sheet panel（concepts/announcements/...）
  //     - 决策 hero（decision，顶部 DecisionBanner）
  //     - 交易页（trade，跳转路由）
  useEffect(() => {
    const handle = () => {
      const raw = searchParams.get("timelineJump");
      if (!raw) { return; }
      const [tabKey, panelKey] = raw.split(":");
      const plan = resolveTimelineJump(tabKey, panelKey);
      if (plan.activeTab) { setActiveTab(plan.activeTab); }
      if (plan.sheetTab) {
        setSheetTab(plan.sheetTab);
        if (!sheetOpen) { setSheetOpen(true); }
      }
      if (plan.navigateTo) { navigate(plan.navigateTo); }
      if (plan.scrollTo) {
        const el = document.getElementById(plan.scrollTo);
        el?.scrollIntoView({ behavior: "smooth", block: "start" });
      }
    };
    window.addEventListener("timeline-jump", handle);
    return () => window.removeEventListener("timeline-jump", handle);
  }, [searchParams, navigate, sheetOpen]);

  const tabs = useMemo(() => [
    {
      key: "market",
      label: t("stockAnalysis.tab.market"),
      icon: <LineChart size={14} />,
      children: (
        <>
          <StockQuoteCard />
          <KLineChart />
        </>
      ),
    },
    {
      key: "analysts",
      label: t("stockAnalysis.tab.analysts"),
      icon: <Users size={14} />,
      children: <AnalystReportGrid />,
    },
    {
      key: "debate",
      label: t("stockAnalysis.tab.debate"),
      icon: <ArrowLeftRight size={14} />,
      children: <DebatePanel />,
    },
    {
      key: "value",
      label: t("stockAnalysis.tab.value"),
      icon: <Coins size={14} />,
      children: <ValueAssessmentPanel />,
    },
    { key: "risk", label: t("stockAnalysis.tab.risk"), icon: <Shield size={14} />, children: <RiskMatrix /> },
    {
      key: "decision-comparison",
      label: t("stockAnalysis.tab.decision"),
      icon: <SplitSquareHorizontal size={14} />,
      children: <DecisionComparisonTabContent />,
    },
    // Decision tab removed — now rendered as full-width hero at top
    {
      key: "reflection",
      label: t("stockAnalysis.tab.reflection"),
      icon: <RotateCcw size={14} />,
      children: <ReflectionPanel />,
    },
    {
      key: "evolution",
      label: t("stockAnalysis.tab.evolution"),
      icon: <Sparkles size={14} />,
      children: <EvolutionDriftPanel />,
    },
  ], [t]);

  // [Phase 2] 决策一致性胶囊 "切换 tab" 事件（tabs 定义之后，确保引用有效性）
  useEffect(() => {
    const handleSwitchTab = (e: Event) => {
      const tab = (e as CustomEvent).detail;
      if (typeof tab === "string" && tabs.some((t) => t.key === tab)) {
        setActiveTab(tab);
      }
    };
    window.addEventListener("switch-tab", handleSwitchTab);
    return () => window.removeEventListener("switch-tab", handleSwitchTab);
  }, [tabs]);

  const allSheetPanels: SheetPanel[] = [
    { key: "holdings", label: t("stockAnalysis.holdingsSheet"), element: <PositionsMiniPanel /> },
    { key: "index", label: t("stockAnalysis.indexQuotes"), element: <IndexQuotesPanel /> },
    { key: "sectors", label: t("stockAnalysis.settings.sheet.sectors"), element: <SectorHeatmapPanel /> },
    { key: "north", label: t("stockAnalysis.settings.sheet.north"), element: <NorthBoundPanel /> },
    { key: "events", label: t("stockAnalysis.settings.sheet.events"), element: <EventCalendarPanel /> },
    {
      key: "announcements",
      label: t("stockAnalysis.announcements"),
      element: <AnnouncementsPanel stockCode={stockCode} />,
    },
    { key: "concepts", label: t("stockAnalysis.conceptBlocks"), element: <ConceptBlocksPanel stockCode={stockCode} /> },
    { key: "optionpcr", label: t("stockAnalysis.optionPcr"), element: <OptionPcrPanel stockCode={stockCode} /> },
    { key: "industry", label: t("stockAnalysis.industryRanking"), element: <IndustryRankingPanel /> },
    { key: "flash", label: t("stockAnalysis.clsFlash"), element: <ClsFlashPanel /> },
  ];
  // 桌面全部显示，移动端只直接显示 3 个核心面板，其余通过"更多"下拉菜单访问
  // 总面板数 = 3 + 6 = 9
  // 注: 荐股面板已迁出 —— 选股中心用 Tabs 统一了"智能荐股 / 我的筛选",
  // 避免"两处都看到荐股"的歧义。
  const mobileCoreKeys = [
    "index",
    "sectors",
    "north",
  ];
  const sheetPanels = isMobile ? allSheetPanels.filter((p) => mobileCoreKeys.includes(p.key)) : allSheetPanels;

  const activePanel = allSheetPanels.find((p) => p.key === sheetTab);
  const activeContent = tabs.find((t) => t.key === activeTab);

  return (
    <PageErrorBoundary title={t("error.page")}>
      <StockAnalysisPageContext.Provider value={{ openDataSourceSettings }}>
        <div className="sa-layout">
          <div className="sa-header">
            <button type="button" className="sa-header-back" onClick={() => navigate("/")}>
              ? {t("nav.chat")}
            </button>
            <h2 className="sa-header-title">{t("stockAnalysis.title")}</h2>
            <span className="sa-header-meta">{marketStatus || t("stockAnalysis.subtitle")}</span>
            <PageTimeAnchor />
            <button
              type="button"
              className="sa-header-back"
              onClick={() => navigate("/trade")}
              title={t("nav.trade")}
            >
              <ArrowLeftRight size={14} /> {t("nav.trade")}
            </button>
            <button
              type="button"
              className="sa-header-back"
              onClick={() => setSettingsOpen(!settingsOpen)}
              title={t("stockAnalysis.settings.title")}
              style={settingsOpen && !isMobile ? { background: "var(--accent-bg)", color: "var(--accent)" } : undefined}
            >
              {settingsOpen && !isMobile ? <X size={16} /> : <Settings size={16} />}
            </button>
          </div>
          {isReplay && (
            // L2 视觉信号: 页面态细条 — 顶部 1px 紫色细线 + 极淡紫色背景,让用户一眼看到当前是回放模式
            <div
              data-testid="sa-replay-stripe"
              style={{
                height: 2,
                background: "linear-gradient(90deg, #7c3aed 0%, #a855f7 50%, #7c3aed 100%)",
                opacity: 0.85,
              }}
            />
          )}

          <div
            className="sa-body"
            style={isReplay
              ? {
                backgroundImage:
                  "linear-gradient(180deg, rgba(124,58,237,0.04) 0%, rgba(124,58,237,0.01) 30%, transparent 100%)",
              }
              : undefined}
          >
            <StockSearchBar />
            <div style={{ margin: "4px 16px 0 16px", display: "flex", gap: 8, alignItems: "center" }}>
              <AnalysisHistoryButton />
              {/* Debug 面板切换按钮 — 始终可见 */}
              <button
                className="text-xs px-2 py-0.5 rounded cursor-pointer hover:opacity-80 transition-opacity"
                style={{
                  background: showDebug ? "#7c3aed" : "rgba(124,58,237,0.12)",
                  color: showDebug ? "#fff" : "#a78bfa",
                  border: "1px solid rgba(124,58,237,0.3)",
                  fontWeight: 500,
                }}
                onClick={() => setShowDebug(!showDebug)}
                title="打开/关闭分析调试面板"
              >
                🔍 {showDebug ? "隐藏 Debug" : "Debug 面板"}
              </button>
            </div>

            {/* Decision hero (full width, at top) — outside flex row to avoid layout shift */}
            {status === "completed" && (
              <div style={{ margin: "0 16px 12px 16px" }}>
                <DecisionBanner />
                {/* Analyst consensus summary */}
                <AnalystConsensusBar />
                {/* Experiment trail */}
                <ExperimentTrail />
              </div>
            )}

            <div className="sa-body-inner">
              <div className="sa-main">
                {settingsOpen && !isMobile
                  ? (
                    <div className="sa-settings-inline">
                      <div className="sa-settings-header">
                        <span className="sa-settings-title">{t("stockAnalysis.settings.title")}</span>
                        <button type="button" className="sa-header-back" onClick={() => setSettingsOpen(false)}>
                          <X size={14} /> {t("common.close")}
                        </button>
                      </div>
                      <div className="sa-settings-body">
                        <StockAnalysisSettings key={settingsDefaultTab ?? "default"} defaultTab={settingsDefaultTab} />
                      </div>
                    </div>
                  )
                  : (
                    <>
                      {showDebug && <AnalysisDebugPanel />}

                      {status === "loading" && (
                        <div className="sa-loading">
                          <AnalysisProgress />
                          <div className="flex justify-center mt-2">
                            <button
                              className="text-xs px-3 py-1 rounded cursor-pointer transition-colors"
                              style={{
                                background: "rgba(239,68,68,0.15)",
                                color: "#ef4444",
                                border: "1px solid rgba(239,68,68,0.3)",
                              }}
                              onClick={() => {
                                if (window.confirm(t("stockAnalysis.stopAnalysisConfirm"))) {
                                  useStockAnalysisStore.getState().cancelAnalysis();
                                }
                              }}
                            >
                              ⏹ {t("stockAnalysis.stopAnalysis")}
                            </button>
                          </div>
                        </div>
                      )}

                      {status === "idle" && <InvestDashboard />}

                      {status === "error" && (
                        <div
                          style={{
                            padding: 16,
                            margin: 16,
                            border: "1px solid var(--sa-red)",
                            borderRadius: 8,
                            background: "var(--surface)",
                          }}
                        >
                          <h3 style={{ margin: "0 0 8px 0", color: "var(--sa-red)" }}>
                            {failedNodes.length > 0
                              ? t("stockAnalysis.workflow.partialFailed", { count: failedNodes.length })
                              : t("stockAnalysis.workflow.startFailed")}
                          </h3>
                          <p style={{ margin: "0 0 12px 0", color: "var(--muted)", whiteSpace: "pre-wrap" }}>
                            {error ?? t("common.unknownError")}
                          </p>
                          {failedNodes.length > 0 && (
                            <div style={{ marginBottom: 12 }}>
                              <div style={{ fontSize: 12, fontWeight: 600, color: "var(--sa-red)", marginBottom: 4 }}>
                                {t("stockAnalysis.workflow.failedSteps")}
                              </div>
                              <div style={{ display: "flex", flexWrap: "wrap", gap: 4 }}>
                                {failedNodes.map((id) => (
                                  <div key={id}>
                                    <span
                                      onClick={() =>
                                        setExpandedFailedNode(
                                          expandedFailedNode === id ? null : id,
                                        )}
                                      style={{
                                        fontSize: 11,
                                        padding: "2px 6px",
                                        borderRadius: 4,
                                        background: "var(--sa-red-bg)",
                                        color: "var(--sa-red)",
                                        cursor: "pointer",
                                        display: "inline-block",
                                      }}
                                    >
                                      {id} {expandedFailedNode === id ? "▼" : "▶"}
                                    </span>
                                    {expandedFailedNode === id && (
                                      <div
                                        style={{
                                          marginTop: 4,
                                          padding: 8,
                                          background: "var(--surface)",
                                          borderRadius: 6,
                                          border: "1px solid var(--sa-red)",
                                          fontSize: 11,
                                          color: "var(--muted)",
                                          whiteSpace: "pre-wrap",
                                          lineHeight: 1.5,
                                        }}
                                      >
                                        {failedNodeErrors[id] || error || t("common.unknownError")}
                                      </div>
                                    )}
                                  </div>
                                ))}
                              </div>
                            </div>
                          )}
                          <Button
                            type="primary"
                            disabled={!stockCode}
                            onClick={() => stockCode && startAnalysis(stockCode)}
                          >
                            {t("common.retry")}
                          </Button>
                        </div>
                      )}

                      {(status === "running" || status === "completed") && (
                        <>
                          {status === "running" && (
                            <div
                              style={{
                                padding: "12px 16px",
                                margin: "12px 16px 0 16px",
                                border: "1px solid var(--border, #e0e0e0)",
                                borderRadius: 8,
                                background: "var(--surface)",
                              }}
                            >
                              <div className="flex items-start gap-3">
                                <div className="flex-1">
                                  <AnalysisProgress />
                                </div>
                                <button
                                  className="text-xs px-2.5 py-1 rounded cursor-pointer transition-colors shrink-0 mt-1"
                                  style={{
                                    background: "rgba(239,68,68,0.15)",
                                    color: "#ef4444",
                                    border: "1px solid rgba(239,68,68,0.3)",
                                  }}
                                  onClick={() => {
                                    if (window.confirm(t("stockAnalysis.stopAnalysisConfirm"))) {
                                      useStockAnalysisStore.getState().cancelAnalysis();
                                    }
                                  }}
                                >
                                  ⏹ {t("stockAnalysis.stopAnalysis")}
                                </button>
                              </div>
                            </div>
                          )}
                          <div className="sa-tabs">
                            {tabs.map((tab) => (
                              <button
                                key={tab.key}
                                type="button"
                                className={`sa-tab${tab.key === activeTab ? " active" : ""}`}
                                onClick={() => setActiveTab(tab.key)}
                              >
                                {tab.icon}
                                {tab.label}
                              </button>
                            ))}
                          </div>

                          {activeContent?.children}
                        </>
                      )}
                    </>
                  )}
              </div>

              <div className="sa-sidebar">
                {status === "completed" && (
                  <>
                    <ExperimentSidebar />
                    {/* Execute shortcut */}
                    <div
                      onClick={() => navigate("/trade")}
                      style={{
                        marginTop: 8,
                        padding: "8px 0",
                        textAlign: "center",
                        fontSize: 12,
                        border: "0.5px solid var(--color-border-tertiary)",
                        borderRadius: 6,
                        cursor: "pointer",
                        color: "var(--color-text-secondary)",
                      }}
                    >
                      {t("stockAnalysis.experiment.executeTrade")}
                    </div>
                  </>
                )}
                {status !== "completed" && (
                  <Collapse
                    size="small"
                    ghost
                    defaultActiveKey={["screener"]}
                    items={sheetPanels.map((p) => ({
                      key: p.key,
                      label: <span className="text-xs font-medium">{p.label}</span>,
                      children: <div className="sa-panel-body">{p.element}</div>,
                    }))}
                  />
                )}
              </div>
            </div>
          </div>

          {/* Experiment trail (Phase 10) */}
          {/* 底部滑出面板  ? 平板/移动 ? */}
          <div className={`sa-bottom-sheet${sheetOpen ? " open" : ""}`}>
            <div className="sa-sheet-handle" onClick={() => setSheetOpen(!sheetOpen)}>
              <div className="sa-sheet-handle-bar" />
            </div>

            <div className="sa-sheet-tabs">
              {sheetPanels.map((p) => (
                <button
                  key={p.key}
                  type="button"
                  className={`sa-sheet-tab${sheetTab === p.key ? " active" : ""}`}
                  onClick={() => {
                    setSheetTab(p.key);
                    if (!sheetOpen) { setSheetOpen(true); }
                  }}
                >
                  {p.label}
                </button>
              ))}
              {isMobile && (
                <Dropdown
                  menu={{
                    items: allSheetPanels.filter((p) => !mobileCoreKeys.includes(p.key)).map((p) => ({
                      key: p.key,
                      label: p.label,
                      onClick: () => {
                        setSheetTab(p.key);
                        if (!sheetOpen) { setSheetOpen(true); }
                      },
                    })),
                  }}
                  trigger={["click"]}
                >
                  <button type="button" className="sa-sheet-tab">{t("stockAnalysis.settings.sheet.more")} ?</button>
                </Dropdown>
              )}
            </div>

            <div className="sa-sheet-body">
              {activePanel?.element}
            </div>
          </div>

          <button
            type="button"
            className="sa-sheet-toggle"
            onClick={() => setSheetOpen(!sheetOpen)}
          >
            {sheetOpen ? " ?" : "+"}
          </button>
        </div>
        {isMobile && (
          <StockAnalysisSettingsModal
            open={settingsOpen}
            onClose={() => setSettingsOpen(false)}
            defaultTab={settingsDefaultTab}
          />
        )}
      </StockAnalysisPageContext.Provider>
    </PageErrorBoundary>
  );
}

/** 分析师共识条 — 在 Decision Hero 下方显示 10 位分析师的 bull/bear/neutral 投票分布 */
function AnalystConsensusBar() {
  const { t } = useTranslation();
  const analystReports = useStockAnalysisStore((s) => s.analystReports);
  const reports = Object.values(analystReports).filter(Boolean) as string[];
  const total = reports.length;
  if (total === 0) { return null; }

  // 与 AnalystReportGrid 一致：优先用结构化 bull_score/bear_score
  let bull = 0;
  let bear = 0;
  let neutral = 0;
  for (const rawReport of reports) {
    const report = rawReport.replace(/<\/?tool_call[^>]*>/g, "");
    // 尝试 VERDICT JSON
    const vIdx = report.indexOf("<!-- VERDICT:");
    let scores: { bull: number; bear: number } | null = null;
    if (vIdx !== -1) {
      try {
        const js = report.slice(vIdx + "<!-- VERDICT:".length);
        const je = js.indexOf("-->");
        if (je !== -1) {
          const m = JSON.parse(js.slice(0, je).trim());
          const b = m.bull_score ?? m.strength_score ?? null;
          const br = m.bear_score ?? null;
          if (typeof b === "number" || typeof br === "number") {
            scores = { bull: b ?? 0, bear: br ?? 0 };
          }
        }
      } catch { /* ignore */ }
    }
    if (scores) {
      if (scores.bull > scores.bear * 1.2) { bull++; }
      else if (scores.bear > scores.bull * 1.2) { bear++; }
      else { neutral++; }
    } else {
      const s = classifySentiment(report);
      if (s === "bullish") { bull++; }
      else if (s === "bearish") { bear++; }
      else { neutral++; }
    }
  }

  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 12,
        padding: "6px 12px",
        marginTop: 8,
        border: "0.5px solid var(--color-border-tertiary)",
        borderRadius: 6,
        fontSize: 12,
      }}
    >
      <span style={{ fontWeight: 500, color: "var(--color-text-secondary)", whiteSpace: "nowrap" }}>
        {t("stockAnalysis.analystConsensus")}
      </span>
      {bull > 0 && (
        <span style={{ color: "var(--sa-red)" }}>
          {bull} {t("stockAnalysis.bullish")}
        </span>
      )}
      {bear > 0 && (
        <span style={{ color: "var(--sa-green)" }}>
          {bear} {t("stockAnalysis.bearish")}
        </span>
      )}
      {neutral > 0 && (
        <span style={{ color: "var(--color-text-tertiary)" }}>
          {neutral} {t("stockAnalysis.neutral")}
        </span>
      )}
    </div>
  );
}
