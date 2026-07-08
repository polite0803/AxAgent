import { useStockAnalysisStore } from "@/stores/feature/stockAnalysisStore";
import { Empty } from "antd";
import { useTranslation } from "react-i18next";
import { DecisionPhaseOverview } from "./decision-timeline/DecisionPhaseOverview";
import { TimelineBody } from "./decision-timeline/TimelineBody";

/**
 * DecisionTimelinePanel — 把"决策"Tab 从静态卡片升级为时间线脊柱
 * 4 Phase: scan → diagnose → debate → decide
 * 复用 store 中 workflow-step-done 事件流自动写入的 timeline
 */
export function DecisionTimelinePanel() {
  const { t } = useTranslation();
  const timeline = useStockAnalysisStore((s) => s.timeline);
  const status = useStockAnalysisStore((s) => s.status);

  if (status === "idle") {
    return (
      <div className="p-4">
        <Empty
          image={Empty.PRESENTED_IMAGE_SIMPLE}
          description={t("stockAnalysis.timeline.idleHint")}
        />
      </div>
    );
  }

  return (
    <div className="p-2">
      {/* 4 Phase 横向概览条(借鉴 TradingAgents-CN 决策链路可视化) */}
      <DecisionPhaseOverview />
      {timeline.length === 0
        ? (
          <div className="text-xs italic px-2 py-3" style={{ color: "var(--muted)" }}>
            {t("stockAnalysis.timeline.emptyHint")}
          </div>
        )
        : <TimelineBody nodes={timeline} />}
    </div>
  );
}
