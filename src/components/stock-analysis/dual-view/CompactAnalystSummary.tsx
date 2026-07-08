/**
 * CompactAnalystSummary — AnalystReportGrid 在 chat 中的紧凑版本
 * 输入:分析师报告 { expertId: report }
 * 输出:共识标签 + 多空条 + Top 3 报告摘要
 */
import { classifySentiment } from "@/lib/stock-analysis-utils";
import { Tooltip } from "antd";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { cleanToolCallTags } from "../utils";

interface CompactAnalystSummaryProps {
  data: Record<string, string> | unknown;
}

type Consensus = "bullish" | "bearish" | "neutral" | "divided";

/** 根据多空比例推共识（与 AnalystReportGrid 保持一致） */
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

function normalizeMap(data: CompactAnalystSummaryProps["data"]): Record<string, string> {
  if (data && typeof data === "object" && !Array.isArray(data)) {
    const out: Record<string, string> = {};
    for (const [k, v] of Object.entries(data as Record<string, unknown>)) {
      if (typeof v === "string") { out[k] = v; }
    }
    return out;
  }
  return {};
}

function getAgentShortName(id: string): string {
  // 形如 "a-value-investor" → "Value"
  // 形如 "a-momentum-trader" → "Momentum"
  const m = id.replace(/^a-/, "").replace(/-/g, " ");
  const parts = m.split(" ");
  // 取前 1-2 词首字母
  if (parts.length === 1) { return parts[0].slice(0, 8); }
  return (parts[0][0] + (parts[1]?.[0] ?? "")).toUpperCase();
}

export function CompactAnalystSummary({ data }: CompactAnalystSummaryProps) {
  const { t } = useTranslation();
  const reports = useMemo(() => normalizeMap(data), [data]);

  const summary = useMemo(() => {
    const entries = Object.entries(reports);
    if (entries.length === 0) { return null; }
    let bullish = 0;
    let bearish = 0;
    let neutral = 0;
    const details: Array<{ id: string; name: string; sentiment: "bullish" | "bearish" | "neutral"; snippet: string }> =
      [];
    for (const [id, rawReport] of entries) {
      // 先剥掉 tool_call 标签，再让统一 classifySentiment 解析（支持 JSON + 计分回退）
      const cleaned = cleanToolCallTags(rawReport);
      const sentiment = classifySentiment(cleaned);
      if (sentiment === "bullish") { bullish++; }
      else if (sentiment === "bearish") { bearish++; }
      else { neutral++; }
      details.push({
        id,
        name: getAgentShortName(id),
        sentiment,
        snippet: cleaned.slice(0, 80).replace(/\s+/g, " ").trim(),
      });
    }
    const total = bullish + bearish + neutral;
    return { bullish, bearish, neutral, total, details };
  }, [reports]);

  if (!summary) {
    return (
      <div className="text-[12px] italic" style={{ color: "var(--muted)" }}>
        暂无分析师报告
      </div>
    );
  }

  const consensus = deriveConsensus(summary.bullish, summary.bearish, summary.neutral);
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

  return (
    <div className="space-y-1 text-[12px]">
      {/* 共识标签 — 与 AnalystReportGrid 风格一致 */}
      <div className="flex items-center gap-1.5 flex-wrap">
        <span
          className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[11px] font-semibold"
          style={{ background: cc.bg, color: cc.color }}
        >
          <span>{cc.icon}</span>
          <span>{t(`stockAnalysis.recommendation.${cc.labelKey}`)}</span>
        </span>
        <span style={{ color: "var(--muted)" }}>
          {summary.total}
          {t("stockAnalysis.recommendation.reportCountSuffix")}
        </span>
      </div>

      {/* 多空条 */}
      <div className="flex items-baseline gap-2 flex-wrap">
        <Tooltip title={t("stockAnalysis.recommendation.bullishTooltip")}>
          <span style={{ color: "var(--sa-red, #dc2626)" }} className="cursor-default">
            📈 {t("stockAnalysis.recommendation.bullish")} {summary.bullish}
          </span>
        </Tooltip>
        <Tooltip title={t("stockAnalysis.recommendation.bearishTooltip")}>
          <span style={{ color: "var(--sa-green, #16a34a)" }} className="cursor-default">
            📉 {t("stockAnalysis.recommendation.bearish")} {summary.bearish}
          </span>
        </Tooltip>
        <Tooltip title={t("stockAnalysis.recommendation.neutralTooltip")}>
          <span style={{ color: "var(--muted)" }} className="cursor-default">
            ➖ {t("stockAnalysis.recommendation.neutral")} {summary.neutral}
          </span>
        </Tooltip>
      </div>
      {summary.total > 0 && (
        <div className="flex h-1.5 rounded overflow-hidden" style={{ background: "var(--muted-bg, #e5e7eb)" }}>
          {summary.bullish > 0 && (
            <div
              style={{
                width: `${(summary.bullish / summary.total) * 100}%`,
                background: "var(--sa-red, #dc2626)",
                transition: "width 0.3s ease",
              }}
            />
          )}
          {summary.neutral > 0 && (
            <div
              style={{
                width: `${(summary.neutral / summary.total) * 100}%`,
                background: "var(--muted, #6b7280)",
                transition: "width 0.3s ease",
              }}
            />
          )}
          {summary.bearish > 0 && (
            <div
              style={{
                width: `${(summary.bearish / summary.total) * 100}%`,
                background: "var(--sa-green, #16a34a)",
                transition: "width 0.3s ease",
              }}
            />
          )}
        </div>
      )}
      <div className="space-y-0.5 mt-1">
        {summary.details.slice(0, 3).map((d) => (
          <div key={d.id} className="flex items-start gap-1.5 text-[11px]">
            <span
              className="text-[9px] font-mono px-1 rounded shrink-0"
              style={{
                background: d.sentiment === "bullish"
                  ? "var(--sa-red-bg, #fee2e2)"
                  : d.sentiment === "bearish"
                  ? "var(--sa-green-bg, #dcfce7)"
                  : "var(--muted-bg, #e5e7eb)",
                color: d.sentiment === "bullish"
                  ? "var(--sa-red, #dc2626)"
                  : d.sentiment === "bearish"
                  ? "var(--sa-green, #16a34a)"
                  : "var(--muted, #6b7280)",
              }}
            >
              {d.name}
            </span>
            <span className="flex-1 truncate leading-snug" style={{ color: "var(--color-text-secondary)" }}>
              {d.snippet}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}
