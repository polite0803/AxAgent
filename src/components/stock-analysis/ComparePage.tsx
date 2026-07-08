import { PageErrorBoundary } from "@/components/shared/ErrorBoundary";
import { PageHeader } from "./_shared/PageHeader";
import { CompareView } from "./CompareView";
import { PeersPanel } from "./PeersPanel";

/**
 * ComparePage — 对标研究
 * 覆盖:CompareView(多股对比)+ PeersPanel(同行业 peers)
 */
export function ComparePage() {
  return (
    <PageErrorBoundary title="Compare">
      <div className="flex h-full flex-col">
        <PageHeader titleKey="compare.title" backTo="/stock-analysis" />
        <div className="flex-1 overflow-auto p-4 space-y-4">
          <CompareView />
          <PeersPanel />
        </div>
      </div>
    </PageErrorBoundary>
  );
}
