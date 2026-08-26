// SPDX-License-Identifier: AGPL-3.0-only

import { AppHeader } from "@/components/layout/AppHeader";
import { IpcReconnectBanner } from "@/components/layout/IpcReconnectBanner";
import { PageErrorBoundary } from "@/components/shared/ErrorBoundary";
import { PageContextProvider } from "@/components/shared/PageContextProvider";
import { useIpcHealth } from "@/hooks/useIpcHealth";
import { CAPABILITY_DOMAIN_META } from "@/lib/domainMeta";
import { BUILTIN_PAGE_PATH, DEFAULT_HOME } from "@/lib/pageRegistry";

import { Button, Result, Spin } from "antd";
import { lazy, memo, Suspense } from "react";
import { useTranslation } from "react-i18next";
import { Navigate, Route, Routes, useLocation, useNavigate, useParams } from "react-router-dom";

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
const LazyDomainHubPage = lazy(() => import("@/pages/DomainHubPage").then((m) => ({ default: m.DomainHubPage })));

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

/** 旧路由重定向到 /chat，通过 location.state.tab 传递目标功能 Tab。 */
function redirectToChat(tab: string) {
  return <Navigate to={BUILTIN_PAGE_PATH.chat} replace state={{ tab }} />;
}

/** 重定向到 /chat 并保留当前 URL 的查询参数（如 template=xxx）。
 * 用于 /workflow/new?template=xxx 等需要透传查询参数的场景。
 * 必须是真正的组件，因为 useLocation 只能在组件渲染时调用，
 * 而路由的 element 属性在路由定义时就被求值。 */
function RedirectToChatWithParams({ tab }: { tab: string }) {
  const location = useLocation();
  const qs = location.search;
  const to = qs ? `${BUILTIN_PAGE_PATH.chat}${qs}` : BUILTIN_PAGE_PATH.chat;
  return <Navigate to={to} replace state={{ tab }} />;
}

/** 旧股票业务路由重定向到 /finance/investment?tab=xxx */
function redirectToFinanceInvestment(
  tab: string,
  extra?: { stockCode?: string; view?: string },
) {
  const params = new URLSearchParams({ tab });
  if (extra?.stockCode) { params.set("stockCode", extra.stockCode); }
  if (extra?.view) { params.set("view", extra.view); }
  return <Navigate to={`${BUILTIN_PAGE_PATH.financeInvestment}?${params.toString()}`} replace />;
}

/** 旧路径 /invest/:stockCode → /finance/investment?tab=workspace&stockCode=xxx */
function RedirectStockWorkspace() {
  const { stockCode } = useParams<{ stockCode?: string }>();
  return redirectToFinanceInvestment("workspace", { stockCode });
}

/** 旧路径 /stock-analysis/:id → /finance/investment?tab=workspace&view=analysis&analysisId=xxx */
function RedirectStockAnalysisById() {
  const { id } = useParams<{ id: string }>();
  const params = new URLSearchParams({ tab: "workspace", view: "analysis" });
  if (id) { params.set("analysisId", id); }
  return <Navigate to={`${BUILTIN_PAGE_PATH.financeInvestment}?${params.toString()}`} replace />;
}

/** 旧 /invest 路径重定向到 /finance/investment */
function RedirectInvestPath() {
  return <Navigate to={BUILTIN_PAGE_PATH.financeInvestment} replace />;
}

/** 旧 /opc 路径重定向到 /automation/operations */
function RedirectOpcPath() {
  const { tab } = useParams<{ tab?: string }>();
  if (tab) {
    return <Navigate to={`${BUILTIN_PAGE_PATH.automationOperations}/${tab}`} replace />;
  }
  return <Navigate to={BUILTIN_PAGE_PATH.automationOperations} replace />;
}

/** 旧行业路径重定向到域化路径 */
function RedirectIndustryPath({ to }: { to: string }) {
  return <Navigate to={to} replace />;
}

