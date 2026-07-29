// SPDX-License-Identifier: AGPL-3.0-only

import { AppHeader } from "@/components/layout/AppHeader";
import { IpcReconnectBanner } from "@/components/layout/IpcReconnectBanner";
import { PageErrorBoundary } from "@/components/shared/ErrorBoundary";
import { PageContextProvider } from "@/components/shared/PageContextProvider";
import { useIpcHealth } from "@/hooks/useIpcHealth";
import { BUILTIN_PAGE_PATH, DEFAULT_HOME } from "@/lib/pageRegistry";
import { useWorkspaceStore } from "@/stores";
import { Button, Result, Spin } from "antd";
import { lazy, memo, Suspense, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { Navigate, Route, Routes, useNavigate } from "react-router-dom";

// ── 页面 lazy 导入 ──
const LazyWorkspaceHub = lazy(() =>
  import("@/components/layout/WorkspaceHub").then((m) => ({ default: m.WorkspaceHub }))
);
const LazyMemoryPage = lazy(() => import("@/pages/MemoryPage").then((m) => ({ default: m.MemoryPage })));
const LazyGatewayLinkPage = lazy(() =>
  import("@/pages/GatewayLinkPage").then((m) => ({
    default: m.GatewayLinkPage,
  }))
);
const LazySettingsPage = lazy(() => import("@/pages/SettingsPage").then((m) => ({ default: m.SettingsPage })));
const LazyDevToolsPage = lazy(() => import("@/pages/DevTools/DevToolsPage").then((m) => ({ default: m.DevToolsPage })));
const LazyIngestPage = lazy(() => import("@/pages/IngestPage").then((m) => ({ default: m.IngestPage })));
const LazyWikiGraphPage = lazy(() => import("@/pages/WikiGraphPage").then((m) => ({ default: m.WikiGraphPage })));
const LazyWikiEditPage = lazy(() => import("@/pages/WikiEditPage").then((m) => ({ default: m.WikiEditPage })));
const LazyQuickBarPage = lazy(() => import("@/pages/QuickBarPage").then((m) => ({ default: m.QuickBarPage })));
const LazyLearningGraphPage = lazy(() =>
  import("@/pages/LearningGraphPage").then((m) => ({ default: m.LearningGraphPage }))
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

/**
 * 将旧路由（/dashboard, /workflow, /terminal, /knowledge）重定向到 /chat，
 * 并设置对应的工作台功能 Tab。
 */
function RedirectAndSetTab({ tab }: { tab: "dashboard" | "workflow" | "terminal" | "knowledge" }) {
  const setActiveTab = useWorkspaceStore((s) => s.setActiveTab);
  const navigate = useNavigate();
  useEffect(() => {
    setActiveTab(tab);
    navigate(BUILTIN_PAGE_PATH.chat, { replace: true });
  }, [tab, setActiveTab, navigate]);
  return null;
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
          {/* 工作台 Hub：对话页作为核心，内含仪表盘/工作流/终端/知识源 Tab */}
          <Route
            path={BUILTIN_PAGE_PATH.chat}
            element={
              <PageContextProvider page="chat">
                <SafeLazyPage Page={LazyWorkspaceHub} />
              </PageContextProvider>
            }
          />
          {/* 以下路由重定向到 /chat 并设置对应功能 Tab */}
          <Route path={BUILTIN_PAGE_PATH.dashboard} element={<RedirectAndSetTab tab="dashboard" />} />
          <Route path={BUILTIN_PAGE_PATH.workflow} element={<RedirectAndSetTab tab="workflow" />} />
          <Route path={BUILTIN_PAGE_PATH.terminal} element={<RedirectAndSetTab tab="terminal" />} />
          <Route path={BUILTIN_PAGE_PATH.knowledge} element={<RedirectAndSetTab tab="knowledge" />} />
          <Route path={BUILTIN_PAGE_PATH.marketplace} element={<RedirectAndSetTab tab="workflow" />} />
          <Route path={BUILTIN_PAGE_PATH.files} element={<RedirectAndSetTab tab="terminal" />} />
          {/* 记忆页保留独立路由（无侧栏入口，通过知识源内 Memory Tab 访问） */}
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
          {/* 知识库子路由（/llm-wiki 与 /knowledge 共用 KnowledgeHubPage，保留 wiki 上下文） */}
          <Route
            path={BUILTIN_PAGE_PATH.llmWiki}
            element={<RedirectAndSetTab tab="knowledge" />}
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
          {/* 动态页面查看器保留独立路由（通过 schemaId 访问） */}
          <Route
            path={`${BUILTIN_PAGE_PATH["dynamic-ui"]}/:schemaId`}
            element={
              <PageContextProvider page="dynamic-ui">
                <SafeLazyPage Page={LazyDynamicPageViewer} />
              </PageContextProvider>
            }
          />
          {/* 动态页面管理入口重定向到设置（已迁入设置/扩展分组） */}
          <Route
            path={BUILTIN_PAGE_PATH["dynamic-ui"]}
            element={<Navigate to={BUILTIN_PAGE_PATH.settings} replace />}
          />
          {/* 开发者工具统一入口（5 项合并为 1 项，内部 Tab 切换） */}
          <Route
            path={BUILTIN_PAGE_PATH.devtoolsTraceExplorer}
            element={<Navigate to={BUILTIN_PAGE_PATH.devtools} replace />}
          />
          <Route
            path={BUILTIN_PAGE_PATH.devtoolsBenchmark}
            element={<Navigate to={BUILTIN_PAGE_PATH.devtools} replace />}
          />
          <Route
            path={BUILTIN_PAGE_PATH.devtoolsToolRecommender}
            element={<Navigate to={BUILTIN_PAGE_PATH.devtools} replace />}
          />
          <Route
            path={BUILTIN_PAGE_PATH.devtoolsFineTune}
            element={<Navigate to={BUILTIN_PAGE_PATH.devtools} replace />}
          />
          <Route
            path={BUILTIN_PAGE_PATH.devtoolsRlTraining}
            element={<Navigate to={BUILTIN_PAGE_PATH.devtools} replace />}
          />
          <Route
            path={BUILTIN_PAGE_PATH.devtools}
            element={
              <PageContextProvider page="devtools">
                <SafeLazyPage Page={LazyDevToolsPage} />
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
