// i18n-exempt: 业务逻辑判断字符串，非 UI 展示文本
import { classifySentiment } from "@/lib/stock-analysis-utils";
import { useStockAnalysisStore } from "@/stores";
import { Button, Card, Table, Tag, Tooltip } from "antd";
import type { ColumnsType } from "antd/es/table";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { AnalystReportCard } from "./AnalystReportCard";
import { cleanToolCallTags } from "./utils";

type Consensus = "bullish" | "bearish" | "neutral" | "divided";

// ── 10 个分析师 AgentNode ID（与 stockWorkflowChatBridge 的 ANALYST_NODE_TO_NAME 一致） ──
// 顺序按工作流惯常执行顺序：技术面 → 情绪面 → 消息面 → 基本面 → 政策面 → 资金面 → 解禁 → 研报 → 板块 → 催化剂
const ANALYST_NODE_IDS = [
  "a-market-analyst",
  "a-sentiment",
  "a-news",
  "a-fundamentals",
  "a-policy",
  "a-hot-money",
  "a-lockup",
  "a-research",
  "a-sector",
  "a-catalyst",
] as const;

type AnalystEntry =
  | { nodeId: string; expertId: string; status: "done"; report: string }
  | { nodeId: string; expertId: string; status: "pending" }
  | { nodeId: string; expertId: string; status: "failed"; error?: string };

/**
 * 分析师占位卡片：工作流运行中或节点失败时显示，让用户看到"分析师 tab 在同步工作流状态"
 *  - pending: ⏳ 等待中
 *  - failed:  ❌ 失败 + 错误信息
 */
function AnalystPlaceholderCard({
  expertId,
  status,
  error,
}: {
  expertId: string;
  status: "pending" | "failed";
  error?: string;
}) {
  const { t } = useTranslation();
  const name = t(`stockAnalysis.workflow.analyst.${expertId}`, expertId);

  const config = {
    pending: {
      icon: "⏳",
      color: "var(--muted, #6b7280)",
      bg: "var(--muted-bg, #e5e7eb)",
      label: t("stockAnalysis.workflow.pending"),
      tagColor: "default" as const,
    },
    failed: {
      icon: "❌",
      color: "var(--sa-red, #dc2626)",
      bg: "var(--sa-red-bg, #fee2e2)",
      label: t("common.failed"),
      tagColor: "error" as const,
    },
  }[status];

  return (
    <Card
      size="small"
      className="h-full"
      styles={{ body: { padding: 12 } }}
    >
      <div className="flex items-center gap-2 mb-2">
        <span className="text-base leading-none">{config.icon}</span>
        <span className="font-medium text-sm" style={{ color: "var(--color-text-base)" }}>{name}</span>
      </div>
      <Tag color={config.tagColor}>{config.label}</Tag>
      {status === "failed" && error && (
        <div
          className="mt-2 text-xs"
          style={{
            color: config.color,
            whiteSpace: "pre-wrap",
            wordBreak: "break-word",
            maxHeight: 120,
            overflow: "auto",
          }}
        >
          {error}
        </div>
      )}
    </Card>
  );
}

/**
 * 从报告中提取结构化多空分数（与 AnalystReportCard 同源）
 * 优先解析 <!-- VERDICT: {bull_score, bear_score} --> 格式
 * 回退到 classifySentiment 关键词匹配
 */
function extractBullBearScores(report: string): { bull: number; bear: number } | null {
  // 1) 尝试 VERDICT JSON 格式（与 AnalystReportCard.tryParseVerdictFormat 一致）
  const verdictIdx = report.indexOf("<!-- VERDICT:");
  if (verdictIdx !== -1) {
    try {
      const jsonStr = report.slice(verdictIdx + "<!-- VERDICT:".length);
      const jsonEnd = jsonStr.indexOf("-->");
      if (jsonEnd !== -1) {
        const meta = JSON.parse(jsonStr.slice(0, jsonEnd).trim());
        const bull = meta.bull_score ?? meta.strength_score ?? null;
        const bear = meta.bear_score ?? null;
        if (typeof bull === "number" || typeof bear === "number") {
          return {
            bull: typeof bull === "number" ? (bull > 1 ? bull : bull * 100) : 0,
            bear: typeof bear === "number" ? (bear > 1 ? bear : bear * 100) : 0,
          };
        }
        // 有 stance/verdict 字段但没有分数 → 用 stance 判断
        const stance = String(meta.verdict ?? meta.stance ?? "").toLowerCase();
        if (/看多|买入|增持|做多|看涨|bull/i.test(stance)) { return { bull: 60, bear: 0 }; }
        if (/看空|卖出|减持|做空|看跌|bear/i.test(stance)) { return { bull: 0, bear: 60 }; }
      }
    } catch { /* ignore */ }
  }

  // 2) 尝试直接从文本正则匹配 bull_score/bear_score
  try {
    const bullMatch = report.match(/"bull_score"\s*:\s*(\d+(?:\.\d+)?)/);
    const bearMatch = report.match(/"bear_score"\s*:\s*(\d+(?:\.\d+)?)/);
    if (bullMatch || bearMatch) {
      return {
        bull: bullMatch ? parseFloat(bullMatch[1]) : 0,
        bear: bearMatch ? parseFloat(bearMatch[1]) : 0,
      };
    }
  } catch { /* ignore */ }

  // 3) 无结构化数据 → 返回 null，调用方用 classifySentiment fallback
  return null;
}

