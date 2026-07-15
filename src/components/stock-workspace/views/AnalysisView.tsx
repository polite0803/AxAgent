// SPDX-License-Identifier: AGPL-3.0-only

import { StockAnalysisPage } from "@/components/stock-analysis/StockAnalysisPage";

/**
 * 分析视图 — 工作区中栏的"分析"视图。
 *
 * 阶段 3：直接复用现有 StockAnalysisPage 组件，保持向后兼容。
 * 旧路由 /stock-analysis 仍可独立访问。
 * 阶段 4 会深入重构，将 4 阶段时间线作为主轴。
 */
export function AnalysisView() {
  return <StockAnalysisPage />;
}
