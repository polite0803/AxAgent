// SPDX-License-Identifier: AGPL-3.0-only
// i18n-exempt: EvolutionDriftPanel 的 DynamicUI Schema 纯数据定义，非用户可见 UI 文案。

import type { UISchema } from "@/types";

/**
 * EvolutionDriftPanel 的 DynamicUI Schema — 纯数据定义。
 *
 * 三段 UI（均为 UISchema 纯数据，无组件包装）：
 * 1. 策略权重表 → DynamicUI Table
 * 2. 权重时间线 → DynamicUI Chart
 * 3. 策略摘要 → DynamicUI Dashboard
 *
 * ## 用法
 *
 * ```tsx
 * const dashboard = useStockAnalysisStore(s => s.evolutionDashboard);
 * <DynamicUIRenderer
 *   schema={evolutionDriftSchema()}
 *   dataContext={{ data: dashboard }}
 * />
 * ```
 *
 * 数据自动从 DataBindingEngine 绑定的 stock-analysis store 获取。
 */

export function evolutionDriftSchema(): UISchema {
  return {
    version: "1.0",
    id: "evolution-drift-panel",
    type: "Column",
    props: { gap: 12 },

    children: [
      // ── 策略权重表 ──
      {
        version: "1.0",
        id: "weight-table",
        type: "Table",
        props: {
          size: "small",
          pagination: false,
          columns: [
            { key: "strategyId", title: "策略", dataIndex: "strategyId" },
            { key: "period", title: "周期", dataIndex: "period" },
            { key: "newWeight", title: "权重", dataIndex: "newWeight", render: { type: "format", format: "fixed(2)" } },
            {
              key: "deltaPct",
              title: "变化",
              dataIndex: "deltaPct",
              render: { type: "color", positive: "red", negative: "green" },
            },
            { key: "winRate", title: "胜率", dataIndex: "winRate", render: { type: "format", format: "pct" } },
            { key: "sampleSize", title: "样本", dataIndex: "sampleSize" },
          ],
        },
        dataSource: {
          type: "store",
          config: { storeName: "stock-analysis", fields: "evolutionDashboard.stats" },
        },
      },

      // ── 重算按钮 ──
      {
        version: "1.0",
        id: "recalc-button",
        type: "Row",
        props: { gap: 8 },
        children: [
          {
            version: "1.0",
            id: "recalc-btn",
            type: "Button",
            props: { content: "重新计算", type: "primary", size: "small" },
            events: [
              {
                trigger: "onClick",
                actions: [{ type: "invoke", config: { command: "stock_evolution_recalc" } }],
              },
            ],
          },
        ],
      },

      // ── 权重时间线 ──
      {
        version: "1.0",
        id: "timeline",
        type: "Card",
        props: { size: "small", title: "权重时间线" },
        dataSource: {
          type: "store",
          config: { storeName: "stock-analysis", fields: "evolutionDashboard.recentChanges" },
        },
        children: [
          {
            version: "1.0",
            id: "timeline-chart",
            type: "Chart",
            props: { type: "line", xField: "appliedAt", yField: "newWeight", smooth: true, height: 200 },
          },
        ],
      },

      // ── 策略摘要 Dashboard ──
      {
        version: "1.0",
        id: "summary",
        type: "Dashboard",
        dataSource: {
          type: "store",
          config: { storeName: "stock-analysis", fields: "evolutionDashboard.strategySummary" },
        },
        props: {
          cards: [
            { key: "strategyId", title: "策略", valueField: "strategyId" },
            { key: "avgWeight", title: "平均权重", valueField: "avgWeight", format: "fixed(2)" },
            { key: "avgWinRate", title: "平均胜率", valueField: "avgWinRate", format: "pct" },
            { key: "totalSamples", title: "总样本", valueField: "totalSamples" },
          ],
        },
      },
    ],
  };
}
