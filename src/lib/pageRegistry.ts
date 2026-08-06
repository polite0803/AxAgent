// SPDX-License-Identifier: AGPL-3.0-only

/**
 * Unified page registry — 内置路由路径的单一真相源（single source of truth）。
 *
 * 所有内置页面的路径在此集中声明。以下位置必须从此处 import，禁止散写硬编码：
 *   - ContentArea 的 <Route path=...> 字面量
 *   - Sidebar 的 builtinNavItems / pathToPageKey
 *   - usePageRouting 的 path→key 映射
 *
 * 新增内置页面时：
 *   1. 在 BUILTIN_PAGE_PATH 增加 key→path；
 *   2. 在 ContentArea 增加对应 <Route path={BUILTIN_PAGE_PATH.xxx}> 与懒加载组件；
 *   3. 在 Sidebar builtinNavItems 增加导航项（path 引用本表）。
 */

/** 应用冷启动后的默认首页路径（"/" 重定向目标）。
 *  仪表盘已合并到对话页的「工作台」Tab，默认进入对话页。 */
export const DEFAULT_HOME = "/chat";

/**
 * key→path 映射，覆盖所有内置页面（含未进入导航栏的 link/marketplace 等）。
 */
export const BUILTIN_PAGE_PATH: Record<string, string> = {
  chat: "/chat",
  dashboard: "/dashboard",
  knowledge: "/knowledge",
  memory: "/memory",
  link: "/link",
  gateway: "/gateway",
  settings: "/settings",
  workflow: "/workflow",
  files: "/files",
  terminal: "/terminal",
  "dynamic-ui": "/dynamic-ui",
  marketplace: "/marketplace",
  wiki: "/wiki",
  // AxInvest 股票业务统一入口（7 个业务页面合并为 Tab）
  invest: "/invest",
  // 以下为 invest 页面子 tab 的历史独立路由（已重定向到 /invest?tab=xxx）
  workspace: "/workspace",
  "stock-analysis": "/stock-analysis",
  screener: "/screener",
  watchlist: "/watchlist",
  portfolio: "/portfolio",
  "paper-portfolio": "/paper-portfolio",
  "market-mainline": "/market-mainline",
  "screenshot-diagnosis": "/screenshot-diagnosis",
  "trade": "/trade",
  backtest: "/backtest",
  compare: "/compare",
  "scheduled-analysis": "/scheduled-analysis",
  quant: "/quant",
  "replay-workbench": "/replay-workbench",
  pipeline: "/pipeline",
  // G1 跨市场数据接入
  "cross-market": "/cross-market",
  // G5 Multi-Agent 固定角色 pool
  multiAgent: "/multi-agent",
  // OPC 一人公司管理
  opc: "/opc",
  // OPC 管理页面（仪表板、发票、客户、项目等）
  opcDashboard: "/opc/dashboard",
  opcInvoices: "/opc/invoices",
  opcCustomers: "/opc/customers",
  opcProjects: "/opc/projects",
  opcSites: "/opc/sites",
  opcTalent: "/opc/talent",
  opcMarket: "/opc/market",
  opcKanban: "/opc/kanban",
  // OPC 9 大垂直行业入口
  opcIndustryAiResearch: "/opc/industries/ai-research",
  opcIndustrySoftwareDev: "/opc/industries/software-dev",
  opcIndustryFinanceInvest: "/opc/industries/finance-invest",
  opcIndustrySalesGrowth: "/opc/industries/sales-growth",
  opcIndustryContentMedia: "/opc/industries/content-media",
  opcIndustryIndustryConsulting: "/opc/industries/industry-consulting",
  opcIndustryAccounting: "/opc/industries/accounting",
  opcIndustryEcommerce: "/opc/industries/ecommerce",
  opcIndustryEducation: "/opc/industries/education",
  // 以下为历史兼容入口 / devtools 等次要路由，同样收归此处以消除散写硬编码
  llmWiki: "/llm-wiki",
  learningGraph: "/learning-graph",
  quickbar: "/quickbar",
  // 开发者工具已并入对话页「开发工具」Tab（/chat + state.tab），以下路径保留作旧路由重定向
  devtools: "/devtools",
  devtoolsTraceExplorer: "/devtools/trace-explorer",
  devtoolsBenchmark: "/devtools/benchmark",
  devtoolsToolRecommender: "/devtools/tool-recommender",
  devtoolsFineTune: "/devtools/fine-tune",
  devtoolsRlTraining: "/devtools/rl-training",
};
