/**
 * DecisionComparisonPanel — 双视角决策对比的紧凑面板版
 *
 * 紧凑设计原则：
 *   - 标题 + 一致性分数同行（色条+数字，无圆环）
 *   - 公式/LLM 双列并排卡片，每列内: action tag + conf% | pos% 单行
 *   - reasoning 截断 80 字符单行，点击展开完整
 *   - 分歧诊断用 Tooltip 内联（不另占卡片空间）
 *   - 总高度控制在 ~140px 以内
 */
import { getActionTKey } from "@/lib/stock-analysis-utils";
import { Empty, Tag, Tooltip } from "antd";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { CompactDecisionShape } from "./CompactDecisionComparison";

interface DecisionComparisonPanelProps {
  data: CompactDecisionShape | unknown;
}

function normalize(data: DecisionComparisonPanelProps["data"]): CompactDecisionShape {
  if (data && typeof data === "object") {
    return data as CompactDecisionShape;
  }
  return {};
}

function normalizeAction(a?: string | null): string {
  return (a ?? "").trim().toLowerCase().replace(/[\s/_\u3000]+/g, "");
}

function actionTagColor(action?: string | null): string {
  const norm = normalizeAction(action);
  if (norm.includes("买") || norm.includes("增持")) { return "green"; }
  if (norm.includes("卖") || norm.includes("减持")) { return "red"; }
  return "default";
}

function agreementLevel(score: number): "high" | "mid" | "low" {
  if (score >= 80) { return "high"; }
  if (score >= 40) { return "mid"; }
  return "low";
}

/** 数字格式化 */
function fmtNum(v: number | null | undefined, suffix = ""): string {
  if (typeof v !== "number" || !Number.isFinite(v)) { return "—"; }
  return `${v.toFixed(0)}${suffix}`;
}

/** 截断文本 */
function truncate(text: string, maxLen: number): string {
  if (text.length <= maxLen) { return text; }
  return text.slice(0, maxLen) + "…";
}