/** 根据多空比例推共识：
 *  - 一方 > 65% → 共识看多 / 共识看空
 *  - 双方都有且 35-65% → 分歧
 *  - 全部中性 / 单边极小 → 中性
 */
function deriveConsensus(
  bullish: number,
  bearish: number,
  neutral: number,
): Consensus {
  const total = bullish + bearish + neutral;
  if (total === 0) { return "neutral"; }
  const bullRatio = bullish / total;
  const bearRatio = bearish / total;
  if (bullRatio > 0.65) { return "bullish"; }
  if (bearRatio > 0.65) { return "bearish"; }
  if (bullRatio > 0 && bearRatio > 0) { return "divided"; }
  return "neutral";
}

export function AnalystReportGrid() {
  const { t } = useTranslation();
  const analystReports = useStockAnalysisStore((s) => s.analystReports);
  const failedNodes = useStockAnalysisStore((s) => s.failedNodes);
  const failedNodeErrors = useStockAnalysisStore((s) => s.failedNodeErrors);
  const workflowStatus = useStockAnalysisStore((s) => s.status);

  // Aggregate sentiment from reports — 优先用结构化 bull_score/bear_score（与单个分析师卡片一致）
  const sentiment = useMemo(() => {
    const entries = Object.values(analystReports);
    let bullish = 0;
    let bearish = 0;
    let neutral = 0;
    for (const rawReport of entries) {
      const report = cleanToolCallTags(rawReport);
      // 先尝试提取结构化分数（与 AnalystReportCard 同源）
      const scores = extractBullBearScores(report);
      if (scores) {
        // 有结构化分数：用多空对比判断方向
        if (scores.bull > scores.bear * 1.2) { bullish++; }
        else if (scores.bear > scores.bull * 1.2) { bearish++; }
        else { neutral++; } // 接近 → 中性/分歧
      } else {
        // 无结构化数据：回退到关键词匹配
        const s = classifySentiment(report);
        if (s === "bullish") { bullish++; }
        else if (s === "bearish") { bearish++; }
        else { neutral++; }
      }
    }
    const total = bullish + bearish + neutral;
    return { bullish, bearish, neutral, total };
  }, [analystReports]);

  const consensus = useMemo(
    () => deriveConsensus(sentiment.bullish, sentiment.bearish, sentiment.neutral),
    [sentiment],
  );

  // ── 统一构造显示列表：done / pending / failed ──
  // done:    analystReports 已有内容（且清理后非空）→ 渲染完整 AnalystReportCard
  // failed:  failedNodes 包含该 nodeId → 渲染失败占位卡片（带错误信息）
  // pending: 工作流运行中且节点未完成未失败 → 渲染等待占位卡片
  // 工作流完成后既无 report 也未 failed 的节点 → 显示失败（无数据兜底，避免一直"等待中"）
  const entries = useMemo<AnalystEntry[]>(() => {
    const result: AnalystEntry[] = [];
    const seen = new Set<string>();
    const isRunning = workflowStatus === "running" || workflowStatus === "loading";

    for (const nodeId of ANALYST_NODE_IDS) {
      const expertId = nodeId.slice(2);
      seen.add(expertId);
      const reportRaw = analystReports[expertId];
      // 双重检查：即使 reportRaw 存在，也要确保清理工具标签后确实有实际内容
      const hasContent = typeof reportRaw === "string"
        && reportRaw.length > 0
        && cleanToolCallTags(reportRaw).trim().length > 0;

      if (hasContent) {
        result.push({ nodeId, expertId, status: "done", report: reportRaw });
      } else if (failedNodes.includes(nodeId)) {
        result.push({ nodeId, expertId, status: "failed", error: failedNodeErrors[nodeId] });
      } else if (isRunning) {
        result.push({ nodeId, expertId, status: "pending" });
      } else {
        // Bug #P0 修复: 工作流已完成但节点无有效数据也未标记失败，
        // 显示失败状态（无数据），避免卡片消失或永远卡在"等待中"
        result.push({ nodeId, expertId, status: "failed", error: t("stockAnalysis.analystReport.nodeNoData") });
      }
    }

    // 追加 analystReports 中存在但不在预定义列表里的 key（如 trader 的 "investment-plan"）
    for (const [expertId, report] of Object.entries(analystReports)) {
      if (seen.has(expertId)) { continue; }
      const hasContent = typeof report === "string"
        && report.length > 0
        && cleanToolCallTags(report).trim().length > 0;
      if (hasContent) {
        result.push({ nodeId: expertId, expertId, status: "done", report });
      }
    }

    return result;
  }, [analystReports, failedNodes, failedNodeErrors, workflowStatus, t]);

  // 空态：工作流未启动 / 无任何分析师数据 → 不渲染（保持原行为）
  if (entries.length === 0) { return null; }

  const consensusConfig: Record<Consensus, { color: string; bg: string; labelKey: string; icon: string }> = {
    bullish: {
      color: "var(--sa-red, #dc2626)",
      bg: "var(--sa-red-bg, #fee2e2)",
      labelKey: "consensusBullish",
      icon: "📈",
    },
    bearish: {
      color: "var(--sa-green, #16a34a)",
      bg: "var(--sa-green-bg, #dcfce7)",
      labelKey: "consensusBearish",
      icon: "📉",
    },
    neutral: {
      color: "var(--muted, #6b7280)",
      bg: "var(--muted-bg, #e5e7eb)",
      labelKey: "consensusNeutral",
      icon: "➖",
    },
    divided: {
      color: "var(--ant-warning, #f59e0b)",
      bg: "var(--ant-warning-bg, #fef3c7)",
      labelKey: "consensusDivided",
      icon: "⚖️",
    },
  };
  const cc = consensusConfig[consensus];
  const total = sentiment.total;
  const bullPct = total > 0 ? Math.round((sentiment.bullish / total) * 100) : 0;
  const bearPct = total > 0 ? Math.round((sentiment.bearish / total) * 100) : 0;
  const neutPct = total > 0 ? Math.round((sentiment.neutral / total) * 100) : 0;

  // ── 解析 10 位分析师的 VERDICT 数据（供辩手的数据诊断） ──
  const [showBrief, setShowBrief] = useState(false);

  interface AnalystVerdictRow {
    key: string;
    name: string;
    available: boolean;
    verdict: string | null;
    directionStatus: "good" | "warning" | "issue";
    bullScore: number | null;
    bearScore: number | null;
    consensusScore: number | null;
    confidence: number | null;
    hasBullPoints: boolean;
    hasBearPoints: boolean;
  }

  const analystVerdicts = useMemo<AnalystVerdictRow[]>(() => {
    return ANALYST_NODE_IDS.map((nodeId) => {
      const expertId = nodeId.slice(2);
      const name = t(`stockAnalysis.workflow.analyst.${expertId}`, expertId);
      const report = analystReports[expertId];
      if (!report) {
        return {
          key: expertId,
          name,
          available: false,
          verdict: null,
          directionStatus: "issue",
          bullScore: null,
          bearScore: null,
          consensusScore: null,
          confidence: null,
          hasBullPoints: false,
          hasBearPoints: false,
        };
      }
      const cleaned = cleanToolCallTags(report);
      // 解析 VERDICT tag
      let verdict: Record<string, unknown> | null = null;
      const vIdx = cleaned.indexOf("<!-- VERDICT:");
      if (vIdx !== -1) {
        try {
          const jsonStr = cleaned.slice(vIdx + "<!-- VERDICT:".length);
          const jsonEnd = jsonStr.indexOf("-->");
          if (jsonEnd !== -1) { verdict = JSON.parse(jsonStr.slice(0, jsonEnd).trim()); }
        } catch { /* ignore */ }
      }
      // 兜底：纯 JSON
      if (!verdict) {
        try {
          verdict = JSON.parse(cleaned);
        } catch { /* ignore */ }
      }
      const v = verdict ?? {};
      // V61 fix: strict_mode 下 verdict 可能是嵌套对象（如 {"verdict":"看多","bull_score":7,...}），
      // 必须提取为字符串，否则 Table 渲染 <span>{v}</span> 抛出 "Objects not valid as React child"
      const rawVerdict = v.verdict ?? v.stance ?? null;
      let verdictStr = rawVerdict !== null && typeof rawVerdict === "object"
        ? (typeof (rawVerdict as Record<string, unknown>).verdict === "string"
          ? (rawVerdict as Record<string, unknown>).verdict as string
          : null)
        : rawVerdict as string | null;
      let bull = typeof v.bull_score === "number" ? (v.bull_score > 1 ? v.bull_score : v.bull_score * 10) : null;
      let bear = typeof v.bear_score === "number" ? (v.bear_score > 1 ? v.bear_score : v.bear_score * 10) : null;
      const conf = typeof v.confidence === "number" ? (v.confidence > 1 ? v.confidence : v.confidence * 10) : null;

      // V62 fix: 不论 VERDICT/JSON 解析成功与否，都用 extractBullBearScores 补充。
      // 解决 LLM 输出 JSON 能解析但缺 bull_score/bear_score 字段时，全部列空的问题。
      const fallbackScores = extractBullBearScores(cleaned);
      if (fallbackScores) {
        // JSON 里有 bull_score/bear_score 时优先（精度更高），否则用 regex
        if (bull === null) { bull = fallbackScores.bull; }
        if (bear === null) { bear = fallbackScores.bear; }
        // 仍未提取到 verdict 文字时，用分数推断方向
        if (!verdictStr) {
          if (fallbackScores.bull > fallbackScores.bear * 1.2) {
            verdictStr = t("stockAnalysis.recommendation.bullish");
          } else if (fallbackScores.bear > fallbackScores.bull * 1.2) {
            verdictStr = t("stockAnalysis.recommendation.bearish");
          }
        }
      }
      // 仍未提取到 verdict 文字时，用 classifySentiment 兜底
      if (!verdictStr && cleaned.trim().length > 0) {
        const s = classifySentiment(cleaned);
        if (s === "bullish") { verdictStr = t("stockAnalysis.recommendation.bullish"); }
        else if (s === "bearish") { verdictStr = t("stockAnalysis.recommendation.bearish"); }
        if (!verdictStr) { verdictStr = t("stockAnalysis.recommendation.neutral"); }
      }
      let ds: "good" | "warning" | "issue" = "issue";
      if (verdict || bull !== null || bear !== null) {
        if (verdictStr && bull !== null && bear !== null) { ds = "good"; }
        else if (verdictStr || bull !== null || bear !== null) { ds = "warning"; }
      }
      const fallbackAvailable = verdict !== null || bull !== null || bear !== null || !!verdictStr;
      return {
        key: expertId,
        name,
        available: fallbackAvailable,
        verdict: verdictStr,
        directionStatus: ds,
        bullScore: bull,
        bearScore: bear,
        consensusScore: bull !== null && bear !== null ? Math.round(bull - bear) : null,
        confidence: conf,
        hasBullPoints: Array.isArray(v.bull_points) && (v.bull_points as unknown[]).length > 0,
        hasBearPoints: Array.isArray(v.bear_points) && (v.bear_points as unknown[]).length > 0,
      };
    });
  }, [analystReports, t]);

  const briefColumns: ColumnsType<AnalystVerdictRow> = [
    { title: t("stockAnalysis.tab.analysts"), dataIndex: "name", key: "name", width: 110, fixed: "left" },
    {
      title: "",
      dataIndex: "available",
      key: "available",
      width: 40,
      align: "center",
      render: (avail: boolean) =>
        avail
          ? (
            <Tooltip title={t("stockAnalysis.analystReport.dataAvailable")}>
              <span style={{ color: "#52c41a" }}>✅</span>
            </Tooltip>
          )
          : (
            <Tooltip title={t("stockAnalysis.viz.noData")}>
              <span>❌</span>
            </Tooltip>
          ),
    },
    {
      title: t("stockAnalysis.simVsBacktest.judgment"),
      dataIndex: "verdict",
      key: "verdict",
      width: 60,
      render: (v: unknown) => {
        if (v === null || v === undefined || typeof v !== "string") {
          return <span style={{ color: "var(--muted)", fontSize: 11 }}>-</span>;
        }
        const isBull = /看多|bull|偏多|买入|增持|正面/i.test(v);
        const isBear = /看空|bear|偏空|卖出|减持|负面/i.test(v);
        const color = isBull ? "#f5222d" : isBear ? "#52c41a" : "var(--muted)";
        return <span style={{ color, fontWeight: 600, fontSize: 12 }}>{v}</span>;
      },
    },
    {
      title: "📈",
      dataIndex: "bullScore",
      key: "bullScore",
      width: 44,
      align: "right",
      render: (v: number | null) =>
        v !== null
          ? <span style={{ color: "#f5222d", fontWeight: 600, fontSize: 12 }}>{Math.round(v)}</span>
          : <span style={{ color: "var(--muted)", fontSize: 11 }}>-</span>,
    },
    {
      title: "📉",
      dataIndex: "bearScore",
      key: "bearScore",
      width: 44,
      align: "right",
      render: (v: number | null) =>
        v !== null
          ? <span style={{ color: "#52c41a", fontWeight: 600, fontSize: 12 }}>{Math.round(v)}</span>
          : <span style={{ color: "var(--muted)", fontSize: 11 }}>-</span>,
    },
    {
      title: t("stockAnalysis.consensusAbbr"),
      dataIndex: "consensusScore",
      key: "consensusScore",
      width: 48,
      align: "right",
      render: (v: number | null) => {
        if (v === null) { return <span style={{ color: "var(--muted)", fontSize: 11 }}>-</span>; }
        const color = v > 0 ? "#f5222d" : v < 0 ? "#52c41a" : "var(--muted)";
        return <span style={{ color, fontWeight: 700, fontSize: 12 }}>{v > 0 ? `+${v}` : v}</span>;
      },
    },
    {
      title: t("stockAnalysis.experiment.conf"),
      dataIndex: "confidence",
      key: "confidence",
      width: 44,
      align: "right",
      render: (v: number | null) =>
        v !== null
          ? <span style={{ fontWeight: 600, fontSize: 12 }}>{Math.round(v)}</span>
          : <span style={{ color: "var(--muted)", fontSize: 11 }}>-</span>,
    },
    {
      title: t("stockAnalysis.analystReport.bullArgument"),
      key: "hasBullPoints",
      width: 40,
      align: "center",
      render: (_: unknown, row: AnalystVerdictRow) =>
        row.hasBullPoints
          ? (
            <Tooltip title={t("stockAnalysis.analystReport.hasBullArgument")}>
              <span style={{ color: "#f5222d" }}>✅</span>
            </Tooltip>
          )
          : <span style={{ color: "var(--muted)" }}>➖</span>,
    },
    {
      title: t("stockAnalysis.analystReport.bearArgument"),
      key: "hasBearPoints",
      width: 40,
      align: "center",
      render: (_: unknown, row: AnalystVerdictRow) =>
        row.hasBearPoints
          ? (
            <Tooltip title={t("stockAnalysis.analystReport.hasBearArgument")}>
              <span style={{ color: "#52c41a" }}>✅</span>
            </Tooltip>
          )
          : <span style={{ color: "var(--muted)" }}>➖</span>,
    },
  ];

  return (
    <div>
      {/* 舆情摘要 bar — 醒目的共识卡片 + 大号数字 + 百分比 + 色彩条 */}
      {sentiment.total > 0 && (
        <div
          className="mb-3 p-3 rounded-md"
          style={{ background: "var(--surface)", border: "1px solid var(--border, #e5e7eb)" }}
        >
          {/* 共识标签 + 总数 */}
          <div className="flex items-center justify-between mb-2">
            <div className="flex items-center gap-2">
              <span
                className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-sm font-semibold"
                style={{ background: cc.bg, color: cc.color }}
              >
                <span>{cc.icon}</span>
                <span>{t(`stockAnalysis.recommendation.${cc.labelKey}`)}</span>
              </span>
              <span className="text-xs" style={{ color: "var(--muted)" }}>
                {t("stockAnalysis.tab.analysts")} · {total}
                {t("stockAnalysis.recommendation.reportCountSuffix")}
              </span>
            </div>
          </div>

          {/* 大号三色数字 */}
          <div className="grid grid-cols-3 gap-2 mb-2">
            <Tooltip title={t("stockAnalysis.recommendation.bullishTooltip")}>
              <div
                className="rounded p-2 text-center cursor-default"
                style={{ background: "var(--sa-red-bg, #fee2e2)" }}
              >
                <div className="text-2xl font-bold leading-none" style={{ color: "var(--sa-red, #dc2626)" }}>
                  {sentiment.bullish}
                </div>
                <div className="text-[10px] mt-1" style={{ color: "var(--sa-red, #dc2626)" }}>
                  📈 {t("stockAnalysis.recommendation.bullish")} {bullPct}%
                </div>
              </div>
            </Tooltip>
            <Tooltip title={t("stockAnalysis.recommendation.bearishTooltip")}>
              <div
                className="rounded p-2 text-center cursor-default"
                style={{ background: "var(--sa-green-bg, #dcfce7)" }}
              >
                <div
                  className="text-2xl font-bold leading-none"
                  style={{ color: "var(--sa-green, #16a34a)" }}
                >
                  {sentiment.bearish}
                </div>
                <div className="text-[10px] mt-1" style={{ color: "var(--sa-green, #16a34a)" }}>
                  📉 {t("stockAnalysis.recommendation.bearish")} {bearPct}%
                </div>
              </div>
            </Tooltip>
            <Tooltip title={t("stockAnalysis.recommendation.neutralTooltip")}>
              <div
                className="rounded p-2 text-center cursor-default"
                style={{ background: "var(--muted-bg, #e5e7eb)" }}
              >
                <div className="text-2xl font-bold leading-none" style={{ color: "var(--muted, #6b7280)" }}>
                  {sentiment.neutral}
                </div>
                <div className="text-[10px] mt-1" style={{ color: "var(--muted, #6b7280)" }}>
                  ➖ {t("stockAnalysis.recommendation.neutral")} {neutPct}%
                </div>
              </div>
            </Tooltip>
          </div>

          {/* 色彩条 */}
          <div className="flex h-2.5 rounded-full overflow-hidden" style={{ background: "var(--muted-bg, #e5e7eb)" }}>
            {sentiment.bullish > 0 && (
              <div
                style={{
                  width: `${bullPct}%`,
                  background: "var(--sa-red, #dc2626)",
                  transition: "width 0.3s ease",
                }}
              />
            )}
            {sentiment.neutral > 0 && (
              <div
                style={{
                  width: `${neutPct}%`,
                  background: "var(--muted, #6b7280)",
                  transition: "width 0.3s ease",
                }}
              />
            )}
            {sentiment.bearish > 0 && (
              <div
                style={{
                  width: `${bearPct}%`,
                  background: "var(--sa-green, #16a34a)",
                  transition: "width 0.3s ease",
                }}
              />
            )}
          </div>
        </div>
      )}

      {/* ── 分析师数据诊断明细（供辩手） ── */}
      {sentiment.total > 0 && (
        <div className="mb-3">
          <Button
            type="default"
            size="small"
            onClick={() => setShowBrief((v) => !v)}
            style={{ fontSize: 12 }}
          >
            {showBrief ? "🔼" : "🔽"} {t("stockAnalysis.analystReport.dataDetail")}
            <span style={{ color: "var(--muted)", marginLeft: 6, fontSize: 11 }}>
              ({analystVerdicts.filter((r) => r.available).length}/{analystVerdicts.length}{" "}
              {t("stockAnalysis.evidenceCitation.supported")})
            </span>
          </Button>
          {showBrief && (
            <div
              style={{
                marginTop: 8,
                border: "1px solid var(--border, #e5e7eb)",
                borderRadius: 6,
                overflow: "auto",
              }}
            >
              <Table
                dataSource={analystVerdicts}
                columns={briefColumns}
                rowKey="key"
                pagination={false}
                size="small"
                bordered
                style={{ fontSize: 12 }}
                scroll={{ x: 540 }}
                onHeaderRow={() => ({ style: { fontSize: 11 } })}
              />
            </div>
          )}
        </div>
      )}

      <div
        className="grid gap-2 analyst-cards-grid"
        style={{ gridTemplateColumns: "repeat(auto-fill, minmax(220px, 1fr))" }}
      >
        {entries.map((entry) => {
          if (entry.status === "done") {
            return (
              <AnalystReportCard
                key={entry.expertId}
                expertId={entry.expertId}
                report={entry.report}
              />
            );
          }
          const isFailed = entry.status === "failed";
          return (
            <AnalystPlaceholderCard
              key={entry.expertId}
              expertId={entry.expertId}
              status={entry.status}
              error={isFailed ? entry.error : undefined}
            />
          );
        })}
      </div>
    </div>
  );
}
