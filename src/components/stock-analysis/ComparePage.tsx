import { PageErrorBoundary } from "@/components/shared/ErrorBoundary";
import { useTranslation } from "react-i18next";
import { PageHeader } from "./_shared/PageHeader";
import { CompareView } from "./CompareView";
import { PeersPanel } from "./PeersPanel";
import { SimulationVsBacktestCompare } from "./SimulationVsBacktestCompare";

/**
 * ComparePage — 对标研究
 * 覆盖:CompareView(多股对比)+ PeersPanel(同行业 peers)
 */
export function ComparePage() {
  const { t } = useTranslation();
  return (
    <PageErrorBoundary title={t("stockAnalysis.page.compare")}>
      <div className="flex h-full flex-col">
        <PageHeader titleKey="compare.title" backTo="/stock-analysis" />
        <div className="flex-1 overflow-auto p-4 space-y-4">
          <CompareView />
          <PeersPanel />
          <div className="bg-gray-900/50 rounded-lg p-4">
            <SimulationVsBacktestCompare
              report={{
                strategyName: "",
                stockCode: "",
                period: "",
                comparisons: [],
                backtestTrades: 0,
                simTrades: 0,
                backtestSummary: "运行回测和市场模拟后显示对比结果",
                simSummary: "需要同时运行回测和市场模拟才能生成对比报告",
              }}
            />
          </div>
        </div>
      </div>
    </PageErrorBoundary>
  );
}