export function DecisionComparisonPanel({ data }: DecisionComparisonPanelProps) {
  const { t } = useTranslation();
  const [expandedReasoning, setExpandedReasoning] = useState(false);
  const view = useMemo(() => normalize(data), [data]);

  const hasLlm = view.llmDecisionAction != null
    && view.llmDecisionAction !== ""
    && view.llmDecisionAction !== "null";
  const agreement = typeof view.decisionAgreementScore === "number" ? view.decisionAgreementScore : null;

  // LLM 不可用 → 紧凑回退
  if (!hasLlm && agreement === null) {
    const hasFormula = view.decisionAction != null && view.decisionAction !== "" && view.decisionAction !== "null";
    if (!hasFormula) {
      return (
        <div className="decision-comparison-panel p-2">
          <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description={t("dualView.decision.llmUnavailableHint")} />
        </div>
      );
    }
    return (
      <div className="decision-comparison-panel p-2 space-y-1 text-sm">
        <Tag color="warning">{t("stockAnalysis.llmViewUnavailable")}</Tag>
        <div className="grid grid-cols-2 gap-x-3 gap-y-0.5">
          <span style={{ color: "var(--muted)" }}>{t("dualView.decision.action")}</span>
          <span>
            <Tag color={actionTagColor(view.decisionAction)}>{t(getActionTKey(view.decisionAction ?? ""))}</Tag>
          </span>
          <span style={{ color: "var(--muted)" }}>{t("dualView.decision.confidence")}</span>
          <span className="font-mono">{fmtNum(view.confidence)}</span>
        </div>
      </div>
    );
  }

  const level = agreement !== null ? agreementLevel(agreement) : null;
  const agreeColor = level === "high" ? "#10b981" : level === "mid" ? "#f59e0b" : "#ef4444";
  const agreeBg = level === "high"
    ? "rgba(16,185,129,0.10)"
    : level === "mid"
    ? "rgba(245,158,11,0.10)"
    : "rgba(239,68,68,0.10)";

  const REASON_MAX = expandedReasoning ? 500 : 80;
  const formulaReasoning = view.decisionReasoning || "";
  const llmReasoning = view.llmDecisionReasoning || "";

  return (
    <div className="decision-comparison-panel p-2 space-y-1.5 text-sm">
      {/* ═══ 第1行：标题 + 一致性色条 + 分歧诊断(Tooltip) + 展开按钮 ═══ */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-1.5">
          <span className="text-sm font-semibold">{t("dualView.decision.title")}</span>
          {agreement !== null && (
            <span
              className="flex items-center gap-0.5 px-1.5 py-0.5 rounded-full text-sm font-mono font-medium"
              style={{ background: agreeBg, color: agreeColor }}
            >
              {agreement}
              <span className="opacity-60">/100</span>
            </span>
          )}
          {/* 分歧诊断 — 内联 Tooltip，悬停查看详情 */}
          {view.agreementBreakdown && agreement !== null && agreement < 80 && (
            <Tooltip
              title={
                <div className="text-sm space-y-0.5">
                  <div>
                    {t("dualView.decision.action")}: {view.agreementBreakdown.actionNote === "opposite"
                      ? "✗ 相反"
                      : view.agreementBreakdown.actionNote === "same_direction"
                      ? "≈ 同向"
                      : "⚠ 分歧"} ({view.agreementBreakdown.formulaAction} vs {view.agreementBreakdown.llmAction})
                  </div>
                  {view.agreementBreakdown.positionGap != null && (
                    <div>
                      {t("dualView.decision.positionPct")}: 差 {Math.round(view.agreementBreakdown.positionGap)}%
                    </div>
                  )}
                  {view.agreementBreakdown.confidenceGap != null && (
                    <div>
                      {t("dualView.decision.confidence")}: 差 {Math.round(view.agreementBreakdown.confidenceGap)}%
                    </div>
                  )}
                </div>
              }
            >
              <span
                className="cursor-help px-1 py-0.5 rounded text-sm underline decoration-dotted"
                style={{ color: "#ef4444", opacity: 0.85 }}
              >
                ⚠{view.agreementBreakdown.actionNote === "opposite"
                  ? ` ${t("dualView.decision.reviewRecommended")}`
                  : ""}
              </span>
            </Tooltip>
          )}
        </div>
        {/* reasoning 超长时显示展开按钮 */}
        {!expandedReasoning && (formulaReasoning.length > 80 || llmReasoning.length > 80) && (
          <button
            className="text-sm hover:opacity-70 transition-opacity"
            style={{
              color: "var(--accent, #6366f1)",
              border: "none",
              background: "none",
              padding: 0,
              cursor: "pointer",
            }}
            onClick={() => setExpandedReasoning(true)}
          >
            {t("stockAnalysis.showDetail")} ▸
          </button>
        )}
      </div>

      {/* ═══ 第2行：双列对比卡片（公式 | LLM）═══ */}
      <div className="grid grid-cols-2 gap-2">
        {/* ── 公式列 ── */}
        <div
          className="rounded-md p-1.5 space-y-1"
          style={{ background: "rgba(37,99,235,0.05)", border: "1px solid rgba(37,99,235,0.10)" }}
        >
          <div className="flex items-center justify-between">
            <span
              className="text-sm font-medium px-1 py-px rounded"
              style={{ background: "rgba(37,99,235,0.10)", color: "#2563eb" }}
            >
              {t("dualView.decision.formula")}
            </span>
            {view.decisionAction
              ? <Tag color={actionTagColor(view.decisionAction)}>{t(getActionTKey(view.decisionAction ?? ""))}</Tag>
              : <span style={{ color: "var(--muted)" }}>—</span>}
          </div>
          <div className="flex items-center gap-1.5 font-mono text-sm">
            <span style={{ color: "var(--color-text-secondary)" }}>
              置信 <b style={{ fontSize: "13px" }}>{fmtNum(view.confidence)}%</b>
            </span>
            <span style={{ color: "var(--muted)", opacity: 0.4 }}>|</span>
            <span style={{ color: "var(--color-text-secondary)" }}>
              仓位 <b style={{ fontSize: "13px" }}>{fmtNum(view.decisionPositionPct, "%")}</b>
            </span>
          </div>
          {formulaReasoning && (
            <div
              className="leading-tight cursor-pointer hover:opacity-70 transition-opacity line-clamp-1"
              style={{ color: "var(--color-text-secondary)", fontSize: "12px" }}
              onClick={() => setExpandedReasoning(!expandedReasoning)}
              title={expandedReasoning ? "" : formulaReasoning}
            >
              {truncate(formulaReasoning, REASON_MAX)}
            </div>
          )}
        </div>

        {/* ── LLM 列 ── */}
        <div
          className="rounded-md p-1.5 space-y-1"
          style={{ background: "rgba(124,58,237,0.05)", border: "1px solid rgba(124,58,237,0.10)" }}
        >
          <div className="flex items-center justify-between">
            <span
              className="text-sm font-medium px-1 py-px rounded"
              style={{ background: "rgba(124,58,237,0.10)", color: "#7c3aed" }}
            >
              {t("dualView.decision.llm")}
            </span>
            {hasLlm
              ? (
                <Tag color={actionTagColor(view.llmDecisionAction)}>
                  {t(getActionTKey(view.llmDecisionAction ?? ""))}
                </Tag>
              )
              : <span style={{ color: "var(--muted)" }}>—</span>}
          </div>
          <div className="flex items-center gap-1.5 font-mono text-sm">
            <span style={{ color: "var(--color-text-secondary)" }}>
              置信 <b style={{ fontSize: "13px" }}>{fmtNum(view.llmConfidence)}%</b>
            </span>
            <span style={{ color: "var(--muted)", opacity: 0.4 }}>|</span>
            <span style={{ color: "var(--color-text-secondary)" }}>
              仓位 <b style={{ fontSize: "13px" }}>{fmtNum(view.llmDecisionPositionPct, "%")}</b>
            </span>
          </div>
          {llmReasoning && (
            <div
              className="leading-tight cursor-pointer hover:opacity-70 transition-opacity line-clamp-1"
              style={{ color: "var(--color-text-secondary)", fontSize: "12px" }}
              onClick={() => setExpandedReasoning(!expandedReasoning)}
              title={expandedReasoning ? "" : llmReasoning}
            >
              {truncate(llmReasoning, REASON_MAX)}
            </div>
          )}
        </div>
      </div>

      {/* LLM 缺失提示 */}
      {!hasLlm && agreement !== null && (
        <div
          className="text-sm italic px-1.5 py-0.5 rounded"
          style={{ background: "var(--surface)", color: "var(--muted)" }}
        >
          {t("dualView.decision.llmMissingHint", { score: agreement })}
        </div>
      )}

      {/* 收起按钮 */}
      {expandedReasoning && (
        <button
          className="text-sm hover:opacity-70 transition-opacity"
          style={{ color: "var(--muted)", border: "none", background: "none", padding: 0, cursor: "pointer" }}
          onClick={() => setExpandedReasoning(false)}
        >
          ▾ {t("collapseComparison") ?? "收起"}
        </button>
      )}
    </div>
  );
}
