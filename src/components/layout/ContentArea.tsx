// SPDX-License-Identifier: AGPL-3.0-only

import { AppHeader } from "@/components/layout/AppHeader";
import { IpcReconnectBanner } from "@/components/layout/IpcReconnectBanner";
import { PageErrorBoundary } from "@/components/shared/ErrorBoundary";
import { PageContextProvider } from "@/components/shared/PageContextProvider";
import { useIpcHealth } from "@/hooks/useIpcHealth";
import { BUILTIN_PAGE_PATH, DEFAULT_HOME } from "@/lib/pageRegistry";
import { Button, Result, Spin } from "antd";
import { lazy, memo, Suspense } from "react";
import { useTranslation } from "react-i18next";
import { Navigate, Route, Routes, useNavigate } from "react-router-dom";

const LazyChatPage = lazy(() =>
  import("@/pages/ChatPage").then((m) => ({
    default: m.ChatPage,
  }))
);
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
const LazyDashboardPage = lazy(() =>
  import("@/pages/DashboardPage").then((m) => ({
    default: m.DashboardPage,
  }))
);
const LazySettingsPage = lazy(() => import("@/pages/SettingsPage").then((m) => ({ default: m.SettingsPage })));
const LazyWorkflowPage = lazy(() => import("@/pages/WorkflowPage").then((m) => ({ default: m.WorkflowPage })));
const LazyWorkflowMarketplace = lazy(() =>
  import("@/pages/WorkflowMarketplace").then((m) => ({ default: m.WorkflowMarketplace }))
);
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
const LazyLearningGraphPage = lazy(() =>
  import("@/pages/LearningGraphPage").then((m) => ({ default: m.LearningGraphPage }))
);
const LazyDynamicUIManagerPage = lazy(() =>
  import("@/pages/DynamicUIManagerPage").then((m) => ({ default: m.DynamicUIManagerPage }))
);
const LazyDynamicPageViewer = lazy(() =>
  import("@/pages/DynamicPageViewer").then((m) => ({ default: m.DynamicPageViewer }))
);

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
          <Button type="primary" onClick={() => navigate(DEFAULT_HOME)}>
            {t("common.back")}
          </Button>
        }
      />
    </div>
  );
}

