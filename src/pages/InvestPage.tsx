// i18n-exempt: 业务逻辑/API 描述/日志字符串，非 UI 展示文本
// SPDX-License-Identifier: AGPL-3.0-only

import { InvestHub } from "@/components/invest/InvestHub";
import { useAgentContext } from "@/hooks/useAgentContext";

/** 投资业务统一入口页面 — 薄包装，实际逻辑在 InvestHub */
export function InvestPage() {
  // ── Agent 上下文注入：告知 Agent 当前页面是投资总览页 ──
  useAgentContext({
    page: "invest",
    url: "/invest",
    quickActions: [
      { id: "add-holding", description: "添加股票持仓（代码/名称/股数/成本价）", requireConfirmation: true },
      { id: "remove-holding", description: "移除一笔股票持仓", requireConfirmation: true },
      { id: "list-watchlist", description: "列出当前自选股" },
      { id: "get-quote", description: "查询股票实时行情", params: { stock_code: "string" } },
    ],
  });

  return <InvestHub />;
}
