// SPDX-License-Identifier: AGPL-3.0-only

import { ComparePage } from "@/components/stock-analysis/ComparePage";

/**
 * 对比视图 — 工作区中栏的"对比"视图。
 *
 * 阶段 3：直接复用现有 ComparePage 组件。
 * 多股横向对比矩阵，当前股票自动作为基准。
 */
export function CompareView() {
  return <ComparePage />;
}
