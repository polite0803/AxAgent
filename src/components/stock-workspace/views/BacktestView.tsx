// SPDX-License-Identifier: AGPL-3.0-only

import { BacktestPage } from "@/components/stock-analysis/BacktestPage";

/**
 * 回测视图 — 工作区中栏的"回测"视图。
 *
 * 阶段 3：直接复用现有 BacktestPage 组件。
 * 合并了原 /backtest 路由和 /replay-workbench 的批量回放能力。
 */
export function BacktestView() {
  return <BacktestPage />;
}
