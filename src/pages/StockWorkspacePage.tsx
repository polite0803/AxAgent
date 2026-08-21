// i18n-exempt: 业务逻辑/API 描述/日志字符串，非 UI 展示文本
// SPDX-License-Identifier: AGPL-3.0-only

import { StockWorkspaceShell } from "@/components/stock-workspace/StockWorkspaceShell";
import { useAgentContext } from "@/hooks/useAgentContext";

/** 股票工作区页面入口 — 薄包装，实际逻辑在 StockWorkspaceShell */
export function StockWorkspacePage() {
  // ── Agent 上下文注入：告知 Agent 当前页面是股票工作区 ──
  useAgentContext({
    page: "stock-workspace",
    url: "/stock-workspace",
    quickActions: [
      { id: "list-watchlist", description: "列出当前自选股" },
      { id: "get-quote", description: "查询股票实时行情", params: { stock_code: "string" } },
      { id: "get-hot-stocks", description: "获取当日热门股票" },
      { id: "get-industry-ranking", description: "获取行业板块排名" },
      { id: "add-to-watchlist", description: "将股票加入自选股", requireConfirmation: true },
    ],
  });

  return <StockWorkspaceShell />;
}
