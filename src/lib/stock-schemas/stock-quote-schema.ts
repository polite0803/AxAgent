// SPDX-License-Identifier: AGPL-3.0-only
// i18n-exempt: StockQuoteCard 的 DynamicUI Schema 纯数据定义，非用户可见 UI 文案。

import type { UISchema } from "@/types";
import type { StockQuote } from "@/types/stock-analysis";

/**
 * StockQuoteCard 的 DynamicUI Schema — 纯数据定义。
 *
 * 不包含任何 React 组件代码，返回一个 UISchema JSON 对象，
 * 可被 DynamicUIRenderer 直接渲染。
 *
 * ## 用法
 *
 * ```tsx
 * // 方式 A — 通过 dataContext 传入数据（页面自有 store 时推荐）
 * const quote = useStockAnalysisStore(s => s.quote);
 * <DynamicUIRenderer schema={stockQuoteSchema()} dataContext={{ data: quote }} />
 *
 * // 方式 B — 通过 dataSource 自动轮询（Schema 自带数据获取）
 * // 此时 dataSource 自动从 store 绑定 quote
 * <DynamicUIRenderer schema={stockQuoteSchema()} />
 * ```
 *
 * ## 关键设计
 *
 * - **纯数据**：返回 UISchema，不是 React 组件
 * - **零业务依赖**：不 import 任何 store / hook / 业务组件
 * - **可组合**：可作为子 Schema 嵌入更大的页面 Schema 中
 * - **可序列化**：JSON.stringify 后可在 API / DB / AI 输出中传递
 */

// ── 数据源引用：从 stock-analysis store 获取 quote ──
const STORE_QUOTE_DS = {
  type: "store" as const,
  config: { storeName: "stock-analysis", fields: "quote" },
  polling: 30000,
};

// ── Schema 定义 —— 接收 StockQuote 数据，渲染成行情卡片 ──
export function stockQuoteSchema(): UISchema {
  return {
    version: "1.0",
    id: "stock-quote-card",

    // Card 容器：标题显示 "名称(代码)"
    type: "Card",
    props: {
      size: "small",
      extra: {
        type: "Button",
        props: {
          content: "○ 自动刷新",
          size: "small",
          type: "text",
        },
      },
    },

    // 数据源：从已注册的 stock-analysis store 获取 quote（30s 轮询）
    dataSource: STORE_QUOTE_DS,

    children: [
      // ── 第一行：当前价 + 涨跌幅 Tag + 涨跌额 ──
      {
        version: "1.0",
        id: "quote-header",
        type: "Row",
        props: { align: "bottom", gap: 16, wrap: true },
        children: [
          // 当前价
          {
            version: "1.0",
            id: "quote-price",
            type: "Text",
            props: {
              content: "{{data.price | fixed(2)}}",
            },
            style: {
              fontSize: 28,
              fontWeight: 600,
              fontFamily: "monospace",
            },
          },
          // 涨跌幅 Tag
          {
            version: "1.0",
            id: "quote-change-pct",
            type: "Tag",
            props: {
              color: "{{data.changePct > 0 ? 'red' : data.changePct < 0 ? 'green' : 'default'}}",
              content: "{{data.changePct > 0 ? '+' : ''}}{{data.changePct | fixed(2)}}%",
            },
          },
          // 涨跌额
          {
            version: "1.0",
            id: "quote-change-amount",
            type: "Text",
            props: {
              content: "{{(data.price - data.preClose) >= 0 ? '+' : ''}}{{data.price - data.preClose | fixed(2)}}",
            },
            style: { fontFamily: "monospace", fontSize: 12 },
          },
        ],
      },

      // ── 第二行：核心数据 4 列网格 ──
      {
        version: "1.0",
        id: "quote-detail-grid",
        type: "Grid",
        props: { columns: 4, gap: [12, 4] },
        style: { color: "var(--muted)", fontSize: 12 },
        children: [
          {
            version: "1.0",
            id: "q-open",
            type: "Text",
            props: { content: `开盘: {{data.open | fixed(2)}}` },
          },
          {
            version: "1.0",
            id: "q-high",
            type: "Text",
            props: { content: `最高: {{data.high | fixed(2)}}` },
          },
          {
            version: "1.0",
            id: "q-low",
            type: "Text",
            props: { content: `最低: {{data.low | fixed(2)}}` },
          },
          {
            version: "1.0",
            id: "q-volume",
            type: "Text",
            props: { content: `成交额: {{(data.amount / 1e8) | fixed(1)}}亿` },
          },
          // PE（条件渲染：仅当 pe ≠ null）
          {
            version: "1.0",
            id: "q-pe",
            type: "Text",
            props: { content: `PE: {{data.pe}}` },
            conditionalDisplay: [{ operator: "exists", field: "data.pe" }],
          },
          // PB
          {
            version: "1.0",
            id: "q-pb",
            type: "Text",
            props: { content: `PB: {{data.pb}}` },
            conditionalDisplay: [{ operator: "exists", field: "data.pb" }],
          },
          // 总市值
          {
            version: "1.0",
            id: "q-mv",
            type: "Text",
            props: { content: `市值: {{(data.totalMv / 1e8) | fixed(1)}}亿` },
            conditionalDisplay: [{ operator: "exists", field: "data.totalMv" }],
          },
          // 换手率
          {
            version: "1.0",
            id: "q-turnover",
            type: "Text",
            props: { content: `换手率: {{data.turnoverRate | fixed(2)}}%` },
            conditionalDisplay: [{ operator: "exists", field: "data.turnoverRate" }],
          },
        ],
      },
    ],
  };
}

// ── 验证：运行时检查 schema 是否与 StockQuote 结构匹配 ──
export function validateQuoteSchema(quote: StockQuote | null): string[] {
  const errors: string[] = [];
  if (!quote) { return errors; }
  const required = ["price", "changePct", "preClose", "open", "high", "low", "volume", "code", "name"] as const;
  for (const field of required) {
    if (quote[field] === undefined || quote[field] === null) {
      errors.push(`quote.${field} 缺失`);
    }
  }
  return errors;
}