/** 路径→页面组件映射（域化路径 → 行业页面组件） */
const INDUSTRY_PAGE_MAP: Record<string, React.LazyExoticComponent<React.ComponentType>> = {
  // 金融域
  [BUILTIN_PAGE_PATH.financeAnalysis]: LazyFinanceInvestPage,
  [BUILTIN_PAGE_PATH.financeAccounting]: LazyAccountingPage,
  // 自动化域
  [BUILTIN_PAGE_PATH.automationSales]: LazySalesGrowthPage,
  [BUILTIN_PAGE_PATH.automationProjects2]: LazyProjectManagementPage,
  [BUILTIN_PAGE_PATH.automationConsulting]: LazyIndustryConsultingPage,
  [BUILTIN_PAGE_PATH.automationEcommerce]: LazyEcommercePage,
  // 运维域
  [BUILTIN_PAGE_PATH.devopsSoftware]: LazySoftwareDevPage,
  [BUILTIN_PAGE_PATH.devopsSecurity]: LazySecurityPage,
  // 数据分析域
  [BUILTIN_PAGE_PATH.dataGeospatial]: LazyGeospatialPage,
  [BUILTIN_PAGE_PATH.dataAiResearch]: LazyAiResearchPage,
  // 内容创作域
  [BUILTIN_PAGE_PATH.contentMedia]: LazyContentMediaPage,
  [BUILTIN_PAGE_PATH.contentDesign]: LazyDesignPage,
  [BUILTIN_PAGE_PATH.contentEducation]: LazyEducationPage,
  // AI 媒体域
  [BUILTIN_PAGE_PATH.aiMediaGame]: LazyGameDevPage,
};

/** 根据域化路径渲染对应的行业页面组件 */
function renderIndustryPage(path: string) {
  const Page = INDUSTRY_PAGE_MAP[path] || LazyIndustryPage;
  return (
    <PageContextProvider page="opc">
      <SafeLazyPage Page={Page} />
    </PageContextProvider>
  );
}

/** 旧行业 ID → 新域化路径映射 */
const INDUSTRY_ID_TO_PATH: Record<string, string> = {
  "finance-invest": BUILTIN_PAGE_PATH.financeAnalysis,
  accounting: BUILTIN_PAGE_PATH.financeAccounting,
  "sales-growth": BUILTIN_PAGE_PATH.automationSales,
  "project-management": BUILTIN_PAGE_PATH.automationProjects2,
  "industry-consulting": BUILTIN_PAGE_PATH.automationConsulting,
  ecommerce: BUILTIN_PAGE_PATH.automationEcommerce,
  "software-dev": BUILTIN_PAGE_PATH.devopsSoftware,
  security: BUILTIN_PAGE_PATH.devopsSecurity,
  geospatial: BUILTIN_PAGE_PATH.dataGeospatial,
  "ai-research": BUILTIN_PAGE_PATH.dataAiResearch,
  "content-media": BUILTIN_PAGE_PATH.contentMedia,
  design: BUILTIN_PAGE_PATH.contentDesign,
  education: BUILTIN_PAGE_PATH.contentEducation,
  "game-dev": BUILTIN_PAGE_PATH.aiMediaGame,
};

