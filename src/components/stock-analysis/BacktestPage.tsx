import { PageErrorBoundary } from "@/components/shared/ErrorBoundary";
import { useStockAnalysisStore } from "@/stores/feature/stockAnalysisStore";
import { Tabs } from "antd";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { PageHeader } from "./_shared/PageHeader";
import { BacktestChart } from "./BacktestChart";
import { BacktestPanel } from "./BacktestPanel";
import { HistoricalAnalysisPanel } from "./HistoricalAnalysisPanel";
import { MarketSimPanel } from "./MarketSimPanel";
import { MonteCarloPanel } from "./MonteCarloPanel";
import { PnLHistogram, SectorAllocationDonut } from "./PortfolioCharts";
import { QuantSimPanel } from "./QuantSimPanel";
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
    <PageErrorBoundary title={t("stockAnalysis.page.backtest")}>
      <div className="flex h-full flex-col">
        <PageHeader titleKey="stockAnalysis.backtest.title" backTo="/stock-analysis" />
        <div className="flex-1 overflow-auto p-4">
          <Tabs
            size="small"
            items={[
              {
                key: "quick",
                label: t("stockAnalysis.backtest.quickBacktest"),
                children: <QuickBacktestPanel />,
              },
              {
                key: "analysis",
                label: t("stockAnalysis.backtest.tabAnalysis"),
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
                label: t("stockAnalysis.backtest.tabStrategy"),
                children: (
                  <div className="space-y-4">
                    <RecoStrategyMatrix onSelectStrategy={setSelectedStrategy} />
                    {selectedStrategy && <RecoSignalTimeline strategyId={selectedStrategy} />}
                  </div>
                ),
              },
              {
                key: "simulation",
                label: `🏭 ${t("stockAnalysis.backtest.tabSimulation")}`,
                children: <MarketSimPanel />,
              },
              {
                key: "mc",
                label: `🎲 ${t("stockAnalysis.backtest.tabMonteCarlo")}`,
                children: <MonteCarloPanel />,
              },
              {
                key: "quant_sim",
                label: `🤖 ${t("stockAnalysis.backtest.tabQuantSim")}`,
                children: <QuantSimPanel />,
              },
              {
                key: "charts",
                label: t("stockAnalysis.backtest.tabCharts"),
                children: (
                  <div className="space-y-4">
                    <BacktestChart
                      equityCurve={[]}
                      metrics={{
                        strategyName: "",
                        totalReturn: 0,
                        annualizedReturn: 0,
                        sharpe: 0,
                        maxDrawdown: 0,
                        winRate: 0,
                        totalTrades: 0,
                      }}
                    />
                    <SectorAllocationDonut data={[]} />
                    <PnLHistogram data={[]} />
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
