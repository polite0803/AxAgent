import { PageErrorBoundary } from "@/components/shared/ErrorBoundary";
import { PageHeader } from "@/components/stock-analysis/_shared/PageHeader";
import { ScheduledAnalysisPanel } from "@/components/stock-analysis/ScheduledAnalysisPanel";

export function ScheduledAnalysisPage() {
  return (
    <PageErrorBoundary title="Scheduled Analysis">
      <div className="flex h-full flex-col">
        <PageHeader titleKey="stockAnalysis.scheduledAnalysis.title" backTo="/portfolio" />
        <div className="flex-1 overflow-auto p-4">
          <ScheduledAnalysisPanel />
        </div>
      </div>
    </PageErrorBoundary>
  );
}
