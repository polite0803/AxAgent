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
import { Navigate, Route, Routes, useNavigate, useParams } from "react-router-dom";

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
const LazyInvestPage = lazy(() => import("@/pages/InvestPage").then((m) => ({ default: m.InvestPage })));
const LazyIndustryPage = lazy(() => import("@/pages/IndustryPage").then((m) => ({ default: m.IndustryPage })));
const LazyAiResearchPage = lazy(() =>
  import("@/pages/opc/industries/IndustryPages").then((m) => ({ default: m.AiResearchPage }))
);
const LazySoftwareDevPage = lazy(() =>
  import("@/pages/opc/industries/IndustryPages").then((m) => ({ default: m.SoftwareDevPage }))
);
const LazyFinanceInvestPage = lazy(() =>
  import("@/pages/opc/industries/IndustryPages").then((m) => ({ default: m.FinanceInvestPage }))
);
const LazySalesGrowthPage = lazy(() =>
  import("@/pages/opc/industries/IndustryPages").then((m) => ({ default: m.SalesGrowthPage }))
);
const LazyContentMediaPage = lazy(() =>
  import("@/pages/opc/industries/IndustryPages").then((m) => ({ default: m.ContentMediaPage }))
);
const LazyIndustryConsultingPage = lazy(() =>
  import("@/pages/opc/industries/IndustryPages").then((m) => ({ default: m.IndustryConsultingPage }))
);
const LazyAccountingPage = lazy(() =>
  import("@/pages/opc/industries/IndustryPages").then((m) => ({ default: m.AccountingPage }))
);
const LazyEcommercePage = lazy(() =>
  import("@/pages/opc/industries/IndustryPages").then((m) => ({ default: m.EcommercePage }))
);
const LazyEducationPage = lazy(() =>
  import("@/pages/opc/industries/IndustryPages").then((m) => ({ default: m.EducationPage }))
);
const LazyDesignPage = lazy(() =>
  import("@/pages/opc/industries/IndustryPages").then((m) => ({ default: m.DesignPage }))
);
const LazyProjectManagementPage = lazy(() =>
  import("@/pages/opc/industries/IndustryPages").then((m) => ({ default: m.ProjectManagementPage }))
);
const LazySecurityPage = lazy(() =>
  import("@/pages/opc/industries/IndustryPages").then((m) => ({ default: m.SecurityPage }))
);
const LazyGeospatialPage = lazy(() =>
  import("@/pages/opc/industries/IndustryPages").then((m) => ({ default: m.GeospatialPage }))
);
const LazyGameDevPage = lazy(() =>
  import("@/pages/opc/industries/IndustryPages").then((m) => ({ default: m.GameDevPage }))
);
const LazyIndustryNavigatorPage = lazy(() =>
  import("@/pages/opc/industries/IndustryNavigator").then((m) => ({ default: m.IndustryNavigatorPage }))
);
const LazyOpcSubPage = lazy(() => import("@/pages/OpcPage").then((m) => ({ default: m.OpcSubPage })));

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
 * 旧路由重定向到 /chat，通过 location.state.tab 传递目标功能 Tab。
 * WorkspaceHub 读取 state 并切换 Tab。
 */
function redirectToChat(tab: string) {
  return <Navigate to={BUILTIN_PAGE_PATH.chat} replace state={{ tab }} />;
}

/**
 * 旧股票业务路由重定向到 /invest?tab=xxx。
 * InvestHub 读取 query 参数激活对应 Tab。
 * 附加参数（stockCode/view）以 query 形式传递，workspace tab 会消费。
 */
function redirectToInvest(
  tab: string,
  extra?: { stockCode?: string; view?: string },
) {
  const params = new URLSearchParams({ tab });
  if (extra?.stockCode) { params.set("stockCode", extra.stockCode); }
  if (extra?.view) { params.set("view", extra.view); }
  return <Navigate to={`${BUILTIN_PAGE_PATH.invest}?${params.toString()}`} replace />;
}

/** 从路径参数读取 stockCode 并重定向到 /invest?tab=workspace&stockCode=xxx */
function RedirectStockWorkspace() {
  const { stockCode } = useParams<{ stockCode?: string }>();
  return redirectToInvest("workspace", { stockCode });
}

