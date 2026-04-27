import { PageErrorBoundary } from "@/components/shared/ErrorBoundary";
import { Spin } from "antd";
import { lazy, Suspense } from "react";
import { Route, Routes } from "react-router-dom";

const LazyChatPage = lazy(() => import("@/pages/ChatPage").then((m) => ({ default: m.ChatPage })));
const LazyKnowledgePage = lazy(() => import("@/pages/KnowledgePage").then((m) => ({ default: m.KnowledgePage })));
const LazyMemoryPage = lazy(() => import("@/pages/MemoryPage").then((m) => ({ default: m.MemoryPage })));
const LazyLinkPage = lazy(() => import("@/pages/LinkPage").then((m) => ({ default: m.LinkPage })));
const LazyGatewayPage = lazy(() => import("@/pages/GatewayPage").then((m) => ({ default: m.GatewayPage })));
const LazyFilesPage = lazy(() => import("@/pages/FilesPage").then((m) => ({ default: m.FilesPage })));
const LazySettingsPage = lazy(() => import("@/pages/SettingsPage").then((m) => ({ default: m.SettingsPage })));
const LazySkillsPage = lazy(() => import("@/pages/SkillsPage").then((m) => ({ default: m.SkillsPage })));
const LazyWorkflowMarketplace = lazy(() => import("@/pages/WorkflowMarketplace").then((m) => ({ default: m.WorkflowMarketplace })));
const LazyPromptTemplatesPage = lazy(() => import("@/pages/PromptTemplatesPage").then((m) => ({ default: m.PromptTemplatesPage })));
const LazyTraceExplorer = lazy(() => import("@/pages/DevTools/TraceExplorer").then((m) => ({ default: m.TraceExplorer })));
const LazyBenchmarkRunner = lazy(() => import("@/pages/DevTools/BenchmarkRunner").then((m) => ({ default: m.BenchmarkRunner })));
const LazyToolRecommender = lazy(() => import("@/pages/DevTools/ToolRecommender").then((m) => ({ default: m.ToolRecommender })));

function PageLoader() {
  return (
    <div className="flex items-center justify-center h-full w-full" style={{ minHeight: 200 }}>
      <Spin size="large" />
    </div>
  );
}

function SafeLazyPage({ Page }: { Page: React.LazyExoticComponent<any> }) {
  return (
    <PageErrorBoundary title="Page Error">
      <Suspense fallback={<PageLoader />}>
        <Page />
      </Suspense>
    </PageErrorBoundary>
  );
}

export function ContentArea() {
  return (
    <Routes>
      <Route path="/" element={<SafeLazyPage Page={LazyChatPage} />} />
      <Route path="/knowledge" element={<SafeLazyPage Page={LazyKnowledgePage} />} />
      <Route path="/memory" element={<SafeLazyPage Page={LazyMemoryPage} />} />
      <Route path="/link" element={<SafeLazyPage Page={LazyLinkPage} />} />
      <Route path="/gateway" element={<SafeLazyPage Page={LazyGatewayPage} />} />
      <Route path="/files" element={<SafeLazyPage Page={LazyFilesPage} />} />
      <Route path="/settings/*" element={<SafeLazyPage Page={LazySettingsPage} />} />
      <Route path="/skills" element={<SafeLazyPage Page={LazySkillsPage} />} />
      <Route path="/marketplace" element={<SafeLazyPage Page={LazyWorkflowMarketplace} />} />
      <Route path="/prompts" element={<SafeLazyPage Page={LazyPromptTemplatesPage} />} />
      <Route path="/devtools/trace-explorer" element={<SafeLazyPage Page={LazyTraceExplorer} />} />
      <Route path="/devtools/benchmark" element={<SafeLazyPage Page={LazyBenchmarkRunner} />} />
      <Route path="/devtools/tool-recommender" element={<SafeLazyPage Page={LazyToolRecommender} />} />
    </Routes>
  );
}
