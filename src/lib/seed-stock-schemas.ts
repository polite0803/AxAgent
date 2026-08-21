// SPDX-License-Identifier: AGPL-3.0-only
// i18n-exempt: DynamicUI Schema 种子化脚本，schema 数据定义，非用户可见 UI 文案。

/**
 * 内置股票的 DynamicUI Schema 种子化脚本。
 *
 * 在应用启动时运行一次（fire-and-forget），将 src/lib/stock-schemas/ 中
 * 的 Schema 注册到 DynamicUI 数据库，使其出现在 /dynamic-ui 管理页中。
 *
 * 后续任何 Schema 的查看/编辑都在 /dynamic-ui 页完成——不需要新路由、新页面。
 */

import { evolutionDriftSchema, stockAnalysisPageSchema, stockQuoteSchema } from "@/lib/stock-schemas";
import { useDynamicUIStore } from "@/stores";

const BUILTIN_STOCK_SCHEMAS = [
  {
    title: "股票行情卡片",
    description: "股票当前价/涨跌幅/PE/PB/市值/换手率，30s 轮询",
    category: "dashboard",
    tags: ["stock", "builtin", "quote"],
    schema: stockQuoteSchema(),
  },
  {
    title: "策略权重进化面板",
    description: "策略权重表 + 权重时间线 Chart + 策略摘要",
    category: "dashboard",
    tags: ["stock", "builtin", "evolution"],
    schema: evolutionDriftSchema(),
  },
  {
    title: "股票分析完整页面",
    description: "搜索 / Tabs(行情/分析师/辩论/估值/风险/决策/反思/进化) / 侧栏",
    category: "dashboard",
    tags: ["stock", "builtin", "full-page"],
    schema: stockAnalysisPageSchema(),
  },
];

let _seeded = false;

/**
 * 种子化所有内置股票 Schema。在 main.tsx 中有 IP 后调用一次。
 * 已存在的 Schema 不会覆盖（保留用户编辑）。
 */
export async function seedStockSchemas(): Promise<void> {
  if (_seeded) { return; }
  _seeded = true;

  try {
    const store = useDynamicUIStore.getState();
    await store.fetchSchemas();

    const existingTitles = new Set(store.schemas.map((s) => s.title));

    for (const entry of BUILTIN_STOCK_SCHEMAS) {
      if (existingTitles.has(entry.title)) { continue; }

      await store.createSchema({
        title: entry.title,
        description: entry.description,
        category: entry.category,
        tags: entry.tags,
        schemaJson: JSON.stringify(entry.schema, null, 2),
      });
    }
  } catch (err) {
    // non-fatal: 若 IPC 尚未就绪或后端无 dynamic_ui 表，静默跳过
    console.debug("[seedStockSchemas] skipped (non-fatal):", err);
  }
}
