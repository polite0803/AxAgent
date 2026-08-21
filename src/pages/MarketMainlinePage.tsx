// i18n-exempt: 业务逻辑/API 描述/日志字符串，非 UI 展示文本
// SPDX-License-Identifier: AGPL-3.0-only

import { MarketMainlineDashboard } from "@/components/stock-analysis/MarketMainlineDashboard";
import { useAgentContext } from "@/hooks/useAgentContext";

export function MarketMainlinePage() {
  // ── Agent 上下文注入：告知 Agent 当前页面是市场主线页 ──
  useAgentContext({
    page: "market-mainline",
    url: "/market-mainline",
    quickActions: [
      { id: "get-hot-stocks", description: "获取当日热门股票" },
      { id: "get-industry-ranking", description: "获取行业板块排名" },
      { id: "get-dragon-tiger", description: "获取龙虎榜数据" },
      { id: "get-index-quotes", description: "获取主要指数行情" },
      { id: "get-north-bound", description: "获取北向资金成交额" },
    ],
  });

  return <MarketMainlineDashboard />;
}
