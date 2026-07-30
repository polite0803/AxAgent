// SPDX-License-Identifier: AGPL-3.0-only

import { StockAnalysisPage } from "@/components/stock-analysis/StockAnalysisPage";

/**
 * 分析视图 — 工作区中栏的"分析"视图。
 *
 * 传入 embeddedInWorkspace=true，让 StockAnalysisPage 跳过
 * 冗余的 sa-header（标题栏、交易跳转、PageTimeAnchor）和 InvestDashboard，
 * 因为外层 StockWorkspaceShell 已经渲染了这些。
 *
 * 旧路由 /stock-analysis 仍可独立访问（embeddedInWorkspace 默认为 false）。
 * 阶段 4 会深入重构，将 4 阶段时间线作为主轴。
 */
export function AnalysisView() {
  return <StockAnalysisPage embeddedInWorkspace />;
}