/** 旧 /opc/industry/:id 动态路由 → 重定向到域化路径 */
function RedirectIndustryById() {
  const { id } = useParams<{ id?: string }>();
  if (!id) {
    return <Navigate to={BUILTIN_PAGE_PATH.automationOperations} replace />;
  }
  const newPath = INDUSTRY_ID_TO_PATH[id] || BUILTIN_PAGE_PATH.automationOperations;
  return <Navigate to={newPath} replace />;
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

          {/* ── 能力域聚合入口（8 个业务域） ── */}
          {CAPABILITY_DOMAIN_META.map((domain) => (
            <Route
              key={domain.id}
              path={domain.path}
              element={
                <PageContextProvider page={domain.id}>
                  <SafeLazyPage Page={LazyDomainHubPage} />
                </PageContextProvider>
              }
            />
          ))}

          {/* ── 通用功能 ── */}
          <Route
            path={BUILTIN_PAGE_PATH.chat}
            element={
              <PageContextProvider page="chat">
                <SafeLazyPage Page={LazyWorkspaceHub} />
              </PageContextProvider>
            }
          />
          <Route path={BUILTIN_PAGE_PATH.dashboard} element={redirectToChat("dashboard")} />
          <Route path={BUILTIN_PAGE_PATH.workflow} element={redirectToChat("workflow")} />
          <Route path={`${BUILTIN_PAGE_PATH.workflow}/new`} element={<RedirectToChatWithParams tab="workflow" />} />
          <Route path={BUILTIN_PAGE_PATH.terminal} element={redirectToChat("terminal")} />
          <Route path={BUILTIN_PAGE_PATH.files} element={redirectToChat("files")} />
          <Route path={BUILTIN_PAGE_PATH.knowledge} element={redirectToChat("knowledge")} />
          <Route path={BUILTIN_PAGE_PATH.multiAgent} element={redirectToChat("multiAgent")} />
          <Route path={BUILTIN_PAGE_PATH.marketplace} element={redirectToChat("workflow")} />
          <Route
            path={BUILTIN_PAGE_PATH.memory}
            element={
              <PageContextProvider page="memory">
                <SafeLazyPage Page={LazyMemoryPage} />
              </PageContextProvider>
            }
          />
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
          <Route path={BUILTIN_PAGE_PATH.llmWiki} element={redirectToChat("knowledge")} />
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
            path={`${BUILTIN_PAGE_PATH["dynamic-ui"]}/:schemaId`}
            element={
              <PageContextProvider page="dynamic-ui">
                <SafeLazyPage Page={LazyDynamicPageViewer} />
              </PageContextProvider>
            }
          />
          <Route
            path={BUILTIN_PAGE_PATH["dynamic-ui"]}
            element={<Navigate to={BUILTIN_PAGE_PATH.settings} replace />}
          />

          {/* ── 金融域（finance） ── */}
          {/* 股票业务统一入口 */}
          <Route
            path={BUILTIN_PAGE_PATH.financeInvestment}
            element={
              <PageContextProvider page="invest">
                <SafeLazyPage Page={LazyInvestPage} />
              </PageContextProvider>
            }
          />
          {/* 行业页面 */}
          <Route
            path={BUILTIN_PAGE_PATH.financeAnalysis}
            element={renderIndustryPage(BUILTIN_PAGE_PATH.financeAnalysis)}
          />
          <Route
            path={BUILTIN_PAGE_PATH.financeAccounting}
            element={renderIndustryPage(BUILTIN_PAGE_PATH.financeAccounting)}
          />

          {/* ── 自动化域（automation） ── */}
          {/* OPC 管理根路径重定向到仪表板 */}
          <Route
            path={BUILTIN_PAGE_PATH.automationOperations}
            element={<Navigate to={BUILTIN_PAGE_PATH.automationDashboard} replace />}
          />
          {/* OPC 子页面（仪表板、发票、客户、项目等） */}
          <Route
            path={`${BUILTIN_PAGE_PATH.automationOperations}/:tab`}
            element={
              <PageContextProvider page="opc">
                <SafeLazyPage Page={LazyOpcSubPage} />
              </PageContextProvider>
            }
          />
          {/* 行业页面 */}
          <Route
            path={BUILTIN_PAGE_PATH.automationSales}
            element={renderIndustryPage(BUILTIN_PAGE_PATH.automationSales)}
          />
          <Route
            path={BUILTIN_PAGE_PATH.automationProjects2}
            element={renderIndustryPage(BUILTIN_PAGE_PATH.automationProjects2)}
          />
          <Route
            path={BUILTIN_PAGE_PATH.automationConsulting}
            element={renderIndustryPage(BUILTIN_PAGE_PATH.automationConsulting)}
          />
          <Route
            path={BUILTIN_PAGE_PATH.automationEcommerce}
            element={renderIndustryPage(BUILTIN_PAGE_PATH.automationEcommerce)}
          />

          {/* ── 运维域（devops）行业页面 ── */}
          <Route
            path={BUILTIN_PAGE_PATH.devopsSoftware}
            element={renderIndustryPage(BUILTIN_PAGE_PATH.devopsSoftware)}
          />
          <Route
            path={BUILTIN_PAGE_PATH.devopsSecurity}
            element={renderIndustryPage(BUILTIN_PAGE_PATH.devopsSecurity)}
          />

          {/* ── 数据分析域（data_analysis）行业页面 ── */}
          <Route
            path={BUILTIN_PAGE_PATH.dataGeospatial}
            element={renderIndustryPage(BUILTIN_PAGE_PATH.dataGeospatial)}
          />
          <Route
            path={BUILTIN_PAGE_PATH.dataAiResearch}
            element={renderIndustryPage(BUILTIN_PAGE_PATH.dataAiResearch)}
          />

          {/* ── 内容创作域（content_creation）行业页面 ── */}
          <Route
            path={BUILTIN_PAGE_PATH.contentMedia}
            element={renderIndustryPage(BUILTIN_PAGE_PATH.contentMedia)}
          />
          <Route
            path={BUILTIN_PAGE_PATH.contentDesign}
            element={renderIndustryPage(BUILTIN_PAGE_PATH.contentDesign)}
          />
          <Route
            path={BUILTIN_PAGE_PATH.contentEducation}
            element={renderIndustryPage(BUILTIN_PAGE_PATH.contentEducation)}
          />

          {/* ── AI 媒体域（ai_media）行业页面 ── */}
          <Route
            path={BUILTIN_PAGE_PATH.aiMediaGame}
            element={renderIndustryPage(BUILTIN_PAGE_PATH.aiMediaGame)}
          />

          {/* ── 旧路径重定向（兼容书签和外链） ── */}
          {/* 旧 /invest → /finance/investment */}
          <Route path={BUILTIN_PAGE_PATH.invest} element={<RedirectInvestPath />} />
          <Route path={`${BUILTIN_PAGE_PATH.invest}/:stockCode`} element={<RedirectStockWorkspace />} />
          <Route
            path={BUILTIN_PAGE_PATH["stock-analysis"]}
            element={redirectToFinanceInvestment("workspace", { view: "analysis" })}
          />
          <Route
            path={`${BUILTIN_PAGE_PATH["stock-analysis"]}/:id`}
            element={<RedirectStockAnalysisById />}
          />
          <Route
            path={`${BUILTIN_PAGE_PATH["stock-analysis"]}/*`}
            element={redirectToFinanceInvestment("workspace", { view: "analysis" })}
          />
          <Route path={BUILTIN_PAGE_PATH.screener} element={redirectToFinanceInvestment("screener")} />
          <Route
            path={BUILTIN_PAGE_PATH.watchlist}
            element={redirectToFinanceInvestment("workspace", { view: "monitor" })}
          />
          <Route
            path={BUILTIN_PAGE_PATH.portfolio}
            element={redirectToFinanceInvestment("workspace", { view: "monitor" })}
          />
          <Route path={BUILTIN_PAGE_PATH["paper-portfolio"]} element={redirectToFinanceInvestment("paper-portfolio")} />
          <Route path={BUILTIN_PAGE_PATH["market-mainline"]} element={redirectToFinanceInvestment("market-mainline")} />
          <Route
            path={BUILTIN_PAGE_PATH["screenshot-diagnosis"]}
            element={redirectToFinanceInvestment("screenshot-diagnosis")}
          />
          <Route path={BUILTIN_PAGE_PATH.trade} element={redirectToFinanceInvestment("workspace", { view: "trade" })} />
          <Route
            path={BUILTIN_PAGE_PATH.backtest}
            element={redirectToFinanceInvestment("workspace", { view: "backtest" })}
          />
          <Route
            path={BUILTIN_PAGE_PATH.compare}
            element={redirectToFinanceInvestment("workspace", { view: "compare" })}
          />
          <Route
            path={BUILTIN_PAGE_PATH["scheduled-analysis"]}
            element={redirectToFinanceInvestment("market-mainline")}
          />
          <Route path={BUILTIN_PAGE_PATH.quant} element={redirectToFinanceInvestment("quant")} />
          <Route
            path={BUILTIN_PAGE_PATH["replay-workbench"]}
            element={redirectToFinanceInvestment("market-mainline")}
          />
          <Route path={BUILTIN_PAGE_PATH.pipeline} element={redirectToFinanceInvestment("pipeline")} />

          {/* 旧 /workspace → /finance/investment?tab=workspace */}
          <Route path={BUILTIN_PAGE_PATH.workspace} element={redirectToFinanceInvestment("workspace")} />
          <Route path={`${BUILTIN_PAGE_PATH.workspace}/:stockCode`} element={<RedirectStockWorkspace />} />

          {/* 旧 /opc → /automation/operations */}
          <Route path={BUILTIN_PAGE_PATH.opc} element={<RedirectOpcPath />} />
          <Route path={`${BUILTIN_PAGE_PATH.opc}/:tab`} element={<RedirectOpcPath />} />

          {/* 旧 OPC 行业路径 → 重定向到域化路径 */}
          <Route
            path={BUILTIN_PAGE_PATH.opcIndustryAiResearch}
            element={<RedirectIndustryPath to={BUILTIN_PAGE_PATH.dataAiResearch} />}
          />
          <Route
            path={BUILTIN_PAGE_PATH.opcIndustrySoftwareDev}
            element={<RedirectIndustryPath to={BUILTIN_PAGE_PATH.devopsSoftware} />}
          />
          <Route
            path={BUILTIN_PAGE_PATH.opcIndustryFinanceInvest}
            element={<RedirectIndustryPath to={BUILTIN_PAGE_PATH.financeAnalysis} />}
          />
          <Route
            path={BUILTIN_PAGE_PATH.opcIndustrySalesGrowth}
            element={<RedirectIndustryPath to={BUILTIN_PAGE_PATH.automationSales} />}
          />
          <Route
            path={BUILTIN_PAGE_PATH.opcIndustryContentMedia}
            element={<RedirectIndustryPath to={BUILTIN_PAGE_PATH.contentMedia} />}
          />
          <Route
            path={BUILTIN_PAGE_PATH.opcIndustryIndustryConsulting}
            element={<RedirectIndustryPath to={BUILTIN_PAGE_PATH.automationConsulting} />}
          />
          <Route
            path={BUILTIN_PAGE_PATH.opcIndustryAccounting}
            element={<RedirectIndustryPath to={BUILTIN_PAGE_PATH.financeAccounting} />}
          />
          <Route
            path={BUILTIN_PAGE_PATH.opcIndustryEcommerce}
            element={<RedirectIndustryPath to={BUILTIN_PAGE_PATH.automationEcommerce} />}
          />
          <Route
            path={BUILTIN_PAGE_PATH.opcIndustryEducation}
            element={<RedirectIndustryPath to={BUILTIN_PAGE_PATH.contentEducation} />}
          />
          <Route
            path={BUILTIN_PAGE_PATH.opcIndustryDesign}
            element={<RedirectIndustryPath to={BUILTIN_PAGE_PATH.contentDesign} />}
          />
          <Route
            path={BUILTIN_PAGE_PATH.opcIndustryProjectManagement}
            element={<RedirectIndustryPath to={BUILTIN_PAGE_PATH.automationProjects2} />}
          />
          <Route
            path={BUILTIN_PAGE_PATH.opcIndustrySecurity}
            element={<RedirectIndustryPath to={BUILTIN_PAGE_PATH.devopsSecurity} />}
          />
          <Route
            path={BUILTIN_PAGE_PATH.opcIndustryGeospatial}
            element={<RedirectIndustryPath to={BUILTIN_PAGE_PATH.dataGeospatial} />}
          />
          <Route
            path={BUILTIN_PAGE_PATH.opcIndustryGameDev}
            element={<RedirectIndustryPath to={BUILTIN_PAGE_PATH.aiMediaGame} />}
          />

          {/* 旧动态路由 → 重定向到域化路径 */}
          <Route
            path={`${BUILTIN_PAGE_PATH.opcIndustryDynamic}/:id`}
            element={<RedirectIndustryById />}
          />

          {/* 旧 OPC 行业导航页 → 重定向到自动化域 */}
          <Route
            path={BUILTIN_PAGE_PATH.opcIndustries}
            element={
              <PageContextProvider page="opc">
                <SafeLazyPage Page={LazyIndustryNavigatorPage} />
              </PageContextProvider>
            }
          />

          {/* 开发工具旧路由 → 重定向到 /chat */}
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
