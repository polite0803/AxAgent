// SPDX-License-Identifier: AGPL-3.0-only
/* eslint-disable react-refresh/only-export-components */

import type { AgreementBreakdown } from "@/types";
import { Tag } from "antd";
import { useTranslation } from "react-i18next";

/** 一致性等级 */
type ConsensusLevel = "high" | "medium" | "low" | "conflict" | "fallback";

/** 根据一致性分数和决策状态判定等级 */
export function getConsensusLevel(
  score: number | null,
  isContradictory: boolean,
  isFallback: boolean,
): ConsensusLevel {
  if (isFallback) { return "fallback"; }
  if (isContradictory) { return "conflict"; }
  if (score === null) { return "medium"; }
  if (score >= 80) { return "high"; }
  if (score >= 60) { return "medium"; }
  return "low";
}

const LEVEL_COLORS: Record<ConsensusLevel, { bar: string; text: string; bg: string }> = {
  high: { bar: "#10b981", text: "#10b981", bg: "rgba(16,185,129,0.10)" },
  medium: { bar: "#f59e0b", text: "#f59e0b", bg: "rgba(245,158,11,0.10)" },
  low: { bar: "#ef4444", text: "#ef4444", bg: "rgba(239,68,68,0.10)" },
  conflict: { bar: "#ef4444", text: "#ef4444", bg: "rgba(239,68,68,0.15)" },
  fallback: { bar: "#6b7280", text: "#6b7280", bg: "rgba(107,114,128,0.10)" },
};

interface DecisionConsensusMeterProps {
  /** 双视角一致性分数 (0-100)，null 表示无 LLM 决策 */
  agreementScore: number | null;
  /** 决策是否矛盾 */
  isContradictory: boolean;
  /** 决策是否来自降级路径 */
  isFallback: boolean;
  /** 分维度诊断 */
  agreementBreakdown?: AgreementBreakdown | null;
  /** 紧凑模式（移动端/简洁模式） */
  compact?: boolean;
}

/**
 * 决策一致性仪表盘 — 公式决策 vs LLM 决策的一致性可视化。
 *
 * 三档颜色：
 * - 高一致（≥80）：绿色
 * - 中一致（60-79）：黄色
 * - 低一致（<60）：橙色
 * - 决策冲突（isContradictory）：红色 + 警告
 * - 决策降级（isFallback）：灰色
 */
export function DecisionConsensusMeter({
  agreementScore,
  isContradictory,
  isFallback,
  agreementBreakdown,
  compact = false,
}: DecisionConsensusMeterProps) {
  const { t } = useTranslation();
  const level = getConsensusLevel(agreementScore, isContradictory, isFallback);
  const colors = LEVEL_COLORS[level];

  // 无 LLM 决策时不渲染
  if (agreementScore === null && !isFallback) {
    return null;
  }

  const scoreLabel = isFallback
    ? t("workspace.decisionHero.consensusFallback")
    : agreementScore === null
    ? "—"
    : `${agreementScore}%`;

  const hintLabel = (() => {
    switch (level) {
      case "high":
        return t("workspace.decisionHero.consensusHigh");
      case "medium":
        return t("workspace.decisionHero.consensusMedium");
      case "low":
        return t("workspace.decisionHero.consensusLow");
      case "conflict":
        return t("workspace.decisionHero.consensusConflictHint");
      case "fallback":
        return t("workspace.decisionHero.consensusFallbackHint");
    }
  })();

  if (compact) {
    // ── 紧凑模式：单行条 + 分数 ──
    return (
      <div
        className="flex items-center gap-1.5 px-2 py-0.5 rounded"
        style={{ background: colors.bg }}
      >
        <span className="text-sm" style={{ color: "var(--muted)" }}>
          {t("workspace.decisionHero.consensus")}
        </span>
        <div
          className="relative rounded-full overflow-hidden"
          style={{ width: 60, height: 4, background: "var(--surface)" }}
        >
          <div
            style={{
              width: `${agreementScore ?? 0}%`,
              height: "100%",
              background: colors.bar,
              transition: "width 0.6s ease",
            }}
          />
        </div>
        <span
          className="font-mono text-sm font-semibold"
          style={{ color: colors.text }}
        >
          {scoreLabel}
        </span>
      </div>
    );
  }

  // ── 完整模式：公式 ◀── 一致性 ──▶ LLM + 分项 ──
  return (
    <div
      className="rounded p-2 space-y-1.5"
      style={{ background: colors.bg, border: `1px solid ${colors.bar}30` }}
    >
      {/* 主一致性条 */}
      <div className="flex items-center justify-between">
        <span className="text-sm font-medium" style={{ color: "var(--muted)" }}>
          {t("workspace.decisionHero.formula")}
          <span className="mx-1" style={{ color: "var(--muted)" }}>◀</span>
          <span className="font-semibold" style={{ color: colors.text }}>
            {scoreLabel}
          </span>
          <span className="mx-1" style={{ color: "var(--muted)" }}>▶</span>
          {t("workspace.decisionHero.llm")}
        </span>
        {(level === "conflict" || level === "fallback") && (
          <Tag color={level === "conflict" ? "red" : "default"} style={{ margin: 0 }}>
            {level === "conflict"
              ? `⚠ ${t("workspace.decisionHero.consensusConflict")}`
              : t("workspace.decisionHero.consensusFallback")}
          </Tag>
        )}
      </div>

      {/* 分项诊断 */}
      {agreementBreakdown && agreementScore !== null && agreementScore < 80 && (
        <div
          className="flex items-center gap-3 text-sm pt-1"
          style={{ borderTop: `1px solid ${colors.bar}20`, color: "var(--muted)" }}
        >
          {/* 行动一致性 */}
          <span className="flex items-center gap-1">
            {agreementBreakdown.actionOk ? "✓" : "✗"}
            <span>
              {agreementBreakdown.actionOk
                ? t("workspace.decisionHero.actionMatch")
                : t("workspace.decisionHero.actionMismatch")}
            </span>
            {!agreementBreakdown.actionOk && (
              <span className="font-mono">
                ({agreementBreakdown.formulaAction} vs {agreementBreakdown.llmAction})
              </span>
            )}
          </span>
          {/* 仓位差距 */}
          {agreementBreakdown.positionGap != null && (
            <span>
              {t("workspace.decisionHero.positionGap")}:
              <span className="font-mono font-semibold ml-0.5">
                {Math.round(agreementBreakdown.positionGap)}%
              </span>
            </span>
          )}
          {/* 信心差距 */}
          {agreementBreakdown.confidenceGap != null && (
            <span>
              {t("workspace.decisionHero.confidenceGap")}:
              <span className="font-mono font-semibold ml-0.5">
                {Math.round(agreementBreakdown.confidenceGap)}%
              </span>
            </span>
          )}
        </div>
      )}

      {/* 冲突/降级提示 */}
      {(level === "conflict" || level === "fallback") && (
        <div className="text-sm" style={{ color: colors.text }}>
          {hintLabel}
        </div>
      )}
    </div>
  );
}
