import { PageErrorBoundary } from "@/components/shared/ErrorBoundary";
import { SkillPageRenderer } from "@/components/skill/SkillPageRenderer";
import { useSkillExtensionStore } from "@/stores";
import { Spin } from "antd";
import { lazy, Suspense, useMemo } from "react";
import { Route, Routes, useLocation } from "react-router-dom";

const LazyChatPage = lazy(() => import("@/pages/ChatPage").then((m) => ({ default: m.ChatPage })));
const LazyKnowledgePage = lazy(() => import("@/pages/KnowledgePage").then((m) => ({ default: m.KnowledgePage })));
const LazyMemoryPage = lazy(() => import("@/pages/MemoryPage").then((m) => ({ default: m.MemoryPage })));
const LazyLinkPage = lazy(() => import("@/pages/LinkPage").then((m) => ({ default: m.LinkPage })));
const LazyGatewayPage = lazy(() => import("@/pages/GatewayPage").then((m) => ({ default: m.GatewayPage })));
const LazyFilesPage = lazy(() => import("@/pages/FilesPage").then((m) => ({ default: m.FilesPage })));
const LazySettingsPage = lazy(() => import("@/pages/SettingsPage").then((m) => ({ default: m.SettingsPage })));
const LazySkillsPage = lazy(() => import("@/pages/SkillsPage").then((m) => ({ default: m.SkillsPage })));
const LazyWorkflowPage = lazy(() => import("@/pages/WorkflowPage").then((m) => ({ default: m.WorkflowPage })));
const LazyTraceExplorer = lazy(() =>
  import("@/pages/DevTools/TraceExplorer").then((m) => ({ default: m.TraceExplorer }))
);
const LazyBenchmarkRunner = lazy(() =>
  import("@/pages/DevTools/BenchmarkRunner").then((m) => ({ default: m.BenchmarkRunner }))
);
const LazyToolRecommender = lazy(() =>
  import("@/pages/DevTools/ToolRecommender").then((m) => ({ default: m.ToolRecommender }))
);
const LazyFineTune = lazy(() => import("@/pages/FineTunePage").then((m) => ({ default: m.default })));
const LazyLlmWikiPage = lazy(() => import("@/pages/LlmWikiPage").then((m) => ({ default: m.LlmWikiPage })));
const LazyIngestPage = lazy(() => import("@/pages/IngestPage").then((m) => ({ default: m.IngestPage })));
const LazyWikiGraphPage = lazy(() => import("@/pages/WikiGraphPage").then((m) => ({ default: m.WikiGraphPage })));
const LazyQuickBarPage = lazy(() => import("@/pages/QuickBarPage").then((m) => ({ default: m.QuickBarPage })));

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

/** 动态技能页面：通过当前路径从 store 中匹配页面并渲染 */
function SkillRoutePage() {
  const location = useLocation();
  const pages = useSkillExtensionStore((s) => s.pages);

  const page = useMemo(() => {
    return pages.find((p) => p.path === location.pathname);
  }, [pages, location.pathname]);

  if (!page) {
    return (
      <div style={{ padding: 24, textAlign: "center", color: "var(--color-text-secondary)" }}>
        <Spin size="large" style={{ marginBottom: 16 }} />
        <div>Loading skill page...</div>
      </div>
    );
  }

  return (
    <SkillPageRenderer
      componentType={page.componentType}
      componentConfig={page.componentConfig}
      skillName={page.skillName}
    />
  );
}

const SkillPageByParam = lazy(() => import("@/components/skill/SkillPageByParam"));

export function ContentArea() {
  const skillPages = useSkillExtensionStore((s) => s.pages);

  const pluginRoutes = useMemo(() => {
    return skillPages.map((page) => (
      <Route
        key={page.path}
        path={page.path}
        element={<SkillRoutePage />}
      />
    ));
  }, [skillPages]);

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
      <Route path="/workflow" element={<SafeLazyPage Page={LazyWorkflowPage} />} />
      <Route path="/llm-wiki" element={<SafeLazyPage Page={LazyLlmWikiPage} />} />
      <Route path="/llm-wiki/:wikiId/graph" element={<SafeLazyPage Page={LazyWikiGraphPage} />} />
      <Route path="/llm-wiki/:wikiId/ingest" element={<SafeLazyPage Page={LazyIngestPage} />} />
      <Route path="/quickbar" element={<SafeLazyPage Page={LazyQuickBarPage} />} />
      <Route path="/devtools/trace-explorer" element={<SafeLazyPage Page={LazyTraceExplorer} />} />
      <Route path="/devtools/benchmark" element={<SafeLazyPage Page={LazyBenchmarkRunner} />} />
      <Route path="/devtools/tool-recommender" element={<SafeLazyPage Page={LazyToolRecommender} />} />
      <Route path="/devtools/fine-tune" element={<SafeLazyPage Page={LazyFineTune} />} />

      {/* 技能声明式动态路由 */}
      {pluginRoutes}

      {/* 技能 catch-all 路由 */}
      <Route path="/skill/:skillName" element={<SafeLazyPage Page={SkillPageByParam} />} />
      <Route path="/skill/:skillName/:pageId" element={<SafeLazyPage Page={SkillPageByParam} />} />
    </Routes>
  );
}
