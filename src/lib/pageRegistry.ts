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
 *   - 业务域路径由 CAPABILITY_DOMAIN_META 自动展开（/finance, /automation 等）
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
