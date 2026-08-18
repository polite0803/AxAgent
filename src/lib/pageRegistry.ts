// SPDX-License-Identifier: AGPL-3.0-only

import { CAPABILITY_DOMAIN_META } from "@/lib/domainMeta";

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
 * key→path 映射，覆盖所有内置页面。
 *
 * 路径按 8 个标准能力域组织：
 *   - 通用功能路径保持顶级（/chat, /terminal, /files, /gateway 等）
 *   - 业务路径挂在对应域下（/finance/investment, /automation/operations 等）
 *   - 旧路径保留作重定向（/invest → /finance/investment, /opc → /automation/operations 等）
 */
export const BUILTIN_PAGE_PATH: Record<string, string> = {
  // ── 能力域聚合入口路径（8 个业务域，路径来源 domainMeta 单一真相源） ──
  ...Object.fromEntries(CAPABILITY_DOMAIN_META.map((d) => [d.id, d.path])),

  // ── 通用功能（general 域） ──
  chat: "/chat",
  dashboard: "/dashboard",
  knowledge: "/knowledge",
  memory: "/memory",
  link: "/link",
  settings: "/settings",
  workflow: "/workflow",
  "dynamic-ui": "/dynamic-ui",
  marketplace: "/marketplace",
  wiki: "/wiki",
  multiAgent: "/multi-agent",

  // ── 运维域（devops）：路径保持顶级（/terminal, /files, /gateway） ──
  terminal: "/terminal",
  files: "/files",
  gateway: "/gateway",

  // ── 金融域（finance） ──
  // 股票业务统一入口（原 /invest，路径改为 /finance/investment）
  financeInvestment: "/finance/investment",
  // 行业页面
  financeAnalysis: "/finance/analysis",
  financeAccounting: "/finance/accounting",

  // ── 自动化域（automation） ──
  // OPC 一人公司管理（原 /opc，路径改为 /automation/operations）
  automationOperations: "/automation/operations",
  // OPC 管理子页面（仪表板、发票、客户、项目等）
  automationDashboard: "/automation/operations/dashboard",
  automationInvoices: "/automation/operations/invoices",
  automationCustomers: "/automation/operations/customers",
  automationProjects: "/automation/operations/projects",
  automationSites: "/automation/operations/sites",
  automationTalent: "/automation/operations/talent",
  automationMarket: "/automation/operations/market",
  automationKanban: "/automation/operations/kanban",
  // 行业页面
  automationSales: "/automation/sales",
  automationProjects2: "/automation/projects",
  automationConsulting: "/automation/consulting",
  automationEcommerce: "/automation/ecommerce",

  // ── 运维域行业页面 ──
  devopsSoftware: "/devops/software",
  devopsSecurity: "/devops/security",

  // ── 数据分析域（data_analysis） ──
  dataGeospatial: "/data-analysis/geospatial",
  dataAiResearch: "/data-analysis/ai-research",

  // ── 内容创作域（content_creation） ──
  contentMedia: "/content-creation/media",
  contentDesign: "/content-creation/design",
  contentEducation: "/content-creation/education",

  // ── AI 媒体域（ai_media） ──
  aiMediaGame: "/ai-media/game",

  // ── 通信域（communication） ──
  communicationMessage: "/communication/message",

  // ── 以下为旧路径（保留作重定向，兼容书签和外链） ──
  // 旧股票业务路径 → 重定向到 /finance/investment
  invest: "/invest",
  workspace: "/workspace",
  "stock-analysis": "/stock-analysis",
  screener: "/screener",
  watchlist: "/watchlist",
  portfolio: "/portfolio",
  "paper-portfolio": "/paper-portfolio",
  "market-mainline": "/market-mainline",
  "screenshot-diagnosis": "/screenshot-diagnosis",
  trade: "/trade",
  backtest: "/backtest",
  compare: "/compare",
  "scheduled-analysis": "/scheduled-analysis",
  quant: "/quant",
  "replay-workbench": "/replay-workbench",
  pipeline: "/pipeline",
  "cross-market": "/cross-market",

  // 旧 OPC 路径 → 重定向到 /automation/operations
  opc: "/opc",
  opcDashboard: "/opc/dashboard",
  opcInvoices: "/opc/invoices",
  opcCustomers: "/opc/customers",
  opcProjects: "/opc/projects",
  opcSites: "/opc/sites",
  opcTalent: "/opc/talent",
  opcMarket: "/opc/market",
  opcKanban: "/opc/kanban",

  // 旧 OPC 行业路径 → 重定向到对应的域化路径
  opcIndustryAiResearch: "/opc/industries/ai-research",
  opcIndustrySoftwareDev: "/opc/industries/software-dev",
  opcIndustryFinanceInvest: "/opc/industries/finance-invest",
  opcIndustrySalesGrowth: "/opc/industries/sales-growth",
  opcIndustryContentMedia: "/opc/industries/content-media",
  opcIndustryIndustryConsulting: "/opc/industries/industry-consulting",
  opcIndustryAccounting: "/opc/industries/accounting",
  opcIndustryEcommerce: "/opc/industries/ecommerce",
  opcIndustryEducation: "/opc/industries/education",
  opcIndustryDesign: "/opc/industries/design",
  opcIndustryProjectManagement: "/opc/industries/project-management",
  opcIndustrySecurity: "/opc/industries/security",
  opcIndustryGeospatial: "/opc/industries/geospatial",
  opcIndustryGameDev: "/opc/industries/game-dev",
  // 旧动态路由和导航页
  opcIndustryDynamic: "/opc/industry",
  opcIndustries: "/opc/industries",

  // ── 历史兼容入口 / devtools 等 ──
  llmWiki: "/llm-wiki",
  learningGraph: "/learning-graph",
  quickbar: "/quickbar",
  devtools: "/devtools",
  devtoolsTraceExplorer: "/devtools/trace-explorer",
  devtoolsBenchmark: "/devtools/benchmark",
  devtoolsToolRecommender: "/devtools/tool-recommender",
  devtoolsFineTune: "/devtools/fine-tune",
  devtoolsRlTraining: "/devtools/rl-training",
};
