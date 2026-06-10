import { AppHeader } from "@/components/layout/AppHeader";
import { PageErrorBoundary } from "@/components/shared/ErrorBoundary";
import { SkillPageRenderer } from "@/components/skill/SkillPageRenderer";
import { useSkillExtensionStore } from "@/stores";
import { Button, Result, Spin } from "antd";
import { lazy, memo, Suspense, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { Route, Routes, useLocation, useNavigate } from "react-router-dom";

const LazyChatPage = lazy(() => import("@/pages/ChatPage").then((m) => ({ default: m.ChatPage })));
const LazyKnowledgeHubPage = lazy(() =>
  import("@/pages/KnowledgeHubPage").then((m) => ({
    default: m.KnowledgeHubPage,
  }))
);
const LazyMemoryPage = lazy(() => import("@/pages/MemoryPage").then((m) => ({ default: m.MemoryPage })));
const LazyGatewayLinkPage = lazy(() =>
  import("@/pages/GatewayLinkPage").then((m) => ({
    default: m.GatewayLinkPage,
  }))
);
const LazySettingsPage = lazy(() => import("@/pages/SettingsPage").then((m) => ({ default: m.SettingsPage })));
const LazyWorkflowPage = lazy(() => import("@/pages/WorkflowPage").then((m) => ({ default: m.WorkflowPage })));
const LazyTraceExplorer = lazy(() =>
  import("@/pages/DevTools/TraceExplorer").then((m) => ({
    default: m.TraceExplorer,
  }))
);
const LazyBenchmarkRunner = lazy(() =>
  import("@/pages/DevTools/BenchmarkRunner").then((m) => ({
    default: m.BenchmarkRunner,
  }))
);
const LazyToolRecommender = lazy(() =>
  import("@/pages/DevTools/ToolRecommender").then((m) => ({
    default: m.ToolRecommender,
  }))
);
const LazyFineTune = lazy(() => import("@/pages/FineTunePage").then((m) => ({ default: m.FineTunePage })));
const LazyIngestPage = lazy(() => import("@/pages/IngestPage").then((m) => ({ default: m.IngestPage })));
const LazyWikiGraphPage = lazy(() => import("@/pages/WikiGraphPage").then((m) => ({ default: m.WikiGraphPage })));
const LazyWikiEditPage = lazy(() => import("@/pages/WikiEditPage").then((m) => ({ default: m.WikiEditPage })));
const LazyQuickBarPage = lazy(() => import("@/pages/QuickBarPage").then((m) => ({ default: m.QuickBarPage })));
const LazyTerminalPage = lazy(() => import("@/pages/TerminalPage").then((m) => ({ default: m.TerminalPage })));
const LazyFilesPage = lazy(() => import("@/pages/FilesPage").then((m) => ({ default: m.FilesPage })));

function PageLoader() {
  return (
    <div
      className="flex items-center justify-center h-full w-full"
      style={{ minHeight: 200 }}
    >
      <Spin size="large" />
    </div>
  );
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
function SafeLazyPage({ Page }: { Page: React.LazyExoticComponent<any> }) {
  const { t } = useTranslation();
  return (
    <PageErrorBoundary title={t("error.page")}>
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
  const { t } = useTranslation();
  const pathname = location.pathname;

  const page = useMemo(() => {
    return pages.find((p) => `/skill/${p.skillName}/${p.id}` === pathname);
  }, [pages, pathname]);

  if (!page) {
    return (
      <div
        style={{
          padding: 24,
          textAlign: "center",
          color: "var(--color-text-secondary)",
        }}
      >
        <Spin size="large" style={{ marginBottom: 16 }} />
        <div>{t("skill.loadingPage")}</div>
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

const SkillPageByParam = lazy(
  () => import("@/components/skill/SkillPageByParam").then((m) => ({ default: m.SkillPageByParam })),
);

function NotFoundRoute() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  return (
    <div style={{ padding: 48, textAlign: "center" }}>
      <Result
        status="404"
        title="404"
        subTitle={t("error.pageNotFound")}
        extra={
          <Button type="primary" onClick={() => navigate("/")}>
            {t("common.back")}
          </Button>
        }
      />
    </div>
  );
}

export const ContentArea = memo(function ContentArea() {
  const skillPages = useSkillExtensionStore((s) => s.pages);

  const pluginRoutes = useMemo(() => {
    return skillPages.map((page) => (
      <Route
        key={page.id}
        path={`/skill/${page.skillName}/${page.id}`}
        element={<SkillRoutePage />}
      />
    ));
  }, [skillPages]);

  return (
    <div style={{ flex: 1, display: "flex", flexDirection: "column", minWidth: 0 }}>
      <AppHeader />
      <div style={{ flex: 1, overflow: "auto", display: "flex", flexDirection: "column", minWidth: 0 }}>
        <Routes>
          <Route path="/" element={<SafeLazyPage Page={LazyChatPage} />} />
          <Route
            path="/knowledge"
            element={<SafeLazyPage Page={LazyKnowledgeHubPage} />}
          />
          <Route
            path="/memory"
            element={<SafeLazyPage Page={LazyMemoryPage} />}
          />
          <Route
            path="/link"
            element={<SafeLazyPage Page={LazyGatewayLinkPage} />}
          />
          <Route
            path="/gateway"
            element={<SafeLazyPage Page={LazyGatewayLinkPage} />}
          />
          <Route
            path="/settings/*"
            element={<SafeLazyPage Page={LazySettingsPage} />}
          />
          <Route
            path="/workflow"
            element={<SafeLazyPage Page={LazyWorkflowPage} />}
          />
          <Route
            path="/llm-wiki"
            element={<SafeLazyPage Page={LazyKnowledgeHubPage} />}
          />
          <Route
            path="/llm-wiki/:wikiId/graph"
            element={<SafeLazyPage Page={LazyWikiGraphPage} />}
          />
          <Route
            path="/llm-wiki/:wikiId/ingest"
            element={<SafeLazyPage Page={LazyIngestPage} />}
          />
          <Route
            path="/llm-wiki/:wikiId/edit/:noteId"
            element={<SafeLazyPage Page={LazyWikiEditPage} />}
          />
          <Route path="/wiki" element={<SafeLazyPage Page={LazyWikiGraphPage} />} />
          <Route
            path="/wiki/:wikiId"
            element={<SafeLazyPage Page={LazyWikiGraphPage} />}
          />
          <Route
            path="/quickbar"
            element={<SafeLazyPage Page={LazyQuickBarPage} />}
          />
          <Route
            path="/files"
            element={<SafeLazyPage Page={LazyFilesPage} />}
          />
          <Route
            path="/terminal"
            element={<SafeLazyPage Page={LazyTerminalPage} />}
          />
          <Route
            path="/devtools/trace-explorer"
            element={<SafeLazyPage Page={LazyTraceExplorer} />}
          />
          <Route
            path="/devtools/benchmark"
            element={<SafeLazyPage Page={LazyBenchmarkRunner} />}
          />
          <Route
            path="/devtools/tool-recommender"
            element={<SafeLazyPage Page={LazyToolRecommender} />}
          />
          <Route
            path="/devtools/fine-tune"
            element={<SafeLazyPage Page={LazyFineTune} />}
          />

          {/* 技能声明式动态路由 */}
          {pluginRoutes}

          {/* 技能 catch-all 路由 */}
          <Route
            path="/skill/:skillName"
            element={<SafeLazyPage Page={SkillPageByParam} />}
          />
          <Route
            path="/skill/:skillName/:pageId"
            element={<SafeLazyPage Page={SkillPageByParam} />}
          />
          <Route path="*" element={<NotFoundRoute />} />
        </Routes>
      </div>
    </div>
  );
});
