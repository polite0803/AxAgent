import { PageErrorBoundary } from "@/components/shared/ErrorBoundary";
import { useStockAnalysisStore } from "@/stores/feature/stockAnalysisStore";
import { Tabs } from "antd";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { PageHeader } from "./_shared/PageHeader";
import { BacktestPanel } from "./BacktestPanel";
import { HistoricalAnalysisPanel } from "./HistoricalAnalysisPanel";
import { QuickBacktestPanel } from "./QuickBacktestPanel";
import { RecoSignalTimeline } from "./RecoSignalTimeline";
import { RecoStrategyMatrix } from "./RecoStrategyMatrix";
import { WhatIfBacktest } from "./WhatIfBacktest";

/**
 * BacktestPage — 回测验证
 * 覆盖:BacktestPanel(全量回测统计)+ HistoricalAnalysisPanel(历史分析列表 + 单次回测)
 * + WhatIfBacktest(参数修改回测)
 * analysisId 可选:BacktestPage 默认无聚焦分析,展示历史列表;有 store 当前 analysis 时,会显示其 blackboard snapshot
 */
export function BacktestPage() {
  const { t } = useTranslation();
  const analysisId = useStockAnalysisStore((s) => s.analysisId);
  const [selectedStrategy, setSelectedStrategy] = useState<string | null>(null);
  return (
    <PageErrorBoundary title="Backtest">
      <div className="flex h-full flex-col">
        <PageHeader titleKey="stockAnalysis.backtest.title" backTo="/stock-analysis" />
        <div className="flex-1 overflow-auto p-4">
          <Tabs
            size="small"
            items={[
              {
                key: "quick",
                label: t("stockAnalysis.backtest.quickBacktest") ?? "快速回测",
                children: <QuickBacktestPanel />,
              },
              {
                key: "analysis",
                label: t("stockAnalysis.backtest.tabAnalysis") ?? "分析回测",
                children: (
                  <div className="space-y-4">
                    <BacktestPanel />
                    <WhatIfBacktest />
                    <HistoricalAnalysisPanel analysisId={analysisId ?? ""} />
                  </div>
                ),
              },
              {
                key: "strategy",
                label: t("stockAnalysis.backtest.tabStrategy") ?? "策略回测",
                children: (
                  <div className="space-y-4">
                    <RecoStrategyMatrix onSelectStrategy={setSelectedStrategy} />
                    {selectedStrategy && <RecoSignalTimeline strategyId={selectedStrategy} />}
                  </div>
                ),
              },
            ]}
          />
        </div>
      </div>
    </PageErrorBoundary>
  );
}
