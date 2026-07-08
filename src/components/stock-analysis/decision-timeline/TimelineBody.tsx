import { useStockAnalysisStore } from "@/stores/feature/stockAnalysisStore";
import type { TimelineNode, TimelinePhase } from "@/types/stock-analysis";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { DecisionBanner } from "../DecisionBanner";
import { TimelineNodeCard } from "./TimelineNodeCard";

const PHASE_ORDER: TimelinePhase[] = ["scan", "diagnose", "debate", "decide"];

const PHASE_TKEY: Record<TimelinePhase, string> = {
  scan: "stockAnalysis.timeline.phase.scan",
  diagnose: "stockAnalysis.timeline.phase.diagnose",
  debate: "stockAnalysis.timeline.phase.debate",
  decide: "stockAnalysis.timeline.phase.decide",
};

interface PhaseSectionProps {
  phase: TimelinePhase;
  nodes: TimelineNode[];
}

/** 4 个 Phase 之一(竖向段):标题 + 状态点 + 节点列表 */
export function PhaseSection({ phase, nodes }: PhaseSectionProps) {
  const { t } = useTranslation();
  const [collapsed, setCollapsed] = useState(false);
  // decide 阶段末节点:DecisionBanner(仅在决策存在时渲染)
  const decision = useStockAnalysisStore((s) => s.decision);
  const showDecisionBanner = phase === "decide" && !!decision;

  const total = nodes.length;
  const done = nodes.filter((n) => n.status === "done").length;
  const failed = nodes.filter((n) => n.status === "failed").length;
  const running = nodes.filter((n) => n.status === "running").length;

  const statusColor = failed > 0
    ? "var(--sa-red)"
    : running > 0
    ? "var(--accent)"
    : done === total && total > 0
    ? "var(--sa-green)"
    : "var(--muted)";

  return (
    <div className="relative pl-6 pb-3">
      {/* 段间竖线 */}
      <div
        className="absolute top-3 bottom-0 left-2 w-px"
        style={{ background: "var(--border)" }}
      />
      {/* 段状态点 */}
      <div
        className="absolute top-1 left-0.5 w-3.5 h-3.5 rounded-full"
        style={{
          background: statusColor,
          boxShadow: running > 0 ? `0 0 0 4px ${statusColor}33` : "none",
        }}
      />

      <button
        type="button"
        className="flex items-center gap-2 text-sm font-semibold w-full text-left"
        onClick={() => setCollapsed(!collapsed)}
        style={{ color: "var(--color-text)" }}
      >
        <span className="text-sm" style={{ color: "var(--muted)" }}>
          {collapsed ? "▶" : "▼"}
        </span>
        <span>{t(PHASE_TKEY[phase])}</span>
        <span style={{ color: "var(--muted)", fontWeight: 400 }}>
          {done}/{total}
          {failed > 0 && <span style={{ color: "var(--sa-red)" }}>· {failed} failed</span>}
        </span>
      </button>

      {!collapsed && (
        <div className="mt-1 space-y-1.5">
          {nodes.length === 0
            ? (
              <div className="text-sm italic" style={{ color: "var(--muted)" }}>
                {t("stockAnalysis.timeline.empty")}
              </div>
            )
            : nodes.map((node) => <TimelineNodeCard key={node.id} node={node} />)}
          {showDecisionBanner && (
            <div className="mt-2">
              <DecisionBanner />
            </div>
          )}
        </div>
      )}
    </div>
  );
}

interface TimelineBodyProps {
  nodes: TimelineNode[];
}

/** 竖向 4 Phase 列表容器 */
export function TimelineBody({ nodes }: TimelineBodyProps) {
  return (
    <div className="px-2">
      {PHASE_ORDER.map((phase) => (
        <PhaseSection
          key={phase}
          phase={phase}
          nodes={nodes.filter((n) => n.phase === phase)}
        />
      ))}
    </div>
  );
}