export const ContentArea = memo(function ContentArea() {
  const { ipcHealthy } = useIpcHealth();

  return (
    <div style={{ flex: 1, display: "flex", flexDirection: "column", minWidth: 0 }}>
      <IpcReconnectBanner healthy={ipcHealthy} />
      <AppHeader />
      <div style={{ flex: 1, overflow: "auto", display: "flex", flexDirection: "column", minWidth: 0 }}>
        <Routes>
          <Route path="/" element={<Navigate to={DEFAULT_HOME} replace />} />
          <Route
            path={BUILTIN_PAGE_PATH.chat}
            element={
              <PageContextProvider page="chat">
                <SafeLazyPage Page={LazyChatPage} />
              </PageContextProvider>
            }
          />
          <Route
            path={BUILTIN_PAGE_PATH.dashboard}
            element={
              <PageContextProvider page="dashboard">
                <SafeLazyPage Page={LazyDashboardPage} />
              </PageContextProvider>
            }
          />
          {
            /* /knowledge 与 /llm-wiki 共用 KnowledgeHubPage（LLM 知识库）。
               二者仅 page 上下文不同（"knowledge" vs "wiki"），属同组件双入口别名，
               保留 /llm-wiki 作为 wiki 数据源管理入口（含图谱子路由）。路径已收归 BUILTIN_PAGE_PATH，禁止散写。 */
          }
          <Route
            path={BUILTIN_PAGE_PATH.knowledge}
            element={
              <PageContextProvider page="knowledge">
                <SafeLazyPage Page={LazyKnowledgeHubPage} />
              </PageContextProvider>
            }
          />
          <Route
            path={BUILTIN_PAGE_PATH.memory}
            element={
              <PageContextProvider page="memory">
                <SafeLazyPage Page={LazyMemoryPage} />
              </PageContextProvider>
            }
          />
          {
            /* /link 与 /gateway 共用 GatewayLinkPage。
               /link 承载 OAuth / 网关连接回调（带 token 参数），page 上下文为 "link"；
               /gateway 为常规网关管理页，page 上下文为 "gateway"。
               二者语义不同（连接回调 vs 管理），不可合并。 */
          }
          <Route
            path={BUILTIN_PAGE_PATH.link}
            element={
              <PageContextProvider page="link">
                <SafeLazyPage Page={LazyGatewayLinkPage} />
              </PageContextProvider>
            }
          />
          <Route
            path={BUILTIN_PAGE_PATH.gateway}
            element={
              <PageContextProvider page="gateway">
                <SafeLazyPage Page={LazyGatewayLinkPage} />
              </PageContextProvider>
            }
          />
          <Route
            path={`${BUILTIN_PAGE_PATH.settings}/*`}
            element={
              <PageContextProvider page="settings">
                <SafeLazyPage Page={LazySettingsPage} />
              </PageContextProvider>
            }
          />
          <Route
            path={BUILTIN_PAGE_PATH.workflow}
            element={
              <PageContextProvider page="workflow">
                <SafeLazyPage Page={LazyWorkflowPage} />
              </PageContextProvider>
            }
          />
          <Route
            path={BUILTIN_PAGE_PATH.marketplace}
            element={
              <PageContextProvider page="marketplace">
                <SafeLazyPage Page={LazyWorkflowMarketplace} />
              </PageContextProvider>
            }
          />
          <Route
            path={BUILTIN_PAGE_PATH.llmWiki}
            element={
              <PageContextProvider page="wiki">
                <SafeLazyPage Page={LazyKnowledgeHubPage} />
              </PageContextProvider>
            }
          />
          <Route
            path={`${BUILTIN_PAGE_PATH.llmWiki}/:wikiId/graph`}
            element={
              <PageContextProvider page="wiki">
                <SafeLazyPage Page={LazyWikiGraphPage} />
              </PageContextProvider>
            }
          />
          <Route
            path={`${BUILTIN_PAGE_PATH.llmWiki}/:wikiId/ingest`}
            element={
              <PageContextProvider page="wiki">
                <SafeLazyPage Page={LazyIngestPage} />
              </PageContextProvider>
            }
          />
          <Route
            path={`${BUILTIN_PAGE_PATH.llmWiki}/:wikiId/edit/:noteId`}
            element={
              <PageContextProvider page="wiki">
                <SafeLazyPage Page={LazyWikiEditPage} />
              </PageContextProvider>
            }
          />
          <Route
            path={BUILTIN_PAGE_PATH.quickbar}
            element={
              <PageContextProvider page="quickbar">
                <SafeLazyPage Page={LazyQuickBarPage} />
              </PageContextProvider>
            }
          />
          <Route
            path={BUILTIN_PAGE_PATH.files}
            element={
              <PageContextProvider page="files">
                <SafeLazyPage Page={LazyFilesPage} />
              </PageContextProvider>
            }
          />
          <Route
            path={BUILTIN_PAGE_PATH.terminal}
            element={
              <PageContextProvider page="terminal">
                <SafeLazyPage Page={LazyTerminalPage} />
              </PageContextProvider>
            }
          />
          <Route
            path={BUILTIN_PAGE_PATH["dynamic-ui"]}
            element={
              <PageContextProvider page="dynamic-ui">
                <SafeLazyPage Page={LazyDynamicUIManagerPage} />
              </PageContextProvider>
            }
          />
          <Route
            path={`${BUILTIN_PAGE_PATH["dynamic-ui"]}/:schemaId`}
            element={
              <PageContextProvider page="dynamic-ui">
                <SafeLazyPage Page={LazyDynamicPageViewer} />
              </PageContextProvider>
            }
          />
          <Route
            path={BUILTIN_PAGE_PATH.devtoolsTraceExplorer}
            element={
              <PageContextProvider page="devtools">
                <SafeLazyPage Page={LazyTraceExplorer} />
              </PageContextProvider>
            }
          />
          <Route
            path={BUILTIN_PAGE_PATH.devtoolsBenchmark}
            element={
              <PageContextProvider page="devtools">
                <SafeLazyPage Page={LazyBenchmarkRunner} />
              </PageContextProvider>
            }
          />
          <Route
            path={BUILTIN_PAGE_PATH.devtoolsToolRecommender}
            element={
              <PageContextProvider page="devtools">
                <SafeLazyPage Page={LazyToolRecommender} />
              </PageContextProvider>
            }
          />
          <Route
            path={BUILTIN_PAGE_PATH.devtoolsFineTune}
            element={
              <PageContextProvider page="devtools">
                <SafeLazyPage Page={LazyFineTune} />
              </PageContextProvider>
            }
          />
          <Route
            path={BUILTIN_PAGE_PATH.devtoolsRlTraining}
            element={
              <PageContextProvider page="devtools">
                <SafeLazyPage Page={LazyRLTrainingPanel} />
              </PageContextProvider>
            }
          />

          {/* 学习图 */}
          <Route
            path={BUILTIN_PAGE_PATH.learningGraph}
            element={
              <PageContextProvider page="learning-graph">
                <SafeLazyPage Page={LazyLearningGraphPage} />
              </PageContextProvider>
            }
          />
          <Route path="*" element={<NotFoundRoute />} />
        </Routes>
      </div>
    </div>
  );
});
