import { PageErrorBoundary } from "@/components/shared/ErrorBoundary";
import { PageHeader } from "./_shared/PageHeader";
import { ExecutionReplayPanel } from "./ExecutionReplayPanel";
import { PortfolioMonitorPanel } from "./PortfolioMonitorPanel";
import { TradePanel } from "./TradePanel";

/**
 * TradePage — 交易与回放
 * 覆盖:PortfolioMonitorPanel (组合监控) + TradePanel (交易面板) + ExecutionReplayPanel (执行回放)
 */
export function TradePage() {
  return (
    <PageErrorBoundary title="Trade">
      <div className="flex h-full flex-col">
        <PageHeader titleKey="trade.title" backTo="/stock-analysis" />
        <div className="flex-1 overflow-auto p-4 space-y-4">
          <PortfolioMonitorPanel />
          <TradePanel />
          <ExecutionReplayPanel />
        </div>
      </div>
    </PageErrorBoundary>
  );
}
