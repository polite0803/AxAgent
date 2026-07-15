// SPDX-License-Identifier: AGPL-3.0-only

import { TradePage } from "@/components/stock-analysis/TradePage";

/**
 * 交易视图 — 工作区中栏的"交易"视图。
 *
 * 阶段 3：直接复用现有 TradePage 组件。
 * 顶部 DecisionHeroBar 已显示决策摘要，交易视图聚焦下单 + 复查 + 统计。
 */
export function TradeView() {
  return <TradePage />;
}
