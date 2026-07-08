// SPDX-License-Identifier: AGPL-3.0-only

/**
 * DecisionPhaseOverview — 4 Phase 横向进度概览条
 *
 * 借鉴 TradingAgents-CN 决策链路可视化:在 timeline 顶部
 * 用一条横向进度条展示 4 个 Phase(scan → diagnose → debate → decide)
 * 的完成度,中间节点显示当前执行到的位置。
 *
 * 增强点(2026-06):
 * - 4 phase 等宽分段,每段显示阶段名 + 完成度
 * - 当前 phase 高亮(蓝色光晕)
 * - 失败 phase 红色
 * - 终态(全 done)显示绿色
 */

import { useStockAnalysisStore } from "@/stores/feature/stockAnalysisStore";
import type { TimelineNode, TimelinePhase } from "@/types/stock-analysis";
import { theme } from "antd";
import { CheckCircle2, Circle, Loader2, XCircle } from "lucide-react";
import React, { useMemo } from "react";
import { useTranslation } from "react-i18next";

const PHASE_ORDER: TimelinePhase[] = ["scan", "diagnose", "debate", "decide"];

const PHASE_TKEY: Record<TimelinePhase, string> = {
  scan: "stockAnalysis.timeline.phase.scan",
  diagnose: "stockAnalysis.timeline.phase.diagnose",
  debate: "stockAnalysis.timeline.phase.debate",
  decide: "stockAnalysis.timeline.phase.decide",
};

interface PhaseStats {
  total: number;
  done: number;
  failed: number;
  running: number;
  state: "pending" | "running" | "done" | "failed" | "mixed";
}

function computePhaseStats(nodes: TimelineNode[]): Record<TimelinePhase, PhaseStats> {
  const empty: PhaseStats = { total: 0, done: 0, failed: 0, running: 0, state: "pending" };
  const out: Record<TimelinePhase, PhaseStats> = {
    scan: { ...empty },
    diagnose: { ...empty },
    debate: { ...empty },
    decide: { ...empty },
  };
  for (const node of nodes) {
    const s = out[node.phase];
    s.total += 1;
    if (node.status === "done") { s.done += 1; }
    else if (node.status === "failed") { s.failed += 1; }
    else if (node.status === "running") { s.running += 1; }
  }
  for (const phase of PHASE_ORDER) {
    const s = out[phase];
    if (s.total === 0) { s.state = "pending"; }
    else if (s.failed > 0 && s.done < s.total) { s.state = "failed"; }
    else if (s.running > 0) { s.state = "running"; }
    else if (s.done === s.total) { s.state = "done"; }
    else { s.state = "mixed"; }
  }
  return out;
}

export const DecisionPhaseOverview = React.memo(function DecisionPhaseOverview() {
  const { token } = theme.useToken();
  const { t } = useTranslation();
  const timeline = useStockAnalysisStore((s) => s.timeline);
  const status = useStockAnalysisStore((s) => s.status);

  const stats = useMemo(() => computePhaseStats(timeline), [timeline]);

  if (status === "idle" || timeline.length === 0) {
    return null;
  }

  return (
    <div
      className="px-3 py-2 mb-2 rounded-md"
      style={{
        backgroundColor: token.colorBgLayout,
        border: `1px solid ${token.colorBorderSecondary}`,
      }}
    >
      <div className="flex items-center justify-between mb-1.5">
        <span
          className="text-sm font-semibold uppercase tracking-wide"
          style={{ color: token.colorTextTertiary }}
        >
          {t("stockAnalysis.timeline.overviewTitle")}
        </span>
        <span
          className="text-sm tabular-nums"
          style={{ color: token.colorTextTertiary }}
        >
          {timeline.filter((n) => n.status === "done").length}/{timeline.length}
        </span>
      </div>

      {/* 4 Phase 横向进度条 */}
      <div
        className="relative h-6 rounded overflow-hidden flex"
        style={{ backgroundColor: token.colorFillTertiary }}
      >
        {PHASE_ORDER.map((phase, idx) => {
          const s = stats[phase];
          const pct = s.total > 0 ? (s.done / s.total) * 100 : 0;
          const isActive = s.state === "running";
          const isFailed = s.state === "failed";
          const isDone = s.state === "done" && s.total > 0;
          const bg = isFailed
            ? token.colorError
            : isDone
            ? token.colorSuccess
            : isActive
            ? token.colorPrimary
            : pct > 0
            ? token.colorPrimary
            : "transparent";
          const labelColor = pct > 30
            ? token.colorTextLightSolid
            : token.colorTextSecondary;
          return (
            <div
              key={phase}
              className="relative flex-1 flex items-center justify-center text-sm font-medium"
              style={{
                background: pct > 0
                  ? `linear-gradient(to right, ${bg} 0%, ${bg} ${pct}%, transparent ${pct}%, transparent 100%)`
                  : "transparent",
                borderRight: idx < PHASE_ORDER.length - 1
                  ? `1px solid ${token.colorBgContainer}`
                  : "none",
                color: labelColor,
                boxShadow: isActive ? `inset 0 0 0 1px ${token.colorPrimary}` : "none",
              }}
            >
              <span className="flex items-center gap-1 px-1 truncate">
                {isDone
                  ? <CheckCircle2 size={10} />
                  : isFailed
                  ? <XCircle size={10} />
                  : isActive
                  ? <Loader2 size={10} className="animate-spin" />
                  : <Circle size={10} />}
                <span className="truncate">{t(PHASE_TKEY[phase])}</span>
                {s.total > 0 && (
                  <span className="text-sm opacity-80">
                    {s.done}/{s.total}
                  </span>
                )}
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
});
