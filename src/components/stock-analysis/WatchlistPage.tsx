import { PageErrorBoundary } from "@/components/shared/ErrorBoundary";
import { PageHeader } from "./_shared/PageHeader";
import { DailyReviewPanel } from "./DailyReviewPanel";
import { PriceAlertPanel } from "./PriceAlertPanel";
import { WatchlistPanel } from "./WatchlistPanel";

/**
 * WatchlistPage — 自选股页面
 *
 * 布局：
 *   顶部：WatchlistPanel（全宽，自选股核心）
 *   下方：DailyReviewPanel + PriceAlertPanel（桌面端 2 列，移动端 1 列）
 */
export function WatchlistPage() {
  return (
    <PageErrorBoundary title="Watchlist">
      <div className="flex h-full flex-col">
        <PageHeader titleKey="watchlist.title" backTo="/stock-analysis" />
        <div className="flex-1 overflow-auto p-4 space-y-4">
          <WatchlistPanel />
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
            <DailyReviewPanel />
            <PriceAlertPanel />
          </div>
        </div>
      </div>
    </PageErrorBoundary>
  );
}
