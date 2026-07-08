/**
 * CompactDecisionComparison — 双视角决策对比的 chat bubble 紧凑版
 *
 * 方案 D 双向并存:展示"LLM 视角 vs 公式视角"对比的紧凑 2-3 行视图。
 * 用于 chat bubble 嵌入,默认 `<DecisionComparisonPanel />` 完整版用 `<DualViewRenderer>` 切换。
 *
 * 数据契约:与后端 `stock_analyses` 表的字段对应,前端 `AnalysisSummary` 接口已扩展。
 *
 * 显示规则:
 * - LLM 决策可用时:两行(公式 / LLM 各一行),底部一致性分数
 * - LLM 不可用时:一行占位 + 灰条"LLM 视角不可用"
 */
import { getActionTKey } from "@/lib/stock-analysis-utils";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";

/** 与 AnalysisSummary / LatestAnalysisSummary 的 LLM 字段子集对齐 */
export interface CompactDecisionShape {
  decisionAction?: string | null;
  decisionPositionPct?: number | null;
  confidence?: number | null;
  /** V50: 双视角一致性调制后的置信度 */
  adjustedConfidence?: number | null;
  /** 公式决策推理文本（用于展示公式 reasoning） */
  decisionReasoning?: string | null;
  llmDecisionAction?: string | null;
  llmDecisionPositionPct?: number | null;
  llmConfidence?: number | null;
  /** LLM 决策推理文本(用于 panel 完整版展示) */
  llmDecisionReasoning?: string | null;
  decisionAgreementScore?: number | null;
  /** V50: 双视角一致性分维度诊断 */
  agreementBreakdown?: {
    total: number;
    actionOk: boolean;
    actionNote: string;
    formulaAction: string;
    llmAction: string;
    positionGap: number | null;
    confidenceGap: number | null;
    conflictType: string;
  } | null;
}

interface CompactDecisionComparisonProps {
  data: CompactDecisionShape | unknown;
}

function normalize(data: CompactDecisionComparisonProps["data"]): CompactDecisionShape {
  if (data && typeof data === "object") {
    return data as CompactDecisionShape;
  }
  return {};
}

/** 归一化 action 字符串(与后端 compute_decision_agreement::normalize_action 保持一致) */
function normalizeAction(a?: string | null): string {
  return (a ?? "").trim().toLowerCase().replace(/[\s/_\u3000]+/g, "");
}

/** 把 action 字符串映射成"行动风格"色标(与 DecisionBanner 一致:绿买/红卖/黄观) */
function actionColor(action?: string | null): string {
  const norm = normalizeAction(action);
  // 买/增持 → 蓝绿;持有/观望 → 灰;卖/减持 → 红
  if (norm.includes("买") || norm.includes("增持")) { return "#10b981"; }
  if (norm.includes("卖") || norm.includes("减持")) { return "#ef4444"; }
  return "#94a3b8";
}

export function CompactDecisionComparison({ data }: CompactDecisionComparisonProps) {
  const { t } = useTranslation();
  const view = useMemo(() => normalize(data), [data]);

  const hasLlm = view.llmDecisionAction != null
    && view.llmDecisionAction !== ""
    && view.llmDecisionAction !== "null";
  const agreement = typeof view.decisionAgreementScore === "number" ? view.decisionAgreementScore : null;
  const actionsMatch = hasLlm
    && normalizeAction(view.decisionAction) === normalizeAction(view.llmDecisionAction);

  return (
    <div className="space-y-1.5 text-sm">
      {/* 一致性色条 + 分数 */}
      <div className="flex items-center gap-2">
        <span style={{ color: "var(--muted)" }}>{t("dualView.decision.title")}</span>
        {agreement !== null && (
          <div
            className="flex items-center gap-1 px-1.5 rounded text-sm font-mono"
            style={{
              background: agreement >= 60
                ? "rgba(16, 185, 129, 0.12)"
                : agreement >= 40
                ? "rgba(245, 158, 11, 0.12)"
                : "rgba(239, 68, 68, 0.12)",
              color: agreement >= 60
                ? "#10b981"
                : agreement >= 40
                ? "#f59e0b"
                : "#ef4444",
            }}
          >
            <span className="font-semibold">{agreement}</span>
            <span style={{ opacity: 0.7 }}>/100</span>
          </div>
        )}
      </div>

      {/* 公式行 */}
      <div className="flex items-center gap-1.5">
        <span
          className="px-1 rounded text-sm font-medium"
          style={{ background: "var(--sa-blue-bg, #dbeafe)", color: "#2563eb" }}
        >
          {t("dualView.decision.formulaBadge")}
        </span>
        <span
          className="font-mono text-sm font-semibold"
          style={{ color: actionColor(view.decisionAction) }}
        >
          {view.decisionAction ? t(getActionTKey(view.decisionAction)) : "—"}
        </span>
        {typeof view.decisionPositionPct === "number" && (
          <span className="text-sm font-mono" style={{ color: "var(--muted)" }}>
            {view.decisionPositionPct.toFixed(0)}%
          </span>
        )}
      </div>

      {/* LLM 行(不可用时显示灰条) */}
      {hasLlm
        ? (
          <div className="flex items-center gap-1.5">
            <span
              className="px-1 rounded text-sm font-medium"
              style={{ background: "var(--sa-purple-bg, #ede9fe)", color: "#7c3aed" }}
            >
              {t("dualView.decision.llmBadge")}
            </span>
            <span
              className="font-mono text-sm font-semibold"
              style={{ color: actionColor(view.llmDecisionAction) }}
            >
              {t(getActionTKey(view.llmDecisionAction ?? ""))}
            </span>
            {typeof view.llmDecisionPositionPct === "number" && (
              <span className="text-sm font-mono" style={{ color: "var(--muted)" }}>
                {view.llmDecisionPositionPct.toFixed(0)}%
              </span>
            )}
            {actionsMatch && (
              <span
                className="text-sm px-1 rounded"
                style={{ background: "rgba(16, 185, 129, 0.15)", color: "#10b981" }}
              >
                ✓
              </span>
            )}
          </div>
        )
        : (
          <div
            className="flex items-center gap-1.5 italic"
            style={{ color: "var(--muted)" }}
          >
            <span
              className="px-1 rounded text-sm font-medium"
              style={{ background: "var(--sa-purple-bg, #ede9fe)", color: "#7c3aed" }}
            >
              {t("dualView.decision.llmBadge")}
            </span>
            <span className="text-sm">{t("dualView.decision.llmUnavailable")}</span>
          </div>
        )}
      {/* V50: 分歧诊断摘要（仅低一致时显示） */}
      {agreement !== null && agreement < 60 && view.agreementBreakdown && (
        <div className="text-sm" style={{ color: "#ef4444", opacity: 0.85 }}>
          {view.agreementBreakdown.actionNote === "opposite"
            ? `⚠ ${view.agreementBreakdown.formulaAction} ≠ ${view.agreementBreakdown.llmAction}（方向相反）`
            : `⚠ 分歧（${
              view.agreementBreakdown.conflictType === "opposite_direction"
                ? "方向冲突"
                : view.agreementBreakdown.conflictType
            }）`}
        </div>
      )}
    </div>
  );
}
