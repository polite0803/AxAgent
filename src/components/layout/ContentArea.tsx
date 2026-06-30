// SPDX-License-Identifier: AGPL-3.0-only

import { AppHeader } from "@/components/layout/AppHeader";
import { IpcReconnectBanner } from "@/components/layout/IpcReconnectBanner";
import { PageErrorBoundary } from "@/components/shared/ErrorBoundary";
import { PageContextProvider } from "@/components/shared/PageContextProvider";
import { SkillPageRenderer } from "@/components/skill/SkillPageRenderer";
import { useIpcHealth } from "@/hooks/useIpcHealth";
import { useSkillExtensionStore } from "@/stores";
import { Button, Result, Spin } from "antd";
import { lazy, memo, Suspense, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { Navigate, Route, Routes, useLocation, useNavigate } from "react-router-dom";

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
const LazyRLTrainingPanel = lazy(() =>
  import("@/components/devtools/RLTrainingPanel").then((m) => ({
    default: m.RLTrainingPanel,
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

function SafeLazyPage({ Page }: { Page: React.LazyExoticComponent<React.ComponentType> }) {
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
  const { ipcHealthy } = useIpcHealth();
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
      <IpcReconnectBanner healthy={ipcHealthy} />
      <AppHeader />
      <div style={{ flex: 1, overflow: "auto", display: "flex", flexDirection: "column", minWidth: 0 }}>
        <Routes>
          <Route path="/" element={<Navigate to="/knowledge" replace />} />
          <Route
            path="/knowledge"
            element={
              <PageContextProvider page="knowledge">
                <SafeLazyPage Page={LazyKnowledgeHubPage} />
              </PageContextProvider>
            }
          />
          <Route
            path="/memory"
            element={
              <PageContextProvider page="memory">
                <SafeLazyPage Page={LazyMemoryPage} />
              </PageContextProvider>
            }
          />
          <Route
            path="/link"
            element={
              <PageContextProvider page="link">
                <SafeLazyPage Page={LazyGatewayLinkPage} />
              </PageContextProvider>
            }
          />
          <Route
            path="/gateway"
            element={
              <PageContextProvider page="gateway">
                <SafeLazyPage Page={LazyGatewayLinkPage} />
              </PageContextProvider>
            }
          />
          <Route
            path="/settings/*"
            element={
              <PageContextProvider page="settings">
                <SafeLazyPage Page={LazySettingsPage} />
              </PageContextProvider>
            }
          />
          <Route
            path="/workflow"
            element={
              <PageContextProvider page="workflow">
                <SafeLazyPage Page={LazyWorkflowPage} />
              </PageContextProvider>
            }
          />
          <Route
            path="/llm-wiki"
            element={
              <PageContextProvider page="wiki">
                <SafeLazyPage Page={LazyKnowledgeHubPage} />
              </PageContextProvider>
            }
          />
          <Route
            path="/llm-wiki/:wikiId/graph"
            element={
              <PageContextProvider page="wiki">
                <SafeLazyPage Page={LazyWikiGraphPage} />
              </PageContextProvider>
            }
          />
          <Route
            path="/llm-wiki/:wikiId/ingest"
            element={
              <PageContextProvider page="wiki">
                <SafeLazyPage Page={LazyIngestPage} />
              </PageContextProvider>
            }
          />
          <Route
            path="/llm-wiki/:wikiId/edit/:noteId"
            element={
              <PageContextProvider page="wiki">
                <SafeLazyPage Page={LazyWikiEditPage} />
              </PageContextProvider>
            }
          />
          <Route
            path="/wiki"
            element={
              <PageContextProvider page="wiki">
                <SafeLazyPage Page={LazyWikiGraphPage} />
              </PageContextProvider>
            }
          />
          <Route
            path="/wiki/:wikiId"
            element={
              <PageContextProvider page="wiki">
                <SafeLazyPage Page={LazyWikiGraphPage} />
              </PageContextProvider>
            }
          />
          <Route
            path="/quickbar"
            element={
              <PageContextProvider page="quickbar">
                <SafeLazyPage Page={LazyQuickBarPage} />
              </PageContextProvider>
            }
          />
          <Route
            path="/files"
            element={
              <PageContextProvider page="files">
                <SafeLazyPage Page={LazyFilesPage} />
              </PageContextProvider>
            }
          />
          <Route
            path="/terminal"
            element={
              <PageContextProvider page="terminal">
                <SafeLazyPage Page={LazyTerminalPage} />
              </PageContextProvider>
            }
          />
          <Route
            path="/devtools/trace-explorer"
            element={
              <PageContextProvider page="devtools">
                <SafeLazyPage Page={LazyTraceExplorer} />
              </PageContextProvider>
            }
          />
          <Route
            path="/devtools/benchmark"
            element={
              <PageContextProvider page="devtools">
                <SafeLazyPage Page={LazyBenchmarkRunner} />
              </PageContextProvider>
            }
          />
          <Route
            path="/devtools/tool-recommender"
            element={
              <PageContextProvider page="devtools">
                <SafeLazyPage Page={LazyToolRecommender} />
              </PageContextProvider>
            }
          />
          <Route
            path="/devtools/fine-tune"
            element={
              <PageContextProvider page="devtools">
                <SafeLazyPage Page={LazyFineTune} />
              </PageContextProvider>
            }
          />
          <Route
            path="/devtools/rl-training"
            element={
              <PageContextProvider page="devtools">
                <SafeLazyPage Page={LazyRLTrainingPanel} />
              </PageContextProvider>
            }
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
