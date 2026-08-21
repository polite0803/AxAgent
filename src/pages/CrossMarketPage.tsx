// i18n-exempt: 业务逻辑/API 描述/日志字符串，非 UI 展示文本
// SPDX-License-Identifier: AGPL-3.0-only

import { CrossMarketDashboard } from "@/components/stock-analysis/CrossMarketDashboard";
import { useAgentContext } from "@/hooks/useAgentContext";

export function CrossMarketPage() {
  // ── Agent 上下文注入：告知 Agent 当前页面是跨市场页 ──
  useAgentContext({
    page: "cross-market",
    url: "/cross-market",
    quickActions: [
      { id: "get-quote", description: "查询股票实时行情（A/港股/美股）", params: { stock_code: "string" } },
      { id: "search-stock", description: "搜索股票", params: { keyword: "string" } },
      { id: "get-index-quotes", description: "获取主要指数行情" },
      { id: "search-news", description: "搜索财经新闻", params: { keyword: "string" } },
    ],
  });

  return <CrossMarketDashboard />;
}
