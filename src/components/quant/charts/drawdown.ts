// drawdown.ts — 回撤百分比计算（纯函数，便于单测）
//
// 回撤由权益曲线在「前端」本地推导：Rust 侧 EquityPoint 不含 drawdown 字段，
// 图表层用 running peak 推导每个时间点的回撤百分比。

import type { EquityPoint } from "@/types";

/**
 * 由权益曲线推导每个时间点的回撤百分比（%）。
 *
 * 回撤 = (当前权益 - 历史峰值) / 历史峰值，取负值，乘以 100。
 * 仅依赖 `equity` 字段（Rust EquityPoint 不含 drawdown）。
 * 历史峰值为 0 或负时回撤记为 0，避免除零或产生无意义的负值。
 *
 * 与 `crates/quant/src/metrics.rs::max_drawdown` 的口径一致（峰后用 peak 归一化），
 * 区别在本函数逐点输出序列（供面积图），而 Rust 侧只输出最大回撤标量。
 */
export function computeDrawdownPercent(curve: EquityPoint[]): number[] {
  let peak = Number.NEGATIVE_INFINITY;
  return curve.map((p) => {
    if (p.equity > peak) {
      peak = p.equity;
    }
    const dd = peak > 0 ? (p.equity - peak) / peak : 0;
    return parseFloat((dd * 100).toFixed(2));
  });
}
