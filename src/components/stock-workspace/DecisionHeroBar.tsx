// SPDX-License-Identifier: AGPL-3.0-only

import { useIsMobile } from "@/components/stock-analysis/MobileResponsive";
import { DecisionConsensusMeter } from "@/components/stock-workspace/DecisionConsensusMeter";
import { extractLlmField } from "@/lib/agentOutput";
import { getActionColor, getActionTKey, getRiskColor, getRiskTKey } from "@/lib/stock-analysis-utils";
import { useStockAnalysisStore, useWorkspaceStore } from "@/stores";
import { useTimeAnchorStore } from "@/stores/feature/timeAnchorStore";
import type { StockDecision } from "@/types";
import { Button, Drawer, Tag } from "antd";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

/**
 * 决策 Hero 条 — 工作区永久可见的决策摘要。
 *
 * 跨视图共享，用户在任何视图 Tab 下都能看到当前股票的决策核心信息。
 * 支持两种模式：
 * - 简洁模式（simple）：单行 action + 仓位 + 一致性 + 展开按钮
 * - 专业模式（professional）：多行指标 + 完整一致性仪表盘 + 详情 Drawer
 */
export function DecisionHeroBar() {
  const { t } = useTranslation();
  const userMode = useWorkspaceStore((s) => s.userMode);
  const isMobile = useIsMobile();
  const isSimple = userMode === "simple" || isMobile;

  const decision = useStockAnalysisStore((s) => s.decision);
  const stockCode = useStockAnalysisStore((s) => s.stockCode);
  const stockName = useStockAnalysisStore((s) => s.stockName);
  const quote = useStockAnalysisStore((s) => s.quote);
  const llmDecisionJson = useStockAnalysisStore((s) => s.llmDecisionJson);
  const decisionAgreementScore = useStockAnalysisStore((s) => s.decisionAgreementScore);
  const asOfDate = useTimeAnchorStore((s) => s.asOfDate);

  const [detailOpen, setDetailOpen] = useState(false);

  // 解析 LLM stance（用于一致性展示）
  const llmStance = useMemo(() => {
    if (!llmDecisionJson) { return null; }
    return (extractLlmField(llmDecisionJson, "action") as string | null)
      ?? (extractLlmField(llmDecisionJson, "stance") as string | null)
      ?? null;
  }, [llmDecisionJson]);

  // 预期涨幅
  const upside = useMemo(() => {
    if (!decision || !quote) { return null; }
    const target = decision.targetPrice != null ? Number(decision.targetPrice) : 0;
    if (target <= 0 || quote.price <= 0) { return null; }
    return ((target - quote.price) / quote.price) * 100;
  }, [decision, quote]);

  // ── 无决策占位 ──
  if (!decision) {
    return (
      <div
        className="flex items-center gap-2 px-3 py-2 rounded"
        style={{ background: "var(--surface)", borderLeft: "3px solid var(--muted)" }}
      >
        <span className="text-sm" style={{ color: "var(--muted)" }}>
          {t("workspace.decisionHero.noDecision")}
        </span>
        <span className="text-sm" style={{ color: "var(--muted)" }}>
          {stockCode
            ? t("workspace.decisionHero.noDecisionHint")
            : t("workspace.decisionHero.noDecisionNoStock")}
        </span>
      </div>
    );
  }

  const confidencePct = Math.round(decision.confidence ?? 0);
  const confidenceColor = confidencePct >= 70
    ? "var(--sa-green)"
    : confidencePct >= 45
    ? "var(--sa-amber)"
    : "var(--sa-red)";

  const actionColor = getActionColor(decision.action);

  // ── 简洁模式：单行紧凑条 ──
  if (isSimple) {
    return (
      <div
        className="flex items-center gap-2 px-3 py-1.5 rounded flex-wrap"
        style={{
          background: "var(--surface)",
          borderLeft: `3px solid var(--accent)`,
          fontSize: isMobile ? 12 : undefined,
        }}
      >
        {/* 股票名称 */}
        {stockName && (
          <span className="text-sm font-semibold" style={{ color: "var(--color-text-primary)" }}>
            {stockName}
          </span>
        )}
        {/* Action 标签 */}
        <Tag color={actionColor} style={{ margin: 0 }}>
          {t(getActionTKey(decision.action))}
        </Tag>
        {/* 仓位 */}
        <span className="text-sm font-mono" style={{ color: "var(--color-text-primary)" }}>
          {t("workspace.decisionHero.position")} {decision.positionPct}%
        </span>
        {/* 目标价 */}
        {decision.targetPrice && (
          <span className="text-sm font-mono" style={{ color: "var(--sa-green)" }}>
            {t("workspace.decisionHero.targetPrice")} ¥{decision.targetPrice}
          </span>
        )}
        {/* 预期涨幅 */}
        {upside != null && (
          <span
            className="text-sm font-mono"
            style={{ color: upside >= 0 ? "var(--sa-green)" : "var(--sa-red)" }}
          >
            {upside >= 0 ? "+" : ""}
            {upside.toFixed(1)}%
          </span>
        )}
        {/* 时间旅行标记 */}
        {asOfDate && (
          <Tag color="purple" style={{ margin: 0 }}>
            ⏪ {asOfDate}
          </Tag>
        )}
        {/* 一致性（紧凑） */}
        <DecisionConsensusMeter
          agreementScore={decisionAgreementScore}
          isContradictory={!!decision.isContradictory}
          isFallback={!!decision.isFallback}
          agreementBreakdown={decision.agreementBreakdown}
          compact
        />
        {/* 展开详情 */}
        <Button
          type="text"
          size="small"
          onClick={() => setDetailOpen(true)}
          className="ml-auto"
        >
          {t("workspace.decisionHero.viewDetail")} ▼
        </Button>
        <DecisionDetailDrawer
          open={detailOpen}
          onClose={() => setDetailOpen(false)}
          decision={decision}
          stockCode={stockCode}
          stockName={stockName}
          upside={upside}
          confidencePct={confidencePct}
          confidenceColor={confidenceColor}
          agreementScore={decisionAgreementScore}
          llmStance={llmStance}
          asOfDate={asOfDate}
        />
      </div>
    );
  }

  // ── 专业模式：多行完整指标 + 一致性仪表盘 ──
  return (
    <div
      className="rounded px-3 py-2 space-y-1.5"
      style={{
        background: "var(--surface)",
        borderLeft: `3px solid var(--accent)`,
        maxWidth: "100%",
        minWidth: 0,
        overflow: "hidden",
        wordBreak: "break-all",
        overflowWrap: "anywhere",
      }}
    >
      {/* 第一行：Action + 核心指标 */}
      <div className="flex items-center gap-3 flex-wrap">
        {/* 股票名称 */}
        {stockName && (
          <span className="font-semibold" style={{ color: "var(--color-text-primary)" }}>
            {stockName}
          </span>
        )}
        {/* Action */}
        <Tag color={actionColor} style={{ margin: 0 }}>
          {t(getActionTKey(decision.action))}
        </Tag>
        {/* 仓位 */}
        <span className="text-sm font-mono flex items-center gap-1">
          <span style={{ color: "var(--muted)" }}>{t("workspace.decisionHero.position")}</span>
          <span className="font-semibold">{decision.positionPct}%</span>
        </span>
        {/* 目标价 */}
        {decision.targetPrice && (
          <span className="text-sm font-mono flex items-center gap-1">
            <span style={{ color: "var(--muted)" }}>{t("workspace.decisionHero.targetPrice")}</span>
            <span className="font-semibold" style={{ color: "var(--sa-green)" }}>
              ¥{decision.targetPrice}
            </span>
          </span>
        )}
        {/* 止损 */}
        {decision.stopLoss && (
          <span className="text-sm font-mono flex items-center gap-1">
            <span style={{ color: "var(--muted)" }}>{t("workspace.decisionHero.stopLoss")}</span>
            <span className="font-semibold" style={{ color: "var(--sa-red)" }}>
              ¥{decision.stopLoss}
            </span>
          </span>
        )}
        {/* 预期涨幅 */}
        {upside != null && (
          <span className="text-sm font-mono flex items-center gap-1">
            <span style={{ color: "var(--muted)" }}>{t("workspace.decisionHero.expectedUpside")}</span>
            <span
              className="font-semibold"
              style={{ color: upside >= 0 ? "var(--sa-green)" : "var(--sa-red)" }}
            >
              {upside >= 0 ? "+" : ""}
              {upside.toFixed(1)}%
            </span>
          </span>
        )}
        {/* 风险等级 */}
        <span className="text-sm flex items-center gap-1">
          <span style={{ color: "var(--muted)" }}>{t("workspace.decisionHero.riskLevel")}</span>
          <span className="font-semibold" style={{ color: getRiskColor(decision.riskLevel) }}>
            {t(getRiskTKey(decision.riskLevel))}
          </span>
        </span>
        {/* 时间旅行标记 */}
        {asOfDate && (
          <Tag color="purple" style={{ margin: 0 }}>
            ⏪ {t("workspace.decisionHero.asOf")}: {asOfDate}
          </Tag>
        )}
      </div>

      {/* 第二行：信心 + 一致性仪表盘 */}
      <div className="flex items-center gap-2 flex-wrap">
        {/* 信心条 */}
        <div className="flex items-center gap-1.5 min-w-0">
          <span className="text-sm" style={{ color: "var(--muted)" }}>
            {t("workspace.decisionHero.confidence")}
          </span>
          <span className="font-mono font-semibold" style={{ color: confidenceColor }}>
            {confidencePct}%
          </span>
          {
            /* V68 修复(2026-07-30): 移除 adjustedConfidence 显示
              置信度完全由公式决策计算，与 LLM 决策无关 */
          }
          <div
            className="relative rounded-full overflow-hidden"
            style={{ width: 60, height: 4, background: "var(--color-border-tertiary)" }}
          >
            <div
              style={{
                width: `${confidencePct}%`,
                height: "100%",
                background: confidenceColor,
                transition: "width 0.6s ease",
              }}
            />
          </div>
        </div>

        {/* 一致性仪表盘（紧凑版） */}
        <DecisionConsensusMeter
          agreementScore={decisionAgreementScore}
          isContradictory={!!decision.isContradictory}
          isFallback={!!decision.isFallback}
          agreementBreakdown={decision.agreementBreakdown}
          compact
        />

        {/* 详情按钮 */}
        <Button
          type="text"
          size="small"
          onClick={() => setDetailOpen(true)}
          className="ml-auto"
        >
          {t("workspace.decisionHero.viewDetail")} ▼
        </Button>
      </div>

      <DecisionDetailDrawer
        open={detailOpen}
        onClose={() => setDetailOpen(false)}
        decision={decision}
        stockCode={stockCode}
        stockName={stockName}
        upside={upside}
        confidencePct={confidencePct}
        confidenceColor={confidenceColor}
        agreementScore={decisionAgreementScore}
        llmStance={llmStance}
        asOfDate={asOfDate}
      />
    </div>
  );
}