/** 从路径参数读取 analysis ID 并重定向到 /invest?tab=workspace&view=analysis&analysisId=xxx */
function RedirectStockAnalysisById() {
  const { id } = useParams<{ id: string }>();
  const params = new URLSearchParams({ tab: "workspace", view: "analysis" });
  if (id) { params.set("analysisId", id); }
  return <Navigate to={`${BUILTIN_PAGE_PATH.invest}?${params.toString()}`} replace />;
}

/** 从行业 ID 映射到对应的页面组件 */
const INDUSTRY_PAGE_MAP: Record<string, React.LazyExoticComponent<React.ComponentType>> = {
  "ai-research": LazyAiResearchPage,
  "software-dev": LazySoftwareDevPage,
  "finance-invest": LazyFinanceInvestPage,
  "sales-growth": LazySalesGrowthPage,
  "content-media": LazyContentMediaPage,
  "industry-consulting": LazyIndustryConsultingPage,
  accounting: LazyAccountingPage,
  ecommerce: LazyEcommercePage,
  education: LazyEducationPage,
  design: LazyDesignPage,
  "project-management": LazyProjectManagementPage,
  security: LazySecurityPage,
  geospatial: LazyGeospatialPage,
  "game-dev": LazyGameDevPage,
  _default: LazyIndustryPage, // 保留作为后备
};

/** 根据行业 ID 渲染对应的页面组件 */
function renderIndustryPage(industryId: string) {
  const Page = INDUSTRY_PAGE_MAP[industryId] || INDUSTRY_PAGE_MAP._default;
  return (
    <PageContextProvider page="opc">
      <SafeLazyPage Page={Page} />
    </PageContextProvider>
  );
}

/** 从路径参数读取行业 ID 并重定向到对应的独立页面 */
function RedirectIndustryPath() {
  const { industryId } = useParams<{ industryId?: string }>();
  if (!industryId) {
    return <Navigate to={BUILTIN_PAGE_PATH.opc} replace />;
  }
  return <Navigate to={`${BUILTIN_PAGE_PATH.opcIndustryDynamic}/${industryId}`} replace />;
}

/** 9 个硬编码行业路由重定向到 /opc/industry/:id 动态路由 */
function redirectToIndustry(id: string) {
  return <Navigate to={`${BUILTIN_PAGE_PATH.opcIndustryDynamic}/${id}`} replace />;
}

/** 根据路由参数中的 id 选择对应的行业页面组件 */
function IndustryRouter() {
  const { id } = useParams<{ id?: string }>();
  if (!id) {
    return <Navigate to={BUILTIN_PAGE_PATH.opc} replace />;
  }
  return renderIndustryPage(id);
}

