// SPDX-License-Identifier: AGPL-3.0-only

/**
 * AxInvest DynamicUI Schema 定义入口
 *
 * 所有 Schema 都是**纯数据**（UISchema JSON 结构），不包含任何 React 组件代码。
 * 它们可被 DynamicUIRenderer 直接渲染，无需任何业务组件包装。
 *
 * ## 使用方式
 *
 * ```tsx
 * import { DynamicUIRenderer } from "@/components/dynamicUI/DynamicUIRenderer";
 * import { stockQuoteSchema } from "@/lib/stock-schemas";
 *
 * // 从 store 获取数据
 * const quote = useStockAnalysisStore(s => s.quote);
 *
 * // 直接渲染——不需要任何中间组件
 * <DynamicUIRenderer schema={stockQuoteSchema()} dataContext={{ data: quote }} />
 * ```
 *
 * ## Schema vs 组件
 *
 * | 对比项 | 硬编码组件（旧方式） | DynamicUI Schema（新方式） |
 * |--------|-------------------|------------------------|
 * | 代码形式 | React 组件代码 | 纯 JSON 数据对象 |
 * | 数据源 | useStockAnalysisStore 订阅 | DataSourceConfig 声明 + dataContext |
 * | 渲染 | JSX 手写布局 | DynamicUIRenderer 递归渲染 |
 * | 可序列化 | ❌ 不可序列化 | ✅ JSON.stringify 可传递 |
 * | AI 可生成 | ❌ 不能 | ✅ 可通过 NL2UI / LLM 生成 |
 * | 替换成本 | 需重写整个组件 | 改 schema 对象即可 |
 * | 三方扩展 | ❌ 不可以 | ✅ 可注册为 Skill Panel |
 */

export { evolutionDriftSchema } from "./evolution-drift-schema";
export { stockAnalysisPageSchema } from "./stock-analysis-page-schema";
export { stockQuoteSchema, validateQuoteSchema } from "./stock-quote-schema";
