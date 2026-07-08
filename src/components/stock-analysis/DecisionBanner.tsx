import { extractLlmField } from "@/lib/agentOutput";
import { invoke } from "@/lib/invoke";
import { exportAnalysisReport } from "@/lib/stock-analysis-export";
import type { ExportData, ExportFormat } from "@/lib/stock-analysis-export";
import { computeStockConsensus } from "@/lib/stock-analysis-utils";
import { getActionColor, getActionTKey, getRiskColor, getRiskTKey } from "@/lib/stock-analysis-utils";
import { useSettingsStore, useStockAnalysisStore } from "@/stores";
import { useTimeAnchorStore } from "@/stores/feature/timeAnchorStore";
import { ExpandOutlined, FilePptOutlined, FileTextOutlined, FileWordOutlined, ReloadOutlined } from "@ant-design/icons";
import { App, Button, Card, Dropdown, Modal, Tag, Tooltip } from "antd";
import NodeRenderer from "markstream-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { cleanToolCallTags } from "./utils";

export function DecisionBanner() {
  const { message } = App.useApp();
  const { t } = useTranslation();
  const navigate = useNavigate();
  const themeMode = useSettingsStore((s) => s.settings.theme_mode);
  const isDark = themeMode === "dark"
    || (themeMode === "system" && window.matchMedia("(prefers-color-scheme: dark)").matches);
  const decision = useStockAnalysisStore((s) => s.decision);
  const stockCode = useStockAnalysisStore((s) => s.stockCode);
  const stockName = useStockAnalysisStore((s) => s.stockName);
  // 重跑分析: 透传当前 analysisId 让后端"覆盖"同 id 旧记录(而非新建一条)
  const analysisId = useStockAnalysisStore((s) => s.analysisId);
  // 加载失败错误(loadAnalysis reject 时由 StockAnalysisPage 写入)
  const error = useStockAnalysisStore((s) => s.error);
  const quote = useStockAnalysisStore((s) => s.quote);
  const analystReports = useStockAnalysisStore((s) => s.analystReports);
  const debateRounds = useStockAnalysisStore((s) => s.debateRounds);
  const riskAssessments = useStockAnalysisStore((s) => s.riskAssessments);
  const valueAssessments = useStockAnalysisStore((s) => s.valueAssessments);
  const dataQualitySummary = useStockAnalysisStore((s) => s.dataQualitySummary);
  const failedNodes = useStockAnalysisStore((s) => s.failedNodes);
  const dataWarnings = useStockAnalysisStore((s) => s.dataWarnings);
  const ruleCheckResults = useStockAnalysisStore((s) => s.ruleCheckResults);
  const rawData = useStockAnalysisStore((s) => s.rawData);
  const bumpWatchlistVersion = useStockAnalysisStore((s) => s.bumpWatchlistVersion);
  const watchlistVersion = useStockAnalysisStore((s) => s.watchlistVersion);
  // P0-1: 证据质量驱动共识
  const stockCodeEvidence = useStockAnalysisStore((s) => stockCode ? s.evidenceReport?.[stockCode] : null);
  // 方案 D 双向并存: LLM 决策 JSON + 一致性分数
  const llmDecisionJson = useStockAnalysisStore((s) => s.llmDecisionJson);
  const decisionAgreementScore = useStockAnalysisStore((s) => s.decisionAgreementScore);
  // 分歧阈值（从工作流模板读取,默认 40）
  const [disagreementThreshold, setDisagreementThreshold] = useState(40);
  useEffect(() => {
    invoke<Record<string, unknown>>("get_workflow_template", { id: "stock-analysis" })
      .then((tmpl) => {
        const vars = (tmpl?.variables ?? []) as Record<string, unknown>[];
        const v = vars.find((x: Record<string, unknown>) => x.name === "dual_view_disagreement_threshold");
        if (v && typeof v.value === "number") {
          setDisagreementThreshold(v.value);
        }
      })
      .catch(() => {});
  }, []);
  // 解析 LLM stance + summary 用于 banner 展示
  const llmStance = useMemo(() => {
    if (!llmDecisionJson) { return null; }
    return (extractLlmField(llmDecisionJson, "action") as string | null)
      ?? (extractLlmField(llmDecisionJson, "stance") as string | null)
      ?? null;
  }, [llmDecisionJson]);
  const llmSummary = useMemo(() => {
    if (!llmDecisionJson) { return null; }
    return (extractLlmField(llmDecisionJson, "reasoning") as string | null)
      ?? (extractLlmField(llmDecisionJson, "summary") as string | null)
      ?? null;
  }, [llmDecisionJson]);
  // timeline-jump 高亮：被 evidence 指向时短暂加 ring 样式
  const highlightedPanel = useStockAnalysisStore((s) => s.highlightedPanel);
  // 时间旅行: 当前决策所基于的 as-of 锚点 (live 时为 null)
  const asOfDate = useTimeAnchorStore((s) => s.asOfDate);
  const [adding, setAdding] = useState(false);
  const [watchlisted, setWatchlisted] = useState(false);
  const [expanded, setExpanded] = useState(false);
  // V50+: 分歧时内联展开双视角对比（替代跳 tab）
  const [showInlineComparison, setShowInlineComparison] = useState(false);
  // 快速交易录入 ref（替代 getElementById，防止多实例 ID 冲突）
  const tradePriceRef = useRef<HTMLInputElement>(null);
  const tradeQtyRef = useRef<HTMLInputElement>(null);

  // stockCode 或自选列表变化时同步自选状态
  useEffect(() => {
    if (!stockCode) { return; }
    let cancelled = false;
    (async () => {
      try {
        const list = await invoke<{ stockCode: string }[]>("list_watchlist");
        if (!cancelled) {
          setWatchlisted(list.some((w) => w.stockCode === stockCode));
        }
      } catch {
        if (!cancelled) { setWatchlisted(false); }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [stockCode, watchlistVersion]);

  const addToWatchlist = useCallback(async () => {
    if (!stockCode || !stockName) { return; }
    setAdding(true);
    try {
      await invoke("add_to_watchlist", { stockCode, stockName });
      setWatchlisted(true);
      bumpWatchlistVersion();
      message.success(t("stockAnalysis.addedToWatchlist"));
    } catch {
      message.error(t("stockAnalysis.addFailed"));
    }
    setAdding(false);
  }, [stockCode, stockName, t, bumpWatchlistVersion, message]);

  const actionLabel = (action: string) => t(getActionTKey(action));

  // 共享的决策上下文计算（避免 handleExport / handleAskAI 重复计算）
  const decisionContext = useMemo(() => {
    if (!decision || !stockCode || !stockName) { return null; }
    const currentPrice = quote?.price ?? 0;
    const targetPriceNum = decision.targetPrice != null ? Number(decision.targetPrice) : 0;
    const upside = targetPriceNum > 0 && currentPrice > 0
      ? ((targetPriceNum - currentPrice) / currentPrice * 100)
      : null;
    const confidencePct = Math.round(decision.confidence ?? 0);
    return { currentPrice, targetPriceNum, upside, confidencePct };
  }, [decision, stockCode, stockName, quote]);

  // Hooks 必须在 early return 之前 — 闭包内部自己处理 null decision
  const [exporting, setExporting] = useState<string | null>(null);

  const handleExport = useCallback(async (format: ExportFormat) => {
    if (!decision || !stockCode || !stockName) { return; }
    setExporting(format);
    try {
      const exportData: ExportData = {
        stockCode,
        stockName,
        asOfDate,
        quote: quote
          ? {
            price: quote.price,
            change: quote.price - quote.preClose,
            changePct: quote.changePct,
            high: quote.high,
            low: quote.low,
            volume: quote.volume,
            amount: quote.amount,
          }
          : null,
        analystReports,
        debateRounds,
        riskAssessments,
        valueAssessments,
        decision,
        llmDecisionJson,
        dataQualitySummary,
        ruleCheckResults,
        rawData,
        failedNodes,
        dataWarnings,
      };
      const result = await exportAnalysisReport(exportData, format, t);
      message.success(result);
    } catch (e: unknown) {
      const errMsg = e instanceof Error ? e.message : String(e);
      message.error(`导出失败: ${errMsg}`);
    } finally {
      setExporting(null);
    }
  }, [
    decision,
    stockCode,
    stockName,
    asOfDate,
    quote,
    analystReports,
    debateRounds,
    riskAssessments,
    valueAssessments,
    llmDecisionJson,
    dataQualitySummary,
    ruleCheckResults,
    rawData,
    failedNodes,
    dataWarnings,
    t,
    message,
  ]);

  const exportMenuItems: { key: ExportFormat; icon: React.ReactNode; label: string }[] = [
    { key: "md", icon: <FileTextOutlined />, label: "Markdown (.md)" },
    { key: "docx", icon: <FileWordOutlined />, label: "Word 文档 (.docx)" },
    { key: "pptx", icon: <FilePptOutlined />, label: "PowerPoint (.pptx)" },
  ];

  const handleAskAI = useCallback(() => {
    const ctx = decisionContext;
    if (!ctx || !decision || !stockCode || !stockName) { return; }
    const { currentPrice, upside, confidencePct } = ctx;
    const context = [
      t("stockAnalysis.askAi.prompt", { stockName, code: stockCode }),
      decision ? t("stockAnalysis.askAi.decision", { action: decision.action, confidence: confidencePct }) : "",
      t("stockAnalysis.export.price", { price: currentPrice.toFixed(2) }),
      upside != null ? t("stockAnalysis.export.upside", { pct: (upside >= 0 ? "+" : "") + upside.toFixed(1) }) : "",
      `${t(getRiskTKey(decision.riskLevel))}`,
    ].filter(Boolean).join("\n");

    navigator.clipboard.writeText(context).then(() => {
      message.success(t("stockAnalysis.contextCopied"));
      navigate(`/chat?code=${stockCode}`);
    }).catch(() => {
      navigate(`/chat?code=${stockCode}`);
    });
  }, [decision, stockCode, stockName, decisionContext, navigate, t, message]);

  // ── 决策缺失占位 ──
  // normalizeDecision 入口已保证非空对象，所以这里的 !decision 意味着：
  //   1) LLM 输出无法解析出 decision（portfolio-mgr 节点结果残缺/被截断）
  //   2) 决策 JSON 是全零空壳（loadAnalysis 已主动跳过 set）
  //   3) loadAnalysis 本身失败（store.error 有值，stockCode 同步预填过但记录详情加载失败）
  // 此时不再 return null，而是渲染"决策缺失"占位卡，让用户知道工作流
  // 已完成但决策信息不完整，并提供"重跑 / 重试"入口。
  if (!decision) {
    // 修复：loadAnalysis 失败时优先渲染"加载失败 + 重试"，避免永远停在
    // "搜索股票"分支(用户认为这违反"history 已有 stockCode"的预期)。
    // StockAnalysisPage 的 useEffect 已从 history 缓存同步预填 stockCode，
    // 所以 error 状态下 stockCode 通常是有值的 → 走"重试"分支。
    if (error) {
      return (
        <Card
          id="decision-banner-top"
          size="small"
          styles={{ body: { padding: "12px 16px" } }}
          style={{
            borderLeft: "4px solid var(--sa-red, #ef4444)",
          }}
          data-testid="decision-banner-load-failed"
        >
          <div className="flex items-start gap-2">
            <span style={{ fontSize: 18, lineHeight: 1 }}>⚠️</span>
            <div className="flex-1">
              <div className="text-sm font-semibold mb-1">
                {t("stockAnalysis.loadAnalysisFailed")}
              </div>
              <div
                className="text-sm mb-2"
                style={{ color: "var(--muted)", wordBreak: "break-all" }}
              >
                {error}
              </div>
              <Button
                size="small"
                type="primary"
                onClick={() => {
                  // 重试：清掉 error 状态后重新调用 loadAnalysis
                  useStockAnalysisStore.setState({ error: null });
                  if (analysisId) {
                    void useStockAnalysisStore.getState().loadAnalysis(analysisId);
                  }
                }}
              >
                {t("stockAnalysis.retry")}
              </Button>
            </div>
          </div>
        </Card>
      );
    }
    return (
      <Card
        id="decision-banner-top"
        size="small"
        styles={{ body: { padding: "12px 16px" } }}
        style={{
          borderLeft: "4px solid var(--sa-amber, #f59e0b)",
          ...(highlightedPanel === "decision"
            ? { boxShadow: "0 0 0 3px var(--accent)", transition: "box-shadow 0.4s" }
            : {}),
        }}
        data-testid="decision-banner-missing"
      >
        <div className="flex items-start gap-2">
          <span style={{ fontSize: 18, lineHeight: 1 }}>⚠️</span>
          <div className="flex-1">
            <div className="text-sm font-semibold mb-1">
              {t("stockAnalysis.decisionMissing")}
            </div>
            <div className="text-sm" style={{ color: "var(--muted)" }}>
              {t("stockAnalysis.decisionMissingHint")}
            </div>
            {stockCode
              ? (
                <div className="mt-2">
                  <Button
                    size="small"
                    type="primary"
                    onClick={() =>
                      useStockAnalysisStore.getState().startAnalysis(
                        stockCode,
                        // 重跑覆盖原 analysisId 对应记录(不传则后端新建 UUID)。
                        // 从决策缺失占位卡点出时 store.analysisId 必然是已点开的历史记录 id。
                        analysisId ? { replaceAnalysisId: analysisId } : undefined,
                      )}
                  >
                    {t("stockAnalysis.reAnalyze")}
                  </Button>
                </div>
              )
              : (
                <div className="mt-2 flex items-center gap-2">
                  <Button
                    size="small"
                    type="primary"
                    onClick={() => {
                      // 占位阶段 stockCode 还没就绪(极少见:用户从未打开过 history 缓存,
                      // 也未在搜索栏选过股票,直接通过分享链接进入):
                      // 聚焦顶部搜索栏让用户输入。
                      const search = document.querySelector<HTMLInputElement>(
                        "[data-testid='stock-analysis-search-input']",
                      );
                      search?.focus();
                      message.info(t("stockAnalysis.reAnalyzeNeedCode"));
                    }}
                  >
                    {t("stockAnalysis.searchStock")}
                  </Button>
                  <span className="text-sm" style={{ color: "var(--muted)" }}>
                    {t("stockAnalysis.reAnalyzeNeedCodeHint")}
                  </span>
                </div>
              )}
          </div>
        </div>
      </Card>
    );
  }

  // TypeScript 在此之后已将 decision 收窄为 StockDecision (non-null)

  // ── 矛盾检测 ──
  // 1. 后端检测：trader 输出自相矛盾（action 与 targetPrice 方向冲突）
  // 2. 前端检测：决策 action 与分析师共识矛盾
  const isBackendContradictory = !!decision?.isContradictory;
  const isConsensusContradictory = (() => {
    if (!analystReports || Object.keys(analystReports).length < 3 || !decision) { return false; }
    const consensus = computeStockConsensus(analystReports, undefined, decision.timeHorizon);
    const isBullishAction = decision.action === "BUY" || decision.action === "INCREASE";
    const isBearishAction = decision.action === "SELL" || decision.action === "REDUCE";
    if (isBullishAction && (consensus.consensus === "bearish" || consensus.consensus === "divided")) { return true; }
    if (isBearishAction && (consensus.consensus === "bullish" || consensus.consensus === "divided")) { return true; }
    return false;
  })();
  const isContradictory = isBackendContradictory || isConsensusContradictory;

  const confidencePct = Math.round(decision.confidence ?? 0);
  const meterColor = confidencePct >= 70
    ? "var(--sa-green)"
    : confidencePct >= 45
    ? "var(--sa-amber)"
    : "var(--sa-red)";

  // 置信度定性标签：让用户快速理解数字含义，而非只看到裸百分比
  const confidenceLabel = confidencePct >= 70
    ? t("stockAnalysis.confidenceHigh")
    : confidencePct >= 45
    ? t("stockAnalysis.confidenceMedium")
    : t("stockAnalysis.confidenceLow");
  const confidenceLabelColor = confidencePct >= 70
    ? "var(--sa-green)"
    : confidencePct >= 45
    ? "var(--sa-amber)"
    : "var(--sa-red)";

  // 从报价和决策计算预期收益
  const currentPrice = quote?.price ?? 0;
  const targetPriceNum = decision.targetPrice != null ? Number(decision.targetPrice) : 0;
  const upside = targetPriceNum > 0 && currentPrice > 0
    ? ((targetPriceNum - currentPrice) / currentPrice * 100)
    : null;

  return (
    <>
      <Card
        id="decision-banner-top"
        size="small"
        title={decisionAgreementScore !== null && decisionAgreementScore < disagreementThreshold && llmStance
          ? (
            /* 分歧时：双决策并列标题 */
            <div className="flex items-center gap-2 flex-wrap">
              <span className="text-sm">{t("stockAnalysis.dualViewDisagreementTitle")}</span>
              <Tag color={getActionColor(decision.action)}>
                {t("stockAnalysis.formula")} {actionLabel(decision.action)}
              </Tag>
              <span className="text-sm" style={{ color: "var(--muted)" }}>vs</span>
              <Tag color={getActionColor(llmStance)}>
                LLM {t(getActionTKey(llmStance))}
              </Tag>
              {asOfDate && (
                <Tag color="purple" title={t("timeTravel.replayBadge.tooltip", { date: asOfDate })}>
                  ⏪ {t("timeTravel.pageAnchor.untilDate", { date: asOfDate })}
                </Tag>
              )}
              {isContradictory && (
                <Tag color="orange">
                  ⚠️ {t("stockAnalysis.contradiction")}
                </Tag>
              )}
            </div>
          )
          : (
            /* 一致或无LLM时：原单决策标题 */
            <div className="flex items-center gap-2">
              <span>{t("stockAnalysis.finalDecision")}</span>
              <Tag color={getActionColor(decision.action)}>
                {actionLabel(decision.action)}
              </Tag>
              {asOfDate && (
                <Tag color="purple" title={t("timeTravel.replayBadge.tooltip", { date: asOfDate })}>
                  ⏪ {t("timeTravel.pageAnchor.untilDate", { date: asOfDate })}
                </Tag>
              )}
              {isContradictory && (
                <Tag color="orange">
                  ⚠️ {t("stockAnalysis.contradiction")}
                </Tag>
              )}
              {decision.timeHorizon && (
                <Tag color="geekblue">
                  {t(`stockAnalysis.timeHorizon${
                    decision.timeHorizon === "ultra_short"
                      ? "UltraShort"
                      : decision.timeHorizon === "short"
                      ? "Short"
                      : decision.timeHorizon === "mid"
                      ? "Mid"
                      : "Long"
                  }`)}
                </Tag>
              )}
            </div>
          )}
        extra={
          <Button
            type="text"
            size="small"
            icon={<ExpandOutlined />}
            onClick={() => setExpanded(true)}
          />
        }
        styles={{ body: { padding: "12px 18px" } }}
        style={{
          borderLeft: "4px solid var(--accent)",
          ...(highlightedPanel === "decision"
            ? { boxShadow: "0 0 0 3px var(--accent)", transition: "box-shadow 0.4s" }
            : {}),
        }}
      >
        {/* 置信度条：数字 + 定性标签 + 共识上下文 */}
        <div className="mb-2">
          <div className="flex justify-between items-center text-sm mb-1">
            <div className="flex items-center gap-1.5">
              <span style={{ color: "var(--muted)" }}>{t("stockAnalysis.confidence")}</span>
              <span
                className="font-mono font-semibold"
                style={{ color: meterColor, fontSize: 18 }}
              >
                {confidencePct}%
              </span>
              {/* V50: 双视角一致性调制后的置信度 */}
              {decision.adjustedConfidence != null
                && Math.abs(decision.adjustedConfidence - confidencePct) > 3 && (
                <span
                  className="font-mono font-semibold text-sm ml-0.5"
                  style={{
                    color: decision.adjustedConfidence > confidencePct
                      ? "#10b981"
                      : "#ef4444",
                    opacity: 0.85,
                  }}
                >
                  →{Math.round(decision.adjustedConfidence)}%
                </span>
              )}
              <span
                className="text-sm px-1.5 py-px rounded font-medium"
                style={{
                  background: `${confidenceLabelColor}18`,
                  color: confidenceLabelColor,
                  border: `1px solid ${confidenceLabelColor}40`,
                }}
              >
                {confidenceLabel}
              </span>
            </div>
            {/* 共识分数：当 dual view 分数可用时展示，帮助用户理解低置信原因 */}
            {decisionAgreementScore !== null && (
              <span
                className="text-sm"
                style={{ color: "var(--muted)" }}
              >
                📊 {t("stockAnalysis.consensusAbbr")} {decisionAgreementScore}/100
              </span>
            )}
          </div>
          <div
            className="relative"
            style={{ height: 6, borderRadius: 3, background: "var(--surface)", overflow: "hidden" }}
          >
            <div
              style={{
                width: `${confidencePct}%`,
                height: "100%",
                borderRadius: 3,
                background: `linear-gradient(to right, ${meterColor}88, ${meterColor})`,
                transition: "width 0.6s ease",
              }}
            />
          </div>
        </div>

        {/* V50+: 分歧时内联双视角对比卡片（紧凑版） */}
        {decisionAgreementScore !== null && decisionAgreementScore < disagreementThreshold && llmStance && (
          <div
            className="mb-2 p-1.5 rounded space-y-1"
            style={{ background: "var(--surface)", border: "1px solid var(--border)" }}
          >
            <div className="flex items-center justify-between">
              <span className="text-sm font-semibold" style={{ color: "#7c3aed" }}>
                📊 {t("stockAnalysis.dualViewComparisonTitle")}
              </span>
              <span
                className="text-sm font-mono px-1 py-px rounded"
                style={{ background: "rgba(239,68,68,0.10)", color: "#ef4444" }}
              >
                {decisionAgreementScore}/100
              </span>
            </div>
            <div className="grid grid-cols-2 gap-1.5 text-sm">
              {/* 公式视角 */}
              <div
                className="rounded px-1.5 py-1 space-y-0.5"
                style={{ background: "rgba(37,99,235,0.05)", borderLeft: "2px solid #2563eb" }}
              >
                <div className="flex items-center justify-between">
                  <span className="font-medium" style={{ color: "#2563eb" }}>{t("stockAnalysis.formula")}</span>
                  <Tag
                    color={getActionColor(decision.action)}
                    style={{ fontSize: 12, lineHeight: "20px", height: 20, paddingInline: 6 }}
                  >
                    {actionLabel(decision.action)}
                  </Tag>
                </div>
                <div className="font-mono flex gap-2" style={{ color: "var(--color-text-secondary)" }}>
                  <span>
                    置信 <b>{confidencePct}%</b>
                  </span>
                  <span>|</span>
                  <span>
                    仓位 <b>{decision.positionPct}%</b>
                  </span>
                </div>
                {decision.reasoning && (
                  <div
                    className="line-clamp-1"
                    style={{ color: "var(--muted)", fontSize: "12px" }}
                    title={cleanToolCallTags(decision.reasoning) ?? ""}
                  >
                    {cleanToolCallTags(decision.reasoning)?.slice(0, 50)}
                    {(cleanToolCallTags(decision.reasoning)?.length ?? 0) > 50 ? "…" : ""}
                  </div>
                )}
              </div>
              {/* LLM 视角 */}
              <div
                className="rounded px-1.5 py-1 space-y-0.5"
                style={{ background: "rgba(124,58,237,0.05)", borderLeft: "2px solid #7c3aed" }}
              >
                <div className="flex items-center justify-between">
                  <span className="font-medium" style={{ color: "#7c3aed" }}>LLM</span>
                  <div className="flex items-center gap-1">
                    <Tag
                      color={getActionColor(llmStance)}
                      style={{ fontSize: 12, lineHeight: "20px", height: 20, paddingInline: 6 }}
                    >
                      {t(getActionTKey(llmStance))}
                    </Tag>
                    {(() => {
                      const llmConf = extractLlmField(llmDecisionJson, "confidence") as number | null;
                      if (llmConf != null && Math.round(llmConf) > confidencePct) {
                        return (
                          <span
                            className="text-sm px-0.5 rounded font-medium"
                            style={{ background: "rgba(16,185,129,0.15)", color: "#10b981" }}
                          >
                            ✓ {t("stockAnalysis.recommended")}
                          </span>
                        );
                      }
                      return null;
                    })()}
                  </div>
                </div>
                <div className="font-mono flex gap-2" style={{ color: "var(--color-text-secondary)" }}>
                  <span>
                    置信{" "}
                    <b>
                      {(() => {
                        const c = extractLlmField(llmDecisionJson, "confidence") as number | null;
                        return c != null ? `${Math.round(c)}%` : "—";
                      })()}
                    </b>
                  </span>
                  <span>|</span>
                  <span>
                    仓位{" "}
                    <b>
                      {(() => {
                        const p = extractLlmField(llmDecisionJson, "positionPct") as number | null;
                        return p != null ? `${Math.round(p)}%` : "—";
                      })()}
                    </b>
                  </span>
                </div>
                {llmSummary && (
                  <div className="line-clamp-1" style={{ color: "var(--muted)", fontSize: "12px" }} title={llmSummary}>
                    {llmSummary.slice(0, 50)}…
                  </div>
                )}
              </div>
            </div>
          </div>
        )}

        {/* Reasoning: collapsed in banner, detailed in Modal via expand button */}
        <div
          className="text-sm mb-2 p-2 rounded cursor-pointer hover:opacity-80 transition-opacity"
          style={{ background: "var(--surface)" }}
          onClick={() => setExpanded(true)}
          role="button"
          tabIndex={0}
          onKeyDown={(e) => {
            if (e.key === "Enter") { setExpanded(true); }
          }}
        >
          {cleanToolCallTags(decision.reasoning || "")
            ? (
              <>
                <span style={{ color: "var(--muted)" }}>{t("stockAnalysis.reasoning")}</span>
                <span className="ml-1" style={{ color: "var(--color-text-secondary)" }}>
                  {cleanToolCallTags(decision.reasoning!)?.slice(0, 120)}
                  {cleanToolCallTags(decision.reasoning!)!.length > 120 ? "…" : ""}
                </span>
                <span className="ml-1 text-sm" style={{ color: "var(--accent)" }}>
                  ▶ {t("stockAnalysis.showDetail")}
                </span>
              </>
            )
            : <span style={{ color: "var(--muted)" }}>{t("stockAnalysis.noDecisionReasoning")}</span>}
        </div>

        {/* [Phase 2 step 10] 分歧时 LLM 对比标注 */}
        {decisionAgreementScore !== null && decisionAgreementScore < disagreementThreshold && llmSummary && (
          <div
            className="text-sm mb-2 p-2 rounded"
            style={{ background: "rgba(124, 58, 237, 0.08)", borderLeft: "3px solid #7c3aed" }}
          >
            <span className="font-medium" style={{ color: "#7c3aed" }}>
              💡 {t("stockAnalysis.llmPerspective")}:
            </span>
            <span className="ml-1" style={{ color: "var(--color-text-secondary)" }}>
              {llmSummary}
            </span>
          </div>
        )}

        {/* 紧凑指标行：grid 分列填满 Card 宽度，避免右侧留空 */}
        <div className="grid gap-1.5 mb-2" style={{ gridTemplateColumns: "repeat(auto-fill, minmax(160px, 1fr))" }}>
          {decision.targetPrice && (
            <span
              className="text-sm px-2 py-1 rounded font-mono flex items-center justify-between"
              style={{ background: "var(--surface)", color: "var(--color-text-primary)" }}
            >
              <span style={{ color: "var(--muted)" }}>{t("stockAnalysis.targetPrice")}</span>
              <span className="font-semibold">¥{decision.targetPrice}</span>
            </span>
          )}
          {decision.stopLoss && (
            <span
              className="text-sm px-2 py-1 rounded font-mono flex items-center justify-between"
              style={{ background: "var(--surface)", color: "var(--sa-red)" }}
            >
              <span style={{ color: "var(--muted)" }}>{t("stockAnalysis.stopLoss")}</span>
              <span className="font-semibold">¥{decision.stopLoss}</span>
            </span>
          )}
          <span
            className="text-sm px-2 py-1 rounded font-mono flex items-center justify-between"
            style={{ background: "var(--surface)", color: "var(--color-text-primary)" }}
          >
            <span style={{ color: "var(--muted)" }}>{t("stockAnalysis.position")}</span>
            <span className="font-semibold">{decision.positionPct}%</span>
          </span>
          {upside != null && (
            <span
              className="text-sm px-2 py-1 rounded font-mono flex items-center justify-between"
              style={{
                background: "var(--surface)",
                color: upside >= 0 ? "var(--sa-green)" : "var(--sa-red)",
              }}
            >
              <span style={{ color: "var(--muted)" }}>{t("stockAnalysis.expectedUpside")}</span>
              <span className="font-semibold">{upside >= 0 ? "+" : ""}{upside.toFixed(1)}%</span>
            </span>
          )}
          <span
            className="text-sm px-2 py-1 rounded flex items-center justify-between"
            style={{ background: "var(--surface)", color: getRiskColor(decision.riskLevel) }}
          >
            <span style={{ color: "var(--muted)" }}>{t("stockAnalysis.riskLevel")}</span>
            <span className="font-semibold">{t(getRiskTKey(decision.riskLevel))}</span>
          </span>
          {/* P0-1: 证据质量门控显示 */}
          {stockCodeEvidence && stockCodeEvidence.holdGate && (
            <span
              className="text-sm px-2 py-1 rounded flex items-center justify-between"
              style={{
                background: "var(--surface)",
                color: stockCodeEvidence.holdGate.holdAllowed ? "var(--sa-blue)" : "var(--sa-amber)",
              }}
            >
              <span style={{ color: "var(--muted)" }}>门控</span>
              <span className="font-semibold text-sm">
                {stockCodeEvidence.holdGate.holdAllowed ? "✅ HOLD" : "🎯 强制方向"}
              </span>
            </span>
          )}
          {stockCodeEvidence && (
            <span
              className="text-sm px-2 py-1 rounded flex items-center justify-between font-mono"
              style={{ background: "var(--surface)", color: "var(--color-text-primary)" }}
            >
              <span style={{ color: "var(--muted)" }}>证据分</span>
              <span className="font-semibold">
                多{stockCodeEvidence.consensus.bullishScore.toFixed(1)}{" "}
                | 空{stockCodeEvidence.consensus.bearishScore.toFixed(1)}{" "}
                | 净{stockCodeEvidence.consensus.netScore.toFixed(1)}
              </span>
            </span>
          )}
          {decision.expectedHoldingDays && (
            <span
              className="text-sm px-2 py-1 rounded font-mono flex items-center justify-between"
              style={{ background: "var(--surface)", color: "var(--color-text-primary)" }}
            >
              <span style={{ color: "var(--muted)" }}>{t("stockAnalysis.expectedHoldingDaysLabel")}</span>
              <span className="font-semibold">
                {t("stockAnalysis.expectedHoldingDays", { days: decision.expectedHoldingDays })}
              </span>
            </span>
          )}
          {decision.targetTimeframe && (
            <span
              className="text-sm px-2 py-1 rounded font-mono flex items-center justify-between"
              style={{ background: "var(--surface)", color: "var(--color-text-primary)" }}
            >
              <span style={{ color: "var(--muted)" }}>{t("stockAnalysis.targetTimeframe")}</span>
              <span className="font-semibold">{decision.targetTimeframe}</span>
            </span>
          )}
        </div>

        {/* 快速交易录入 — 决策日可直接在此录入买卖 */}
        {stockCode && decision.action && decision.action !== "HOLD"
          && (
            <div
              className="flex items-center gap-2 p-1.5 mt-1.5 rounded"
              style={{ background: "var(--surface)" }}
            >
              <span className="text-sm whitespace-nowrap" style={{ color: "var(--muted)" }}>
                {t("stockAnalysis.trade.quickRecord")}
              </span>
              <input
                type="number"
                placeholder={t("trade.price")}
                defaultValue={decision.targetPrice ?? undefined}
                ref={tradePriceRef}
                className="text-sm"
                style={{
                  width: 70,
                  padding: "2px 6px",
                  border: "1px solid var(--color-border-tertiary)",
                  borderRadius: 4,
                  background: "transparent",
                  color: "var(--color-text-primary)",
                }}
              />
              <input
                type="number"
                placeholder={t("trade.quantity")}
                defaultValue={100}
                ref={tradeQtyRef}
                className="text-sm"
                style={{
                  width: 60,
                  padding: "2px 6px",
                  border: "1px solid var(--color-border-tertiary)",
                  borderRadius: 4,
                  background: "transparent",
                  color: "var(--color-text-primary)",
                }}
              />
              <Button
                size="small"
                type="primary"
                style={{ fontSize: 12, lineHeight: "20px", height: 24, padding: "0 8px" }}
                onClick={async () => {
                  const price = parseFloat(tradePriceRef.current?.value ?? "0");
                  const qty = parseInt(tradeQtyRef.current?.value ?? "100", 10);
                  if (price <= 0 || qty <= 0) { return; }
                  const analysisId = useStockAnalysisStore.getState().analysisId;
                  try {
                    await invoke("record_trade", {
                      stockCode,
                      stockName,
                      direction: decision.action === "SELL" ? "sell" : "buy",
                      price,
                      quantity: Math.round(qty / 100) * 100,
                      tradeDate: new Date().toISOString().slice(0, 10),
                      tradeTime: new Date().toISOString().slice(11, 16),
                      notes: `${t("stockAnalysis.trade.fromDecision")} (${
                        t("stockAnalysis.confidence")
                      }: ${confidencePct}%)`,
                      analysisId: analysisId ?? null,
                    });
                    message.success(t("trade.recorded"));
                  } catch (e: unknown) {
                    message.error(String(e));
                  }
                }}
              >
                {t("stockAnalysis.trade.record")}
              </Button>
            </div>
          )}

        <div className="flex gap-2 items-center flex-wrap">
          {stockCode && !watchlisted && (
            <Button size="small" type="dashed" loading={adding} onClick={addToWatchlist}>
              ⭐ {t("stockAnalysis.addToWatchlist")}
            </Button>
          )}
          {watchlisted && <Tag color="gold">⭐ {t("stockAnalysis.inWatchlist")}</Tag>}
          {stockCode && (
            <>
              <Button
                size="small"
                icon={<ReloadOutlined />}
                onClick={() => {
                  useStockAnalysisStore.getState().startAnalysis(
                    stockCode,
                    analysisId ? { replaceAnalysisId: analysisId } : undefined,
                  );
                }}
              >
                {t("stockAnalysis.reAnalyze")}
              </Button>
              <Tooltip title={t("stockAnalysis.rerunDecisionHint")}>
                <Button
                  size="small"
                  icon={<span>⚡</span>}
                  onClick={() => {
                    if (analysisId) {
                      useStockAnalysisStore.getState().rerunDecision(analysisId);
                    }
                  }}
                >
                  {t("stockAnalysis.rerunDecision")}
                </Button>
              </Tooltip>
              <Button size="small" icon={<span>💬</span>} onClick={handleAskAI}>
                {t("stockAnalysis.askAI")}
              </Button>
              <Dropdown
                menu={{
                  items: exportMenuItems.map((item) => ({
                    key: item.key,
                    icon: item.icon,
                    label: item.label,
                    disabled: exporting === item.key,
                    onClick: () => handleExport(item.key),
                  })),
                }}
                trigger={["click"]}
              >
                <Button size="small" loading={exporting !== null} icon={<span>📥</span>}>
                  {t("stockAnalysis.exportReport")}
                </Button>
              </Dropdown>
            </>
          )}
        </div>
      </Card>

      {/* [Phase 2] 决策一致性胶囊: 点击展开双视角对比 */}
      {decisionAgreementScore !== null && (
        <div
          className="flex items-center gap-2 px-3 py-1 rounded cursor-pointer hover:opacity-80 transition-opacity mt-0.5"
          style={{
            background: decisionAgreementScore >= 60
              ? "rgba(16, 185, 129, 0.1)"
              : decisionAgreementScore >= 40
              ? "rgba(245, 158, 11, 0.1)"
              : "rgba(239, 68, 68, 0.1)",
          }}
          onClick={() => {
            // V50+: 分歧时内联展开对比面板，一致时跳转 tab 查看详情
            if (decisionAgreementScore !== null && decisionAgreementScore < disagreementThreshold) {
              setShowInlineComparison(!showInlineComparison);
            } else {
              window.dispatchEvent(new CustomEvent("switch-tab", { detail: "decision-comparison" }));
            }
          }}
        >
          <span className="text-sm" style={{ color: "var(--muted)" }}>
            📊 {t("stockAnalysis.dualViewConsistency")}
          </span>
          <span
            className="font-mono text-[12px] font-semibold"
            style={{
              color: decisionAgreementScore >= 60
                ? "#10b981"
                : decisionAgreementScore >= 40
                ? "#f59e0b"
                : "#ef4444",
            }}
          >
            {decisionAgreementScore}/100
          </span>
          {llmStance && (
            <>
              <span className="text-sm" style={{ color: "var(--muted)" }}>·</span>
              <span
                className="px-1.5 rounded text-sm font-medium"
                style={{ background: "var(--sa-purple-bg, #ede9fe)", color: "#7c3aed" }}
              >
                LLM: {llmStance}
              </span>
            </>
          )}
          {decisionAgreementScore < disagreementThreshold && (
            <span className="text-sm" style={{ color: "#ef4444" }}>
              ⚠️ {t("stockAnalysis.dualViewDisagreement")}
            </span>
          )}
        </div>
      )}

      {/* V50+: 内联展开完整双视角对比面板（紧凑版） */}
      {showInlineComparison && decisionAgreementScore !== null && (
        <div
          className="mt-1.5 p-1.5 rounded space-y-1"
          style={{ background: "var(--surface)", border: "1px solid var(--border)" }}
        >
          <div className="flex items-center justify-between">
            <span className="text-sm font-semibold" style={{ color: "#7c3aed" }}>
              📊 {t("stockAnalysis.fullComparisonTitle")}
            </span>
            <button
              className="text-sm hover:opacity-70"
              style={{ color: "var(--muted)", border: "none", background: "none", padding: 0, cursor: "pointer" }}
              onClick={() => setShowInlineComparison(false)}
            >
              ✕
            </button>
          </div>
          <div className="grid grid-cols-2 gap-1.5 text-sm">
            {/* 公式列 */}
            <div
              className="rounded px-1.5 py-1 space-y-0.5"
              style={{ background: "rgba(37,99,235,0.05)", borderLeft: "2px solid #2563eb" }}
            >
              <div className="flex items-center justify-between">
                <span className="font-medium" style={{ color: "#2563eb" }}>{t("dualView.decision.formula")}</span>
                {decision?.action
                  ? <Tag color={getActionColor(decision.action)}>{t(getActionTKey(decision.action ?? ""))}</Tag>
                  : <span style={{ color: "var(--muted)" }}>—</span>}
              </div>
              <div className="font-mono flex gap-2" style={{ color: "var(--color-text-secondary)" }}>
                <span>
                  置信 <b>{confidencePct}%</b>
                </span>
                <span>|</span>
                <span>
                  仓位 <b>{decision ? `${decision.positionPct}%` : "—"}</b>
                </span>
              </div>
              {decision?.reasoning && (
                <div className="line-clamp-2" style={{ color: "var(--muted)", fontSize: "12px" }}>
                  {decision.reasoning.slice(0, 120)}
                  {decision.reasoning.length > 120 ? "…" : ""}
                </div>
              )}
            </div>
            {/* LLM 列 */}
            <div
              className="rounded px-1.5 py-1 space-y-0.5"
              style={{ background: "rgba(124,58,237,0.05)", borderLeft: "2px solid #7c3aed" }}
            >
              <div className="flex items-center justify-between">
                <span className="font-medium" style={{ color: "#7c3aed" }}>LLM</span>
                {llmStance
                  ? <Tag color={getActionColor(llmStance)}>{t(getActionTKey(llmStance))}</Tag>
                  : <span style={{ color: "var(--muted)" }}>—</span>}
              </div>
              <div className="font-mono flex gap-2" style={{ color: "var(--color-text-secondary)" }}>
                <span>
                  置信{" "}
                  <b>
                    {(() => {
                      const c = extractLlmField(llmDecisionJson, "confidence") as number | null;
                      return c != null ? `${Math.round(c)}%` : "—";
                    })()}
                  </b>
                </span>
                <span>|</span>
                <span>
                  仓位{" "}
                  <b>
                    {(() => {
                      const p = extractLlmField(llmDecisionJson, "positionPct") as number | null;
                      return p != null ? `${Math.round(p)}%` : "—";
                    })()}
                  </b>
                </span>
              </div>
              {llmSummary && (
                <div className="line-clamp-2" style={{ color: "var(--muted)", fontSize: "12px" }}>
                  {llmSummary.slice(0, 120)}
                  {llmSummary.length > 120 ? "…" : ""}
                </div>
              )}
            </div>
          </div>
          {/* 分歧诊断（单行内联） */}
          {decision?.agreementBreakdown && decisionAgreementScore < 80 && (
            <div
              className="flex gap-2 text-sm pt-0.5"
              style={{ borderTop: "1px solid var(--border)", color: "var(--muted)" }}
            >
              <span>
                {decision.agreementBreakdown.actionNote === "opposite" ? "✗方向相反" : "⚠分歧"}
                ({decision.agreementBreakdown.formulaAction} vs {decision.agreementBreakdown.llmAction})
              </span>
              {decision.agreementBreakdown.positionGap != null && (
                <span>仓位差{Math.round(decision.agreementBreakdown.positionGap)}%</span>
              )}
              {decision.agreementBreakdown.confidenceGap != null && (
                <span>置信差{Math.round(decision.agreementBreakdown.confidenceGap)}%</span>
              )}
            </div>
          )}
        </div>
      )}
      <Modal
        title={
          <div className="flex items-center gap-2">
            <span>{t("stockAnalysis.finalDecision")}</span>
            <Tag color={getActionColor(decision.action)}>
              {actionLabel(decision.action)}
            </Tag>
            {asOfDate && (
              <Tag color="purple" title={t("timeTravel.replayBadge.tooltip", { date: asOfDate })}>
                ⏪ {t("timeTravel.pageAnchor.untilDate", { date: asOfDate })}
              </Tag>
            )}
          </div>
        }
        open={expanded}
        onCancel={() => setExpanded(false)}
        footer={null}
        width="80vw"
        style={{ top: 20 }}
        styles={{ body: { maxHeight: "80vh", overflow: "auto" } }}
      >
        <div className="mb-4">
          <div className="flex justify-between items-center mb-1">
            <div className="flex items-center gap-2">
              <span style={{ color: "var(--muted)" }}>{t("stockAnalysis.confidence")}</span>
              <span
                className="font-mono font-semibold"
                style={{ color: meterColor, fontSize: 22 }}
              >
                {confidencePct}%
              </span>
              {/* V50: 双视角一致性调制后的置信度 */}
              {decision.adjustedConfidence != null
                && Math.abs(decision.adjustedConfidence - confidencePct) > 3 && (
                <span
                  className="font-mono font-semibold text-sm ml-0.5"
                  style={{
                    color: decision.adjustedConfidence > confidencePct
                      ? "#10b981"
                      : "#ef4444",
                    opacity: 0.85,
                  }}
                >
                  →{Math.round(decision.adjustedConfidence)}%
                </span>
              )}
              <span
                className="text-sm px-2 py-0.5 rounded font-medium"
                style={{
                  background: `${confidenceLabelColor}18`,
                  color: confidenceLabelColor,
                  border: `1px solid ${confidenceLabelColor}40`,
                }}
              >
                {confidenceLabel}
              </span>
            </div>
            {decisionAgreementScore !== null && (
              <span
                className="text-sm"
                style={{ color: "var(--muted)" }}
              >
                📊 {t("stockAnalysis.consensusAbbr")} {decisionAgreementScore}/100
              </span>
            )}
          </div>
          <div
            className="relative"
            style={{ height: 14, borderRadius: 7, background: "var(--surface)", overflow: "hidden" }}
          >
            <div
              style={{
                width: `${confidencePct}%`,
                height: "100%",
                borderRadius: 7,
                background: `linear-gradient(to right, ${meterColor}88, ${meterColor})`,
                transition: "width 0.6s ease",
              }}
            />
          </div>
        </div>

        <div
          className="grid gap-3 mb-4"
          style={{ gridTemplateColumns: "repeat(5, 1fr)" }}
        >
          {decision.targetPrice && (
            <div className="text-center p-3 rounded" style={{ background: "var(--surface)" }}>
              <div className="text-sm" style={{ color: "var(--muted)" }}>{t("stockAnalysis.targetPrice")}</div>
              <div className="text-lg font-semibold font-mono">¥{decision.targetPrice}</div>
            </div>
          )}
          {decision.stopLoss && (
            <div className="text-center p-3 rounded" style={{ background: "var(--surface)" }}>
              <div className="text-sm" style={{ color: "var(--muted)" }}>{t("stockAnalysis.stopLoss")}</div>
              <div className="text-lg font-semibold font-mono" style={{ color: "var(--sa-red)" }}>
                ¥{decision.stopLoss}
              </div>
            </div>
          )}
          <div className="text-center p-3 rounded" style={{ background: "var(--surface)" }}>
            <div className="text-sm" style={{ color: "var(--muted)" }}>{t("stockAnalysis.position")}</div>
            <div className="text-lg font-semibold font-mono">{decision.positionPct}%</div>
          </div>
          {upside != null && (
            <div className="text-center p-3 rounded" style={{ background: "var(--surface)" }}>
              <div className="text-sm" style={{ color: "var(--muted)" }}>{t("stockAnalysis.expectedUpside")}</div>
              <div
                className="text-lg font-semibold font-mono"
                style={{ color: upside >= 0 ? "var(--sa-green)" : "var(--sa-red)" }}
              >
                {upside >= 0 ? "+" : ""}
                {upside.toFixed(1)}%
              </div>
            </div>
          )}
          <div className="text-center p-3 rounded" style={{ background: "var(--surface)" }}>
            <div className="text-sm" style={{ color: "var(--muted)" }}>{t("stockAnalysis.riskLevel")}</div>
            <div
              className="text-lg font-semibold"
              style={{
                color: getRiskColor(decision.riskLevel),
              }}
            >
              {t(getRiskTKey(decision.riskLevel))}
            </div>
          </div>
        </div>

        <div
          className="sa-markdown-content text-sm mb-4 p-4 rounded"
          style={{ background: "var(--surface)" }}
        >
          {cleanToolCallTags(decision.reasoning || "")
            ? <NodeRenderer content={cleanToolCallTags(decision.reasoning || "")} isDark={isDark} />
            : <span style={{ color: "var(--muted)" }}>{t("stockAnalysis.noDecisionReasoning")}</span>}
        </div>

        {/* [Phase 2 step 10] Modal 内分歧时 LLM 对比标注 */}
        {decisionAgreementScore !== null && decisionAgreementScore < disagreementThreshold && llmSummary && (
          <div
            className="text-sm mb-4 p-3 rounded"
            style={{ background: "rgba(124, 58, 237, 0.08)", borderLeft: "3px solid #7c3aed" }}
          >
            <span className="font-medium" style={{ color: "#7c3aed" }}>
              💡 {t("stockAnalysis.llmPerspective")}:
            </span>
            <span className="ml-1" style={{ color: "var(--color-text-secondary)" }}>
              {llmSummary}
            </span>
          </div>
        )}

        <div className="flex gap-2 items-center flex-wrap">
          {stockCode && !watchlisted && (
            <Button type="dashed" loading={adding} onClick={addToWatchlist}>
              ⭐ {t("stockAnalysis.addToWatchlist")}
            </Button>
          )}
          {watchlisted && <Tag color="gold">⭐ {t("stockAnalysis.inWatchlist")}</Tag>}
          {stockCode && (
            <>
              <Button
                icon={<ReloadOutlined />}
                onClick={() => {
                  useStockAnalysisStore.getState().startAnalysis(
                    stockCode,
                    analysisId ? { replaceAnalysisId: analysisId } : undefined,
                  );
                }}
              >
                {t("stockAnalysis.reAnalyze")}
              </Button>
              <Button icon={<span>💬</span>} onClick={handleAskAI}>
                {t("stockAnalysis.askAI")}
              </Button>
              <Dropdown
                menu={{
                  items: exportMenuItems.map((item) => ({
                    key: item.key,
                    icon: item.icon,
                    label: item.label,
                    disabled: exporting === item.key,
                    onClick: () => handleExport(item.key),
                  })),
                }}
                trigger={["click"]}
              >
                <Button loading={exporting !== null} icon={<span>📥</span>}>
                  {t("stockAnalysis.exportReport")}
                </Button>
              </Dropdown>
            </>
          )}
        </div>
      </Modal>
    </>
  );
}