export const ContentArea = memo(function ContentArea() {
  const { ipcHealthy } = useIpcHealth();

  return (
    <div style={{ flex: 1, minHeight: 0, display: "flex", flexDirection: "column", minWidth: 0 }}>
      <IpcReconnectBanner healthy={ipcHealthy} />
      <AppHeader />
      <div
        style={{ flex: 1, minHeight: 0, overflow: "hidden", display: "flex", flexDirection: "column", minWidth: 0 }}
      >
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
          {/* 以下路由重定向到 /chat 并通过 state.tab 设置功能 Tab */}
          <Route path={BUILTIN_PAGE_PATH.dashboard} element={redirectToChat("dashboard")} />
          <Route path={BUILTIN_PAGE_PATH.workflow} element={redirectToChat("workflow")} />
          <Route path={BUILTIN_PAGE_PATH.terminal} element={redirectToChat("terminal")} />
          <Route path={BUILTIN_PAGE_PATH.files} element={redirectToChat("files")} />
          <Route path={BUILTIN_PAGE_PATH.knowledge} element={redirectToChat("knowledge")} />
          <Route path={BUILTIN_PAGE_PATH.multiAgent} element={redirectToChat("multiAgent")} />
          <Route path={BUILTIN_PAGE_PATH.marketplace} element={redirectToChat("workflow")} />
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
            element={redirectToChat("knowledge")}
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
          {/* AxInvest 股票业务统一入口 — 7 个业务页面合并为 InvestHub Tab */}
          <Route
            path={BUILTIN_PAGE_PATH.invest}
            element={
              <PageContextProvider page="invest">
                <SafeLazyPage Page={LazyInvestPage} />
              </PageContextProvider>
            }
          />
          {/* OPC 一人公司管理 — 根路径重定向到仪表板 */}
          <Route
            path={BUILTIN_PAGE_PATH.opc}
            element={<Navigate to={`${BUILTIN_PAGE_PATH.opc}/dashboard`} replace />}
          />
          {/* OPC 行业相关路由（必须在 /opc/:tab 之前，否则会被错误匹配） */}
          {/* OPC 旧复数行业路由（重定向到单数动态路由） */}
          <Route
            path={`${BUILTIN_PAGE_PATH.opc}/industries/:industryId`}
            element={<RedirectIndustryPath />}
          />
          {/* OPC 9 大垂直行业路由（重定向到 /opc/industry/:id 动态路由） */}
          <Route path={BUILTIN_PAGE_PATH.opcIndustryAiResearch} element={redirectToIndustry("ai-research")} />
          <Route path={BUILTIN_PAGE_PATH.opcIndustrySoftwareDev} element={redirectToIndustry("software-dev")} />
          <Route path={BUILTIN_PAGE_PATH.opcIndustryFinanceInvest} element={redirectToIndustry("finance-invest")} />
          <Route path={BUILTIN_PAGE_PATH.opcIndustrySalesGrowth} element={redirectToIndustry("sales-growth")} />
          <Route path={BUILTIN_PAGE_PATH.opcIndustryContentMedia} element={redirectToIndustry("content-media")} />
          <Route
            path={BUILTIN_PAGE_PATH.opcIndustryIndustryConsulting}
            element={redirectToIndustry("industry-consulting")}
          />
          <Route path={BUILTIN_PAGE_PATH.opcIndustryAccounting} element={redirectToIndustry("accounting")} />
          <Route path={BUILTIN_PAGE_PATH.opcIndustryEcommerce} element={redirectToIndustry("ecommerce")} />
          <Route path={BUILTIN_PAGE_PATH.opcIndustryEducation} element={redirectToIndustry("education")} />
          {/* OPC 新增 6 个行业路由 */}
          <Route path={BUILTIN_PAGE_PATH.opcIndustryDesign} element={redirectToIndustry("design")} />
          <Route
            path={BUILTIN_PAGE_PATH.opcIndustryProjectManagement}
            element={redirectToIndustry("project-management")}
          />
          <Route path={BUILTIN_PAGE_PATH.opcIndustrySecurity} element={redirectToIndustry("security")} />
          <Route path={BUILTIN_PAGE_PATH.opcIndustryGeospatial} element={redirectToIndustry("geospatial")} />
          <Route path={BUILTIN_PAGE_PATH.opcIndustryGameDev} element={redirectToIndustry("game-dev")} />
          {/* OPC 行业动态路由（单数形式，根据 ID 渲染对应的独立页面） */}
          <Route
            path={`${BUILTIN_PAGE_PATH.opcIndustryDynamic}/:id`}
            element={<IndustryRouter />}
          />
          {/* OPC 行业导航页面（集中展示所有行业入口） */}
          <Route
            path={BUILTIN_PAGE_PATH.opcIndustries}
            element={
              <PageContextProvider page="opc">
                <SafeLazyPage Page={LazyIndustryNavigatorPage} />
              </PageContextProvider>
            }
          />
          {/* OPC 独立子页面路由（仪表盘、发票、客户、项目等）— 直接渲染对应组件 */}
          <Route
            path={`${BUILTIN_PAGE_PATH.opc}/:tab`}
            element={
              <PageContextProvider page="opc">
                <SafeLazyPage Page={LazyOpcSubPage} />
              </PageContextProvider>
            }
          />
          {/* ── 旧股票业务路由重定向到 /invest?tab=xxx ── */}
          {/* /workspace → workspace tab（单股深度工作区） */}
          <Route path={BUILTIN_PAGE_PATH.workspace} element={redirectToInvest("workspace")} />
          <Route path={`${BUILTIN_PAGE_PATH.workspace}/:stockCode`} element={<RedirectStockWorkspace />} />
          {/* /stock-analysis → workspace tab（股票分析归入工作区 analysis 视图） */}
          <Route
            path={BUILTIN_PAGE_PATH["stock-analysis"]}
            element={redirectToInvest("workspace", { view: "analysis" })}
          />
          <Route
            path={`${BUILTIN_PAGE_PATH["stock-analysis"]}/:id`}
            element={<RedirectStockAnalysisById />}
          />
          <Route
            path={`${BUILTIN_PAGE_PATH["stock-analysis"]}/*`}
            element={redirectToInvest("workspace", { view: "analysis" })}
          />
          {/* /screener → screener tab（选股） */}
          <Route path={BUILTIN_PAGE_PATH.screener} element={redirectToInvest("screener")} />
          {/* /watchlist → workspace tab monitor 视图（自选股跟踪） */}
          <Route path={BUILTIN_PAGE_PATH.watchlist} element={redirectToInvest("workspace", { view: "monitor" })} />
          {/* /portfolio → workspace tab monitor 视图（持仓跟踪） */}
          <Route path={BUILTIN_PAGE_PATH.portfolio} element={redirectToInvest("workspace", { view: "monitor" })} />
          {/* /paper-portfolio → paper-portfolio tab（模拟观察） */}
          <Route path={BUILTIN_PAGE_PATH["paper-portfolio"]} element={redirectToInvest("paper-portfolio")} />
          {/* /market-mainline → market-mainline tab（市场主线） */}
          <Route path={BUILTIN_PAGE_PATH["market-mainline"]} element={redirectToInvest("market-mainline")} />
          {/* /screenshot-diagnosis → screenshot-diagnosis tab（截图诊断） */}
          <Route path={BUILTIN_PAGE_PATH["screenshot-diagnosis"]} element={redirectToInvest("screenshot-diagnosis")} />
          {/* /trade → workspace tab trade 视图（交易） */}
          <Route path={BUILTIN_PAGE_PATH.trade} element={redirectToInvest("workspace", { view: "trade" })} />
          {/* /backtest → workspace tab backtest 视图（回测） */}
          <Route path={BUILTIN_PAGE_PATH.backtest} element={redirectToInvest("workspace", { view: "backtest" })} />
          {/* /compare → workspace tab compare 视图（对比） */}
          <Route path={BUILTIN_PAGE_PATH.compare} element={redirectToInvest("workspace", { view: "compare" })} />
          {/* /scheduled-analysis → 无对应 tab，重定向到默认（market-mainline） */}
          <Route path={BUILTIN_PAGE_PATH["scheduled-analysis"]} element={redirectToInvest("market-mainline")} />
          {/* /quant → quant tab（量化策略验证） */}
          <Route path={BUILTIN_PAGE_PATH.quant} element={redirectToInvest("quant")} />
          {/* /replay-workbench → 无对应 tab，重定向到默认 */}
          <Route path={BUILTIN_PAGE_PATH["replay-workbench"]} element={redirectToInvest("market-mainline")} />
          {/* /pipeline → pipeline tab（流程编排） */}
          <Route path={BUILTIN_PAGE_PATH.pipeline} element={redirectToInvest("pipeline")} />
          {/* 开发工具已并入对话页「开发工具」Tab（见 WorkspaceSwitcher 门控），旧路由/子路由重定向到 /chat + state.tab */}
          <Route path={BUILTIN_PAGE_PATH.devtools} element={redirectToChat("devtools")} />
          <Route path={BUILTIN_PAGE_PATH.devtoolsTraceExplorer} element={redirectToChat("devtools")} />
          <Route path={BUILTIN_PAGE_PATH.devtoolsBenchmark} element={redirectToChat("devtools")} />
          <Route path={BUILTIN_PAGE_PATH.devtoolsToolRecommender} element={redirectToChat("devtools")} />
          <Route path={BUILTIN_PAGE_PATH.devtoolsFineTune} element={redirectToChat("devtools")} />
          <Route path={BUILTIN_PAGE_PATH.devtoolsRlTraining} element={redirectToChat("devtools")} />
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