// ── 详情 Drawer（简洁/专业模式共用） ──

interface DecisionDetailDrawerProps {
  open: boolean;
  onClose: () => void;
  decision: StockDecision;
  stockCode: string | null;
  stockName: string | null;
  upside: number | null;
  confidencePct: number;
  confidenceColor: string;
  agreementScore: number | null;
  llmStance: string | null;
  asOfDate: string | null;
}

function DecisionDetailDrawer({
  open,
  onClose,
  decision,
  stockName,
  upside,
  confidencePct,
  confidenceColor,
  agreementScore,
  llmStance,
  asOfDate,
}: DecisionDetailDrawerProps) {
  const { t } = useTranslation();

  return (
    <Drawer
      title={
        <div className="flex items-center gap-2 flex-wrap">
          <span>{stockName ?? ""}</span>
          <Tag color={getActionColor(decision.action)}>
            {t(getActionTKey(decision.action))}
          </Tag>
          {asOfDate && <Tag color="purple">⏪ {asOfDate}</Tag>}
        </div>
      }
      open={open}
      onClose={onClose}
      width="min(640px, 90vw)"
    >
      <div className="space-y-4">
        {/* 信心 */}
        <div>
          <div className="flex justify-between items-center mb-1">
            <span className="text-sm" style={{ color: "var(--muted)" }}>
              {t("workspace.decisionHero.confidence")}
            </span>
            <span className="font-mono font-semibold" style={{ color: confidenceColor }}>
              {confidencePct}%
              {/* V68 修复: 移除 adjustedConfidence 显示 */}
            </span>
          </div>
          <div
            className="relative rounded-full overflow-hidden"
            style={{ height: 8, background: "var(--color-border-tertiary)" }}
          >
            <div
              style={{
                width: `${confidencePct}%`,
                height: "100%",
                background: confidenceColor,
                transition: "width 0.6s ease",
              }}
            />
          </div>
        </div>

        {/* 指标网格 */}
        <div
          className="grid gap-2"
          style={{ gridTemplateColumns: "repeat(auto-fill, minmax(140px, 1fr))" }}
        >
          {decision.targetPrice && (
            <div className="p-2 rounded" style={{ background: "var(--surface)" }}>
              <div className="text-sm" style={{ color: "var(--muted)" }}>
                {t("workspace.decisionHero.targetPrice")}
              </div>
              <div className="font-mono font-semibold">¥{decision.targetPrice}</div>
            </div>
          )}
          {decision.stopLoss && (
            <div className="p-2 rounded" style={{ background: "var(--surface)" }}>
              <div className="text-sm" style={{ color: "var(--muted)" }}>
                {t("workspace.decisionHero.stopLoss")}
              </div>
              <div className="font-mono font-semibold" style={{ color: "var(--sa-red)" }}>
                ¥{decision.stopLoss}
              </div>
            </div>
          )}
          <div className="p-2 rounded" style={{ background: "var(--surface)" }}>
            <div className="text-sm" style={{ color: "var(--muted)" }}>
              {t("workspace.decisionHero.position")}
            </div>
            <div className="font-mono font-semibold">{decision.positionPct}%</div>
          </div>
          {upside != null && (
            <div className="p-2 rounded" style={{ background: "var(--surface)" }}>
              <div className="text-sm" style={{ color: "var(--muted)" }}>
                {t("workspace.decisionHero.expectedUpside")}
              </div>
              <div
                className="font-mono font-semibold"
                style={{ color: upside >= 0 ? "var(--sa-green)" : "var(--sa-red)" }}
              >
                {upside >= 0 ? "+" : ""}
                {upside.toFixed(1)}%
              </div>
            </div>
          )}
          <div className="p-2 rounded" style={{ background: "var(--surface)" }}>
            <div className="text-sm" style={{ color: "var(--muted)" }}>
              {t("workspace.decisionHero.riskLevel")}
            </div>
            <div
              className="font-semibold"
              style={{ color: getRiskColor(decision.riskLevel) }}
            >
              {t(getRiskTKey(decision.riskLevel))}
            </div>
          </div>
          {decision.expectedHoldingDays && (
            <div className="p-2 rounded" style={{ background: "var(--surface)" }}>
              <div className="text-sm" style={{ color: "var(--muted)" }}>
                {t("workspace.decisionHero.holdingDays")}
              </div>
              <div className="font-mono font-semibold">
                {decision.expectedHoldingDays}
              </div>
            </div>
          )}
          {decision.targetTimeframe && (
            <div className="p-2 rounded" style={{ background: "var(--surface)" }}>
              <div className="text-sm" style={{ color: "var(--muted)" }}>
                {t("workspace.decisionHero.timeframe")}
              </div>
              <div className="font-mono font-semibold">{decision.targetTimeframe}</div>
            </div>
          )}
        </div>

        {/* 完整一致性仪表盘 */}
        {agreementScore !== null && (
          <DecisionConsensusMeter
            agreementScore={agreementScore}
            isContradictory={!!decision.isContradictory}
            isFallback={!!decision.isFallback}
            agreementBreakdown={decision.agreementBreakdown}
          />
        )}

        {/* LLM stance 摘要 */}
        {llmStance && agreementScore !== null && (
          <div
            className="text-sm p-2 rounded"
            style={{ background: "rgba(124,58,237,0.06)", borderLeft: "3px solid #7c3aed" }}
          >
            <span className="font-medium" style={{ color: "#7c3aed" }}>LLM</span>
            <Tag color={getActionColor(llmStance)} className="ml-2">
              {t(getActionTKey(llmStance))}
            </Tag>
          </div>
        )}

        {/* 推理过程 */}
        {decision.reasoning && (
          <div>
            <div className="text-sm mb-1" style={{ color: "var(--muted)" }}>
              {t("stockAnalysis.reasoning")}
            </div>
            <div
              className="text-sm p-3 rounded max-h-60 overflow-auto"
              style={{ background: "var(--surface)" }}
            >
              {decision.reasoning}
            </div>
          </div>
        )}
      </div>
    </Drawer>
  );
}
