// SPDX-License-Identifier: AGPL-3.0-only
// i18n-exempt: StockAnalysisPage 的 DynamicUI Schema 纯数据定义，非用户可见 UI 文案。

import type { UISchema } from "@/types";

/**
 * StockAnalysisPage 的 DynamicUI Schema 等价定义。
 *
 * 布局结构：
 *   顶部：DecisionBanner + AnalystConsensusBar + ExperimentTrail（条件渲染）
 *   主区：Tabs 容器（market / analysts / debate / value / risk / reflection / evolution）
 *   侧栏：Accordion 容器（index / sectors / north / events / announcements / ...）
 *
 * 数据源：stock-analysis store
 * 条件渲染分支：status === "idle" | "loading" | "running" | "completed" | "error"
 *
 * 现有实现参考：src/components/stock-analysis/StockAnalysisPage.tsx
 */
export function stockAnalysisPageSchema(): UISchema {
  return {
    version: "1.0",
    id: "stock-analysis-page",
    type: "Column",
    props: { gap: 0 },
    children: [
      // ── Header ──
      {
        version: "1.0",
        id: "page-header",
        type: "Row",
        props: { align: "center", gap: 8 },
        children: [
          {
            version: "1.0",
            id: "back-button",
            type: "Button",
            props: { content: "← 返回", type: "text", size: "small" },
            events: [
              {
                trigger: "onClick",
                actions: [{ type: "navigate", config: { path: "/" } }],
              },
            ],
          },
          {
            version: "1.0",
            id: "page-title",
            type: "Text",
            props: { content: "股票分析", strong: true },
          },
        ],
      },
      // ── Search + Action bar ──
      {
        version: "1.0",
        id: "search-action-bar",
        type: "Row",
        props: { gap: 8, wrap: true },
        children: [
          {
            version: "1.0",
            id: "stock-search",
            type: "Input",
            props: {
              name: "stockCode",
              placeholder: "搜索股票代码或名称",
              allowClear: true,
            },
            events: [
              {
                trigger: "onSubmit",
                actions: [
                  {
                    type: "invoke",
                    config: {
                      command: "start_analysis",
                      params: { code: "$stockCode" },
                    },
                  },
                ],
              },
            ],
          },
        ],
      },
      // ── Status: idle → 仪表盘 ──
      {
        version: "1.0",
        id: "idle-dashboard",
        type: "Card",
        conditionalDisplay: [
          { operator: "eq", field: "status", value: "idle" },
        ],
        props: { size: "small" },
        children: [
          {
            version: "1.0",
            id: "invest-dashboard-text",
            type: "Text",
            props: { content: "输入股票代码开始分析", type: "secondary" },
          },
        ],
      },
      // ── Status: loading → Progress ──
      {
        version: "1.0",
        id: "loading-state",
        type: "Card",
        conditionalDisplay: [
          { operator: "eq", field: "status", value: "loading" },
        ],
        props: { size: "small" },
        children: [
          {
            version: "1.0",
            id: "progress-bar",
            type: "Progress",
            dataSource: {
              type: "store",
              config: { storeName: "stock-analysis", fields: "progressPct" },
            },
            props: { percent: "{{data}}", status: "active" },
          },
        ],
      },
      // ── Status: running/completed → 主内容 ──
      {
        version: "1.0",
        id: "main-content",
        type: "Column",
        props: { gap: 12 },
        conditionalDisplay: [
          { operator: "in", field: "status", value: ["running", "completed"] },
        ],
        children: [
          // 行情卡片（StockQuoteCard schema）
          {
            version: "1.0",
            id: "quote-section",
            type: "Card",
            dataSource: {
              type: "store",
              config: { storeName: "stock-analysis", fields: "quote" },
            },
            props: { size: "small" },
            children: [
              {
                version: "1.0",
                id: "quote-price-row",
                type: "Row",
                props: { gap: 12, align: "baseline" },
                children: [
                  {
                    version: "1.0",
                    id: "price",
                    type: "Text",
                    props: {
                      content: "{{data.price | fixed(2)}}",
                      strong: true,
                    },
                  },
                  {
                    version: "1.0",
                    id: "change-pct",
                    type: "Tag",
                    props: {
                      content: "{{data.changePct | fixed(2)}}%",
                      color: "{{data.changePct > 0 ? 'red' : data.changePct < 0 ? 'green' : 'default'}}",
                    },
                  },
                ],
              },
            ],
          },
          // Tab 容器
          {
            version: "1.0",
            id: "main-tabs",
            type: "Tabs",
            props: {
              defaultActiveKey: "market",
              tabPosition: "top",
              size: "small",
            },
            children: [
              {
                version: "1.0",
                id: "tab-market",
                type: "Card",
                props: { tabKey: "market", tabLabel: "行情", size: "small" },
              },
              {
                version: "1.0",
                id: "tab-analysts",
                type: "Card",
                props: {
                  tabKey: "analysts",
                  tabLabel: "分析师",
                  size: "small",
                },
              },
              {
                version: "1.0",
                id: "tab-debate",
                type: "Card",
                props: { tabKey: "debate", tabLabel: "辩论", size: "small" },
              },
              {
                version: "1.0",
                id: "tab-value",
                type: "Card",
                props: {
                  tabKey: "value",
                  tabLabel: "估值",
                  size: "small",
                },
              },
              {
                version: "1.0",
                id: "tab-risk",
                type: "Card",
                props: { tabKey: "risk", tabLabel: "风险", size: "small" },
              },
              {
                version: "1.0",
                id: "tab-decision",
                type: "Card",
                props: {
                  tabKey: "decision",
                  tabLabel: "决策",
                  size: "small",
                },
              },
              {
                version: "1.0",
                id: "tab-reflection",
                type: "Card",
                props: {
                  tabKey: "reflection",
                  tabLabel: "反思",
                  size: "small",
                },
              },
              {
                version: "1.0",
                id: "tab-evolution",
                type: "Card",
                props: {
                  tabKey: "evolution",
                  tabLabel: "进化",
                  size: "small",
                },
              },
            ],
          },
        ],
      },
      // ── Sidebar: 侧栏面板（Accordion / Sheet Panel）──
      {
        version: "1.0",
        id: "sidebar-panels",
        type: "Accordion",
        props: {
          ghost: true,
          defaultActiveKey: ["index"],
        },
        children: [
          {
            version: "1.0",
            id: "panel-index",
            type: "Card",
            props: {
              header: "指数行情",
              size: "small",
              collapsible: "header",
            },
          },
          {
            version: "1.0",
            id: "panel-sectors",
            type: "Card",
            props: {
              header: "板块热力图",
              size: "small",
              collapsible: "header",
            },
          },
          {
            version: "1.0",
            id: "panel-north",
            type: "Card",
            props: {
              header: "北向资金",
              size: "small",
              collapsible: "header",
            },
          },
          {
            version: "1.0",
            id: "panel-events",
            type: "Card",
            props: {
              header: "事件日历",
              size: "small",
              collapsible: "header",
            },
          },
          {
            version: "1.0",
            id: "panel-announcements",
            type: "Card",
            props: {
              header: "公告",
              size: "small",
              collapsible: "header",
            },
          },
          {
            version: "1.0",
            id: "panel-concepts",
            type: "Card",
            props: {
              header: "概念板块",
              size: "small",
              collapsible: "header",
            },
          },
          {
            version: "1.0",
            id: "panel-option-pcr",
            type: "Card",
            props: {
              header: "期权PCR",
              size: "small",
              collapsible: "header",
            },
          },
        ],
      },
      // ── Error state ──
      {
        version: "1.0",
        id: "error-state",
        type: "Card",
        conditionalDisplay: [
          { operator: "eq", field: "status", value: "error" },
        ],
        props: { size: "small" },
        children: [
          {
            version: "1.0",
            id: "error-message",
            type: "Text",
            dataSource: {
              type: "store",
              config: { storeName: "stock-analysis", fields: "error" },
            },
            props: { content: "{{data}}", type: "danger" },
          },
          {
            version: "1.0",
            id: "retry-button",
            type: "Button",
            props: { content: "重试", type: "primary" },
            events: [
              {
                trigger: "onClick",
                actions: [
                  {
                    type: "invoke",
                    config: { command: "start_analysis" },
                  },
                ],
              },
            ],
          },
        ],
      },
    ],
  };
}
