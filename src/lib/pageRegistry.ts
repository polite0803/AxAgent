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
  multiAgent: "/multi-agent",
  // 以下为历史兼容入口 / devtools 等次要路由，同样收归此处以消除散写硬编码
  llmWiki: "/llm-wiki",
  learningGraph: "/learning-graph",
  quickbar: "/quickbar",
  // 开发者工具统一入口（5 个子项合并为 1 项，内部 Tab 切换）
  devtools: "/devtools",
  devtoolsTraceExplorer: "/devtools/trace-explorer",
  devtoolsBenchmark: "/devtools/benchmark",
  devtoolsToolRecommender: "/devtools/tool-recommender",
  devtoolsFineTune: "/devtools/fine-tune",
  devtoolsRlTraining: "/devtools/rl-training",
};
